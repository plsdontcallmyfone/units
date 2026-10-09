/**
 * Request guards of the API (app audit 1): the cluster a status may name (never the RPC URL, A-2),
 * URL redaction for logs (A-2), a body cap (A-3), a per-client token bucket (A-4) and integer
 * parsing for path and query numbers (A-12). Pure where possible, so each is unit tested.
 */
import type { IncomingMessage } from 'node:http';

/** The cluster an RPC URL points at, by host only: never the URL, which may carry an API key. */
export function clusterName(rpcUrl: string): 'devnet' | 'testnet' | 'mainnet-beta' | 'localnet' | 'custom' {
  let host = '';
  try { host = new URL(rpcUrl).hostname.toLowerCase(); } catch { return 'custom'; }
  if (host === '127.0.0.1' || host === 'localhost' || host === '::1') return 'localnet';
  if (host.includes('devnet')) return 'devnet';
  if (host.includes('testnet')) return 'testnet';
  if (host.includes('mainnet')) return 'mainnet-beta';
  return 'custom';
}

/** An RPC URL with its query string, user and password removed, for logs. */
export function redactUrl(u: string): string {
  try {
    const x = new URL(u);
    x.search = ''; x.username = ''; x.password = '';
    return x.toString();
  } catch { return '[unparsable url]'; }
}

/** The largest JSON body the API reads. */
export const MAX_BODY_BYTES = 64 * 1024;

export class HttpError extends Error {
  readonly status: number; readonly code: string;
  constructor(status: number, code: string, message: string) { super(message); this.status = status; this.code = code; }
}

/** Reads a JSON object body, refusing a declared or actual size above `max` with 413. */
export async function readJsonBody(req: IncomingMessage, max = MAX_BODY_BYTES): Promise<Record<string, unknown>> {
  const declared = Number(req.headers['content-length'] ?? '0');
  if (Number.isFinite(declared) && declared > max) throw new HttpError(413, 'TooLarge', 'The request body is too large.');
  const chunks: Buffer[] = [];
  let total = 0;
  for await (const c of req) {
    const b = c as Buffer;
    total += b.byteLength;
    if (total > max) { req.destroy(); throw new HttpError(413, 'TooLarge', 'The request body is too large.'); }
    chunks.push(b);
  }
  if (!chunks.length) return {};
  let v: unknown;
  try { v = JSON.parse(Buffer.concat(chunks).toString('utf8')); } catch { throw new HttpError(400, 'BadRequest', 'The body is not JSON.'); }
  if (typeof v !== 'object' || v === null || Array.isArray(v)) throw new HttpError(400, 'BadRequest', 'The body must be a JSON object.');
  return v as Record<string, unknown>;
}

/** A token bucket per client key: `capacity` requests, refilled at `perSecond`. Operator settings
 * (RATE_PREPARE_CAPACITY and friends), not protocol parameters. */
export class RateLimiter {
  private readonly buckets = new Map<string, { tokens: number; at: number }>();
  constructor(readonly capacity: number, readonly perSecond: number, private readonly now: () => number = Date.now) {}
  take(key: string): boolean {
    const t = this.now();
    const b = this.buckets.get(key) ?? { tokens: this.capacity, at: t };
    b.tokens = Math.min(this.capacity, b.tokens + ((t - b.at) / 1000) * this.perSecond);
    b.at = t;
    if (b.tokens < 1) { this.buckets.set(key, b); return false; }
    b.tokens -= 1;
    this.buckets.set(key, b);
    if (this.buckets.size > 50_000) this.prune(t);
    return true;
  }
  private prune(t: number): void {
    for (const [k, b] of this.buckets) if ((t - b.at) / 1000 * this.perSecond >= this.capacity) this.buckets.delete(k);
  }
}

/** The client a request comes from: the first `x-forwarded-for` entry the site sets, else the socket. */
export function clientKey(req: IncomingMessage): string {
  const f = req.headers['x-forwarded-for'];
  const first = (Array.isArray(f) ? f[0] : f)?.split(',')[0]?.trim();
  return first || req.socket.remoteAddress || 'unknown';
}

/** An integer in [lo, hi] from a path or query string, else a 400. */
export function intParam(v: string | null | undefined, name: string, lo: number, hi: number): number {
  if (v === null || v === undefined || !/^\d+$/.test(v)) throw new HttpError(400, 'BadRequest', `"${name}" must be an integer.`);
  const n = Number(v);
  if (!Number.isSafeInteger(n) || n < lo || n > hi) throw new HttpError(400, 'BadRequest', `"${name}" must be from ${lo} to ${hi}.`);
  return n;
}
