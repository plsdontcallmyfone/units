// Changed by Hookwars: tests for the API guards (app audit A-2, A-3, A-4, A-12).
import { describe, expect, it } from 'vitest';
import { Readable } from 'node:stream';
import type { IncomingMessage } from 'node:http';
import { clientKey, clusterName, HttpError, intParam, RateLimiter, readJsonBody, redactUrl } from './guard.ts';

const req = (body: string, headers: Record<string, string> = {}) =>
  Object.assign(Readable.from([Buffer.from(body)]), { headers }) as unknown as IncomingMessage;

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

// Fuzz audit 1, finding 4: `siege` succeeds when it only waits, so the feed keys on the event.
import { KIND_OF } from './reads.ts';
describe('battle feed kinds', () => {
  it('maps SiegeWaited apart from SiegeExecuted', () => {
    expect(KIND_OF.SiegeExecuted).toBe('siege');
    expect(KIND_OF.SiegeWaited).toBe('siege_waited');
  });
});

describe('client key (review 3 L-8)', () => {
  const req = (fwd: string | undefined, remote = '10.0.0.2') => ({ headers: fwd === undefined ? {} : { 'x-forwarded-for': fwd }, socket: { remoteAddress: remote } }) as unknown as IncomingMessage;
  it('ignores a client-set x-forwarded-for unless a proxy is trusted', () => {
    expect(clientKey(req('1.2.3.4'))).toBe('10.0.0.2');
    expect(clientKey(req(undefined))).toBe('10.0.0.2');
  });
  it('behind trusted proxies takes the entry the farthest one appended, never a spoofed left entry', () => {
    expect(clientKey(req('6.6.6.6, 203.0.113.9'), 1)).toBe('203.0.113.9');
    expect(clientKey(req('6.6.6.6, 203.0.113.9, 10.0.0.5'), 2)).toBe('203.0.113.9');
    expect(clientKey(req('203.0.113.9'), 2)).toBe('10.0.0.2');
  });
});
