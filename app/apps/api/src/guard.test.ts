import { describe, expect, it } from 'vitest';
import { Readable } from 'node:stream';
import type { IncomingMessage } from 'node:http';
import { clusterName, HttpError, intParam, RateLimiter, readJsonBody, redactUrl } from './guard.ts';

const req = (body: string, headers: Record<string, string> = {}) =>
  Object.assign(Readable.from([Buffer.from(body)]), { headers, destroy: () => undefined }) as unknown as IncomingMessage;

describe('API guards (app audit 1)', () => {
  it('A-2: names a cluster, never the URL, and redacts keys from logs', () => {
    expect(clusterName('https://mainnet.helius-rpc.com/?api-key=SECRET')).toBe('mainnet-beta');
    expect(clusterName('https://api.devnet.solana.com')).toBe('devnet');
    expect(clusterName('http://127.0.0.1:8899')).toBe('localnet');
    expect(clusterName('https://rpc.example.org/KEY')).toBe('custom');
    expect(redactUrl('https://u:p@mainnet.helius-rpc.com/?api-key=SECRET')).toBe('https://mainnet.helius-rpc.com/');
    expect(redactUrl('https://mainnet.helius-rpc.com/?api-key=SECRET')).not.toContain('SECRET');
  });
  it('A-3: refuses bodies over the cap, declared or read', async () => {
    await expect(readJsonBody(req('{}', { 'content-length': '999999' }))).rejects.toMatchObject({ status: 413 });
    await expect(readJsonBody(req('x'.repeat(70_000)))).rejects.toMatchObject({ status: 413 });
    await expect(readJsonBody(req('[1]'))).rejects.toMatchObject({ status: 400 });
    expect(await readJsonBody(req('{"a":1}'))).toEqual({ a: 1 });
  });
  it('A-4: a client runs out of tokens and refills over time', () => {
    let t = 0;
    const r = new RateLimiter(3, 1, () => t);
    expect([r.take('a'), r.take('a'), r.take('a'), r.take('a')]).toEqual([true, true, true, false]);
    expect(r.take('b')).toBe(true);
    t = 1_000;
    expect(r.take('a')).toBe(true);
    expect(r.take('a')).toBe(false);
  });
  it('A-12: path numbers are integers in range or a 400', () => {
    expect(intParam('12', 'season', 0, 4_294_967_295)).toBe(12);
    for (const bad of ['-1', '1e3', 'x', '99999999999', '']) expect(() => intParam(bad, 'season', 0, 4_294_967_295)).toThrow(HttpError);
  });
});
