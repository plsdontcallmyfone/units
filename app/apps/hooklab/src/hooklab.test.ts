import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { gzipSync } from 'node:zlib';
import { chmodSync, existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { AddressInfo } from 'node:net';
import type { Server } from 'node:http';
import { cleanPath, crateRoot, readTarball, TarError, type TarLimits } from './tar.ts';
import { Lab, type LabConfig } from './lab.ts';
import { createLabServer, RateLimiter } from './server.ts';
import { base58, publicKeyOf } from './key.ts';

const LIMITS: TarLimits = { maxCompressed: 1 << 20, maxUnpacked: 1 << 20, maxFiles: 50, maxPath: 200 };

/** A ustar archive of `files`; `type` per file defaults to a regular file. */
function tar(files: Array<{ path: string; data?: string; type?: string; link?: string }>): Buffer {
  const blocks: Buffer[] = [];
  for (const f of files) {
    const data = Buffer.from(f.data ?? '');
    const h = Buffer.alloc(512);
    h.write(f.path, 0, 100, 'utf8');
    h.write('0000644\0', 100);
    h.write('0000000\0', 108);
    h.write('0000000\0', 116);
    h.write(data.length.toString(8).padStart(11, '0') + '\0', 124);
    h.write('00000000000\0', 136);
    h.write(f.type ?? '0', 156);
    if (f.link) h.write(f.link, 157, 100);
    h.write('ustar\0' + '00', 257);
    h.fill(0x20, 148, 156);
    let sum = 0;
    for (const b of h) sum += b;
    h.write(sum.toString(8).padStart(6, '0') + '\0 ', 148);
    blocks.push(h, data, Buffer.alloc((512 - (data.length % 512)) % 512));
  }
  blocks.push(Buffer.alloc(1024));
  return gzipSync(Buffer.concat(blocks));
}

const crate = (prefix = 'tpl/') => [
  { path: `${prefix}Cargo.toml`, data: '[package]\nname = "x"\n' },
  { path: `${prefix}Cargo.lock`, data: '' },
  { path: `${prefix}hooklab.json`, data: '{}' },
  { path: `${prefix}src/lib.rs`, data: '// x' },
];

describe('tarball reader', () => {
  it('reads a crate under one top directory', () => {
    const e = readTarball(tar(crate()), LIMITS);
    expect(e.map((x) => x.path)).toContain('tpl/src/lib.rs');
    expect(crateRoot(e)).toBe('tpl');
    expect(crateRoot(readTarball(tar(crate('')), LIMITS))).toBe('');
  });

  it('refuses paths that leave the root', () => {
    for (const p of ['../x', '/etc/passwd', 'a/../../x', 'a\\b', 'C:/x', 'a//b']) {
      expect(() => cleanPath(p, LIMITS), p).toThrow(TarError);
    }
    expect(cleanPath('./a/b', LIMITS)).toBe('a/b');
    expect(cleanPath('a/._b', LIMITS)).toBeNull();
  });

  it('refuses links, devices and fifos', () => {
    for (const type of ['1', '2', '3', '4', '6']) {
      expect(() => readTarball(tar([...crate(), { path: 'tpl/l', type, link: '/etc/passwd' }]), LIMITS), type).toThrow(/refused entry type/);
    }
  });

  it('enforces the limits and the required files', () => {
    expect(() => readTarball(tar(crate()), { ...LIMITS, maxFiles: 2 })).toThrow(/too many files/);
    expect(() => readTarball(tar([...crate(), { path: 'tpl/big', data: 'x'.repeat(4096) }]), { ...LIMITS, maxUnpacked: 1024 })).toThrow(TarError);
    expect(() => readTarball(Buffer.from('not gzip'), LIMITS)).toThrow(/gzip/);
    expect(() => readTarball(tar([...crate(), { path: 'tpl/src/lib.rs', data: 'again' }]), LIMITS)).toThrow(/duplicate/);
    expect(() => crateRoot(readTarball(tar(crate().slice(1)), LIMITS))).toThrow(/Cargo.toml/);
  });

  it('honors a pax path header and still checks it', () => {
    const rec = (k: string, v: string): string => {
      const body = ` ${k}=${v}\n`;
      let n = body.length + 1;
      while (String(n).length + body.length !== n) n = String(n).length + body.length;
      return `${n}${body}`;
    };
    const ok = tar([...crate(), { path: 'PaxHeader', type: 'x', data: rec('path', 'tpl/long/name.rs') }, { path: 'short', data: 'y' }]);
    expect(readTarball(ok, LIMITS).map((e) => e.path)).toContain('tpl/long/name.rs');
    const bad = tar([...crate(), { path: 'PaxHeader', type: 'x', data: rec('path', '../escape') }, { path: 'short', data: 'y' }]);
    expect(() => readTarball(bad, LIMITS)).toThrow(/unsafe path/);
  });
});

describe('keys', () => {
  it('encodes base58 like Solana', () => {
    expect(base58(new Uint8Array(32))).toBe('11111111111111111111111111111111');
    expect(publicKeyOf([...new Array(32).fill(7), ...new Array(32).fill(0)])).toBe('11111111111111111111111111111111');
    expect(() => publicKeyOf([1, 2])).toThrow();
  });
});

describe('service', () => {
  let dir: string;
  let server: Server;
  let lab: Lab;
  let base: string;

  const start = (over: Partial<LabConfig> = {}, submitCapacity = 100): Promise<void> => {
    const fake = join(dir, 'fake-hooklab.mjs');
    // Stands in for the CLI: writes a signed-report shape; fails the run when the crate says so.
    writeFileSync(fake, `#!/usr/bin/env node
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
const a = process.argv.slice(2);
const crate = a[1];
const at = (f) => a[a.indexOf(f) + 1];
const verdict = existsSync(crate + '/FAIL') ? 'fail' : 'pass';
if (existsSync(crate + '/HANG')) await new Promise((r) => setTimeout(r, 10000));
writeFileSync(at('--report'), JSON.stringify({ domain: 'units:hooklab:report:v1', body: { verdict, submission: at('--submission'), manifest: JSON.parse(readFileSync(crate + '/hooklab.json', 'utf8')) }, signer: 'S', signature: 'X' }));
process.exit(verdict === 'pass' ? 0 : 1);
`);
    chmodSync(fake, 0o755);
    lab = new Lab({
      dataDir: join(dir, 'data'), bin: fake, keyPath: join(dir, 'key.json'), sandbox: [], unsandboxed: true,
      timeoutMs: 5000, checkArgs: [], tar: LIMITS, workers: 1, ...over,
    });
    server = createLabServer({ lab, signer: 'S', submitLimit: new RateLimiter(submitCapacity, 0.0001), readLimit: new RateLimiter(100, 10), trustProxy: false, maxQueued: 10 });
    return new Promise((r) => server.listen(0, '127.0.0.1', () => { base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`; r(); }));
  };

  const post = (body: Buffer | string, type: string): Promise<Response> =>
    fetch(`${base}/v1/submissions`, { method: 'POST', headers: { 'content-type': type }, body });

  beforeEach(() => { dir = mkdtempSync(join(tmpdir(), 'hooklab-svc-')); });
  afterEach(async () => { await new Promise((r) => server.close(r)); rmSync(dir, { recursive: true, force: true }); });

  it('queues a tarball, runs the lab and serves the report; the same bytes are one submission', async () => {
    await start();
    const gz = tar(crate());
    const r = await post(gz, 'application/gzip');
    expect(r.status).toBe(202);
    const s = (await r.json()) as { id: string; state: string };
    expect(s.id).toMatch(/^[0-9a-f]{64}$/);
    await lab.drained();
    const st = (await (await fetch(`${base}/v1/submissions/${s.id}`)).json()) as { state: string; verdict: string };
    expect(st).toMatchObject({ state: 'done', verdict: 'pass' });
    const rep = (await (await fetch(`${base}/v1/submissions/${s.id}/report`)).json()) as { body: { submission: string } };
    expect(rep.body.submission).toBe(s.id);
    const again = (await (await post(gz, 'application/gzip')).json()) as { id: string; state: string };
    expect(again).toMatchObject({ id: s.id, state: 'done' });
  });

  it('records a failing verdict as done, not as an error', async () => {
    await start();
    const r = (await (await post(tar([...crate(), { path: 'tpl/FAIL', data: '' }]), 'application/gzip')).json()) as { id: string };
    await lab.drained();
    expect(lab.status(r.id)).toMatchObject({ state: 'done', verdict: 'fail' });
  });

  it('kills a run at the time limit', async () => {
    await start({ timeoutMs: 300 });
    const r = (await (await post(tar([...crate(), { path: 'tpl/HANG', data: '' }]), 'application/gzip')).json()) as { id: string };
    await lab.drained();
    expect(lab.status(r.id)).toMatchObject({ state: 'error' });
    expect(lab.status(r.id)!.error).toMatch(/limit/);
  });

  it('refuses to run without a sandbox unless told it is unsandboxed', async () => {
    await start({ unsandboxed: false, sandbox: [] });
    const r = (await (await post(tar(crate()), 'application/gzip')).json()) as { id: string };
    await lab.drained();
    expect(lab.status(r.id)!.error).toMatch(/sandbox/);
  });

  it('runs through the sandbox prefix', async () => {
    await start({ unsandboxed: false, sandbox: ['env', 'HOOKLAB_SANDBOXED=1'] });
    const r = (await (await post(tar(crate()), 'application/gzip')).json()) as { id: string };
    await lab.drained();
    expect(lab.status(r.id)).toMatchObject({ state: 'done', verdict: 'pass' });
  });

  it('rejects bad archives, bad git inputs and unknown types before queueing', async () => {
    await start();
    expect((await post(tar([...crate(), { path: '../x', data: '' }]), 'application/gzip')).status).toBe(400);
    expect((await post(JSON.stringify({ git: 'https://gitlab.com/a/b', commit: 'a'.repeat(40) }), 'application/json')).status).toBe(400);
    expect((await post(JSON.stringify({ git: 'https://github.com/a/b', commit: 'main' }), 'application/json')).status).toBe(400);
    expect((await post(JSON.stringify({ git: 'https://github.com/a/b;rm -rf', commit: 'a'.repeat(40) }), 'application/json')).status).toBe(400);
    expect((await post('x', 'text/plain')).status).toBe(415);
    expect((await fetch(`${base}/v1/submissions/${'0'.repeat(64)}`)).status).toBe(404);
    expect((await fetch(`${base}/v1/submissions/../etc`)).status).toBe(404);
    expect(lab.queued()).toBe(0);
  });

  it('accepts a well-formed git submission (the fetch itself is not tested here)', async () => {
    await start({ bin: '/bin/false' });
    const r = await post(JSON.stringify({ git: 'https://github.com/plsdontcallmyfone/does-not-exist', commit: 'a'.repeat(40) }), 'application/json');
    expect(r.status).toBe(202);
    await lab.drained();
  });

  it('rate limits submissions per client', async () => {
    await start({}, 2);
    expect((await post(tar(crate()), 'application/gzip')).status).toBe(202);
    expect((await post(tar(crate('a/')), 'application/gzip')).status).toBe(202);
    expect((await post(tar(crate('b/')), 'application/gzip')).status).toBe(429);
    await lab.drained();
  });

  it('stores nothing about the submitter', async () => {
    await start();
    const r = (await (await post(tar(crate()), 'application/gzip')).json()) as { id: string };
    await lab.drained();
    const files = readdirSync(join(dir, 'data', r.id));
    expect(files.sort()).toEqual(['report.json', 'status.json', 'submission.tgz']);
    expect(JSON.stringify(lab.status(r.id))).not.toMatch(/127\.0\.0\.1/);
    expect(existsSync(join(dir, 'data', r.id, 'status.json.tmp'))).toBe(false);
  });

  it('describes itself', async () => {
    await start();
    const l = (await (await fetch(`${base}/v1/lab`)).json()) as { domain: string };
    expect(l.domain).toBe('units:hooklab:report:v1');
    expect((await fetch(`${base}/healthz`)).status).toBe(200);
  });
});
