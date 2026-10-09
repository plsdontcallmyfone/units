// Changed by Hookwars: tests that an event is marked posted only after a successful send (app audit A-10).
import { describe, expect, it } from 'vitest';
import type { BattleEvent } from '@hookwars/shared';
import { deliver } from './deliver.ts';

const ev = (sig: string, ordinal = 0) => ({ signature: sig, ordinal } as unknown as BattleEvent);

describe('bots delivery (app audit A-10)', () => {
  it('marks posted only after a successful send and saves after each', async () => {
    const posted = new Set<string>();
    const saves: number[] = [];
    const answers = [true, false, true];
    let i = 0;
    const r = await deliver([ev('a'), ev('b'), ev('c')], posted, () => 'text', async () => answers[i++]!, (p) => saves.push(p.size));
    expect(r).toEqual({ posted: 2, failed: 1, skipped: 0 });
    expect([...posted]).toEqual(['a:0', 'c:0']);
    expect(saves).toEqual([1, 2]);
  });
  it('a throwing send fails that event only; already posted events are not resent', async () => {
    const posted = new Set(['a:0']);
    let calls = 0;
    const r = await deliver([ev('a'), ev('b'), ev('c')], posted, () => 'text', async () => { calls++; if (calls === 1) throw new Error('down'); return true; }, () => undefined);
    expect(r).toEqual({ posted: 1, failed: 1, skipped: 0 });
    expect(posted.has('b:0')).toBe(false);
    expect(posted.has('c:0')).toBe(true);
  });
  it('events with nothing to say are marked seen', async () => {
    const posted = new Set<string>();
    const r = await deliver([ev('a')], posted, () => null, async () => true, () => undefined);
    expect(r.skipped).toBe(1);
    expect(posted.has('a:0')).toBe(true);
  });
});
