import { describe, expect, it } from 'vitest';
import { BodyTooLarge, forwardPath, readCapped } from './proxy.ts';

describe('the /api proxy (audit A-3, A-11)', () => {
  it('forwards only v1 paths without dot segments', () => {
    expect(forwardPath(['v1', 'status'])).toBe('v1/status');
    expect(forwardPath(['v1', 'tokens', 'a b'])).toBe('v1/tokens/a%20b');
    expect(forwardPath(['v2', 'x'])).toBeNull();
    expect(forwardPath(['..', 'x'])).toBeNull();
    expect(forwardPath(['v1', '..'])).toBeNull();
    expect(forwardPath(['v1', '.'])).toBeNull();
    expect(forwardPath(['v1'])).toBeNull();
    expect(forwardPath(['v1', 'a/b'])).toBeNull();
  });
  it('caps request bodies by declared length and by bytes read', async () => {
    const big = new Request('http://x/api/v1/votes/prepare', { method: 'POST', body: 'x'.repeat(70_000) });
    await expect(readCapped(big)).rejects.toBeInstanceOf(BodyTooLarge);
    const declared = new Request('http://x/', { method: 'POST', body: 'small', headers: { 'content-length': '999999' } });
    await expect(readCapped(declared)).rejects.toBeInstanceOf(BodyTooLarge);
    const ok = new Request('http://x/', { method: 'POST', body: '{"a":1}' });
    expect(await readCapped(ok)).toBe('{"a":1}');
    expect(await readCapped(new Request('http://x/', { method: 'GET' }))).toBeUndefined();
  });
});
