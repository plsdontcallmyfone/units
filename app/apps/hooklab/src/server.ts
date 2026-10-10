/**
 * The Hook Lab service on node:http.
 *
 *   POST /v1/submissions            body: a gzip tarball of the crate (application/gzip), or JSON
 *                                   { "git": "https://github.com/<owner>/<repo>", "commit": "<40 hex>" }
 *   GET  /v1/submissions/:id        status
 *   GET  /v1/submissions/:id/report the signed report (tools/hooklab: `hooklab verify` checks it)
 *   GET  /v1/lab                    the lab's signer, report domain and limits
 *   GET  /healthz
 */
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { Lab, SubmissionError } from './lab.ts';
import { TarError } from './tar.ts';

export const REPORT_DOMAIN = 'units:hooklab:report:v1';

/** Token bucket per client address, in memory only (never written anywhere). */
export class RateLimiter {
  private buckets = new Map<string, { tokens: number; at: number }>();
  readonly capacity: number;
  readonly perSec: number;
  constructor(capacity: number, perSec: number) {
    this.capacity = capacity;
    this.perSec = perSec;
  }
  take(key: string, now = Date.now()): boolean {
    const b = this.buckets.get(key) ?? { tokens: this.capacity, at: now };
    b.tokens = Math.min(this.capacity, b.tokens + ((now - b.at) / 1000) * this.perSec);
    b.at = now;
    const ok = b.tokens >= 1;
    if (ok) b.tokens -= 1;
    this.buckets.set(key, b);
    if (this.buckets.size > 10_000) this.buckets.delete(this.buckets.keys().next().value!);
    return ok;
  }
}

export interface ServerOptions {
  lab: Lab;
  signer: string;
  submitLimit: RateLimiter;
  readLimit: RateLimiter;
  /** Read the client address from X-Forwarded-For (behind a proxy the operator controls). */
  trustProxy: boolean;
  /** Most queued submissions before new ones are turned away. */
  maxQueued: number;
}

const json = (res: ServerResponse, status: number, body: unknown): void => {
  res.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store' });
  res.end(JSON.stringify(body));
};

function readBody(req: IncomingMessage, max: number): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    let n = 0;
    req.on('data', (c: Buffer) => {
      n += c.length;
      if (n > max) { reject(new TarError('body too large')); req.destroy(); return; }
      chunks.push(c);
    });
    req.on('end', () => resolve(Buffer.concat(chunks)));
    req.on('error', reject);
  });
}

export function createLabServer(o: ServerOptions): Server {
  const client = (req: IncomingMessage): string => {
    const fwd = o.trustProxy ? String(req.headers['x-forwarded-for'] ?? '').split(',')[0]!.trim() : '';
    return fwd || req.socket.remoteAddress || 'unknown';
  };
  return createServer((req, res) => {
    void (async () => {
      const url = new URL(req.url ?? '/', 'http://lab');
      const path = url.pathname;
      try {
        if (req.method === 'GET' && path === '/healthz') return json(res, 200, { ok: true, queued: o.lab.queued() });
        if (req.method === 'GET' && path === '/v1/lab') {
          return json(res, 200, { signer: o.signer, domain: REPORT_DOMAIN, limits: o.lab.cfg.tar, timeoutMs: o.lab.cfg.timeoutMs });
        }
        if (req.method === 'POST' && path === '/v1/submissions') {
          if (!o.submitLimit.take(client(req))) return json(res, 429, { error: 'too many submissions, try later' });
          if (o.lab.queued() >= o.maxQueued) return json(res, 503, { error: 'the queue is full, try later' });
          const type = String(req.headers['content-type'] ?? '').split(';')[0]!.trim();
          if (type === 'application/json') {
            const body = JSON.parse((await readBody(req, 4096)).toString('utf8')) as { git?: unknown; commit?: unknown };
            return json(res, 202, o.lab.submitGit(body.git, body.commit));
          }
          if (type === 'application/gzip' || type === 'application/x-gzip' || type === 'application/octet-stream') {
            const gz = await readBody(req, o.lab.cfg.tar.maxCompressed);
            return json(res, 202, o.lab.submitTarball(gz));
          }
          return json(res, 415, { error: 'send application/gzip (a tarball) or application/json ({ git, commit })' });
        }
        const m = /^\/v1\/submissions\/([0-9a-f]{64})(\/report)?$/.exec(path);
        if (req.method === 'GET' && m) {
          if (!o.readLimit.take(client(req))) return json(res, 429, { error: 'too many requests' });
          if (m[2]) {
            const r = o.lab.report(m[1]!);
            if (!r) return json(res, 404, { error: 'no report yet' });
            res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' });
            return res.end(r);
          }
          const s = o.lab.status(m[1]!);
          return s ? json(res, 200, s) : json(res, 404, { error: 'unknown submission' });
        }
        return json(res, 404, { error: 'not found' });
      } catch (e) {
        if (e instanceof TarError || e instanceof SubmissionError || e instanceof SyntaxError) {
          return json(res, 400, { error: e.message });
        }
        return json(res, 500, { error: 'internal error' });
      }
    })();
  });
}
