import { describe, expect, it } from 'vitest';
import { parseArgs } from './cli.ts';
import { HEALTH_PORTS, pickPort, startHealth } from './health.ts';

describe('health endpoint', () => {
  it('binds the first free port in 9970 to 9979 on 127.0.0.1 and reports agents without secrets', async () => {
    const port = await pickPort(null);
    expect(HEALTH_PORTS).toContain(port);
    const server = await startHealth(port, () => [{ name: 'scout', passport: 'P', lastTickAt: 1, directiveSeq: 0, halted: null, lastError: null, ticks: 1, sends: false, provider: 'stub', model: 'stub-1' }]);
    try {
      const res = await fetch(`http://127.0.0.1:${port}/healthz`);
      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ ok: true, agents: [expect.objectContaining({ name: 'scout', directiveSeq: 0, sends: false })] });
      expect((await fetch(`http://127.0.0.1:${port}/other`)).status).toBe(404);
      await expect(pickPort(port)).rejects.toThrow(/in use/);
    } finally { server.close(); }
    await expect(pickPort(8080)).rejects.toThrow(/9970/);
  });
});

describe('cli arguments', () => {
  it('reads flags with and without values', () => {
    const a = parseArgs(['run', 'a.json', 'b.json', '--once', '--send', '--seq', '3', '--rules-uri=https://x']);
    expect(a.cmd).toBe('run');
    expect(a.pos).toEqual(['a.json', 'b.json']);
    expect(a.flags.get('once')).toBe(true);
    expect(a.flags.get('send')).toBe(true);
    expect(a.flags.get('seq')).toBe('3');
    expect(a.flags.get('rules-uri')).toBe('https://x');
  });
});
