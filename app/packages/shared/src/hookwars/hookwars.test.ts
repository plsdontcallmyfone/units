// Changed by Hookwars: fixture with the ring's cumulative price.
import { describe, expect, it } from 'vitest';
import {
  bounty, counterStrikeDue, decodeRange, findBannedWords, forgeField, itemSentence, mergeSlotCuts, PARAMS, q64ToDecimal, raidWindowAdd,
  rollingRaidVolume, royaltyPosition, seasonScore, settleSplit, TEMPLATES, windowRead, warSpend,
} from '../index.ts';

describe('forge (04 2.7)', () => {
  it('moves toward the ceiling by the gain and never past it', () => {
    expect(forgeField('towardCeiling', 100, 300, 0, 1000, 5000)).toBe(300 + 350);
    expect(forgeField('towardCeiling', 1000, 900, 0, 1000, 5000)).toBe(1000);
  });
  it('moves toward the floor', () => {
    expect(forgeField('towardFloor', 600, 400, 100, 10000, 5000)).toBe(400 - 150);
  });
  it('keep needs equal values', () => {
    expect(forgeField('keep', 1, 1, 0, 1, 5000)).toBe(1);
    expect(forgeField('keep', 1, 2, 0, 2, 5000)).toBeNull();
    expect(forgeField('none', 1, 1, 0, 1, 5000)).toBeNull();
  });
  it('is symmetric', () => {
    for (const [a, b] of [[3, 9], [70, 12], [0, 0]] as const) {
      expect(forgeField('towardCeiling', a, b, 0, 100, 3333)).toBe(forgeField('towardCeiling', b, a, 0, 100, 3333));
      expect(forgeField('towardFloor', a, b, 0, 100, 3333)).toBe(forgeField('towardFloor', b, a, 0, 100, 3333));
    }
  });
});

describe('settle (04 2.5)', () => {
  it('splits royalty, bounty, rest exactly', () => {
    const s = settleSplit(1_000_000n, 500, 100);
    expect(s.royalty).toBe(50_000n);
    expect(s.bounty).toBe(9_500n);
    expect(s.royalty + s.bounty + s.rest).toBe(1_000_000n);
  });
  it('royalty position sums both sides', () => {
    expect(royaltyPosition(10_000n, 7_000n, 2_000n, 1000)).toEqual({ unsettled: 15_000n, settlesToRoyalty: 1_000n + 500n });
    expect(royaltyPosition(0n, 1n, 5n, 1000).unsettled).toBe(0n);
  });
});

describe('observations (03 3.1)', () => {
  const Q = 1n << 64n;
  const obs = {
    cumulative: 50n * Q, lastPriceQ64: 2n * Q, lastTs: 200n, index: 2, filled: 2,
    entries: [
      { ts: 100n, priceCumulative: 0n, quoteVolume: 10n, swapCount: 1n },
      { ts: 150n, priceCumulative: 50n * Q, quoteVolume: 30n, swapCount: 3n },
      { ts: 0n, priceCumulative: 0n, quoteVolume: 0n, swapCount: 0n },
    ],
  };
  it('reads a TWAP over the window', () => {
    const r = windowRead(obs, 40n, 5n, 200n, 100n, 60n)!;
    // cum_now = 50Q + 2Q*(200-200) = 50Q; from ts 100: 50Q / 100
    expect(r.twapQ64).toBe((50n * Q) / 100n);
    expect(r.quoteVolume).toBe(30n);
    expect(r.swaps).toBe(4n);
    expect(q64ToDecimal(r.twapQ64)).toBe('0.5');
  });
  it('is no signal for a short window or history', () => {
    expect(windowRead(obs, 0n, 0n, 200n, 30n, 60n)).toBeNull();
    expect(windowRead(obs, 0n, 0n, 200n, 150n, 60n)).toBeNull();
    expect(windowRead({ ...obs, filled: 0 }, 0n, 0n, 200n, 100n, 60n)).toBeNull();
  });
});

describe('raid windows (04 2.9, 05 2.5)', () => {
  it('rolls by whole windows and weighs the previous one by overlap', () => {
    let w = raidWindowAdd(null, 100n, 0n, 60n);
    w = raidWindowAdd(w, 50n, 30n, 60n);
    expect(w).toEqual({ windowStart: 0n, volume: 150n, prevVolume: 0n });
    w = raidWindowAdd(w, 10n, 70n, 60n);
    expect(w).toEqual({ windowStart: 60n, volume: 10n, prevVolume: 150n });
    expect(rollingRaidVolume(w, 90n, 60n)).toBe(10n + (150n * 30n) / 60n);
    expect(rollingRaidVolume(w, 300n, 60n)).toBe(0n);
  });
});

describe('war math (05)', () => {
  it('bounty caps by max, chest and points', () => {
    expect(bounty(10n, 1000n, null, 1_000_000n)).toEqual({ pay: 10_000n, spent: 10n });
    expect(bounty(10n, 1000n, 2500n, 1_000_000n)).toEqual({ pay: 2500n, spent: 3n });
    expect(bounty(10n, 1000n, null, 999n)).toEqual({ pay: 999n, spent: 1n });
    expect(bounty(0n, 1000n, null, 999n)).toEqual({ pay: 0n, spent: 0n });
  });
  it('spend is the smaller bound, bounty out of it', () => {
    expect(warSpend(1_000_000n, 1000, 5_000_000n, 100, 50)).toEqual({ spend: 50_000n, bounty: 250n, moved: 49_750n });
  });
  it('counter-strike trigger', () => {
    expect(counterStrikeDue(90n, 100n, 1000)).toBe(true);
    expect(counterStrikeDue(91n, 100n, 1000)).toBe(false);
  });
  it('season score with a besieged penalty', () => {
    const c = { raidVolumeWon: 10n, sieges: 2n, siegeSpend: 5n, timesBesieged: 3n, counterStrikes: 1n, treatySecs: 7n };
    const w = { raid: 1n, sieges: 10n, spend: 1n, besieged: 4n, counter: 2n, treaty: 1n };
    expect(seasonScore(c, w, false)).toBe(10n + 20n + 5n + 2n + 7n + 12n);
    expect(seasonScore(c, w, true)).toBe(10n + 20n + 5n + 2n + 7n - 12n);
    expect(() => seasonScore({ ...c, raidVolumeWon: 1n << 100n }, { ...w, raid: 1n << 100n }, false)).toThrow('MathOverflow');
  });
  it('slot cuts merge within bounds', () => {
    expect(mergeSlotCuts(1000n, [{ slot: 1, cut: 10n, maxCutBps: 100 }, { slot: 2, cut: 5n, maxCutBps: 50 }])).toEqual({ delivered: 985n });
    expect(mergeSlotCuts(1000n, [{ slot: 1, cut: 11n, maxCutBps: 100 }])).toEqual({ error: 'slot 1 cut above its bound' });
  });
});

describe('hook data ranges (00 4.4, 04 2.4)', () => {
  const data = new Uint8Array(64);
  data.set([3, 0x01, 7, 0, 0, 0, 25, 0, 0, 0, 2, 0], 32);
  const spec = { slot: 1, kind: 4, offset: 32, len: 12, dataEpoch: 3, templateId: 1 };
  it('decodes a Raid range when the epoch matches', () => {
    expect(decodeRange(data, spec, 7)).toEqual({ slot: 1, type: 'raid', seasonId: 7, raidPoints: 25, tickets: 2 });
  });
  it('reads points 0 in another season, tickets carry over', () => {
    expect(decodeRange(data, spec, 8)).toMatchObject({ raidPoints: 0, tickets: 2 });
  });
  it('a stale epoch reads empty', () => {
    expect(decodeRange(data, { ...spec, dataEpoch: 4 }, 7)).toEqual({ slot: 1, type: 'stale' });
  });
});

describe('templates and words', () => {
  it('templates 1 to 45, War orders uses 11 fields', () => {
    // Changed by Hookwars: the arsenal (08), the soulbound badge (09) and the expansion templates (10).
    expect(TEMPLATES.map((t) => t.id)).toEqual(Array.from({ length: 45 }, (_, i) => i + 1));
    expect(TEMPLATES.find((t) => t.id === 9)!.fields).toHaveLength(11);
  });
  it('every arsenal template has its own sentence and field indices in order', () => {
    for (const t of TEMPLATES) {
      expect(itemSentence(t.id, t.fields.map(() => 1), { target: 'RIVAL' })).not.toBe(t.name);
      expect(t.fields.map((f) => f.index)).toEqual(t.fields.map((_, i) => i));
    }
  });
  it('every sentence is clean of banned words and em dashes', () => {
    for (const t of TEMPLATES) {
      const s = itemSentence(t.id, t.fields.map(() => 1), { target: 'RIVAL' });
      expect(findBannedWords(s)).toEqual([]);
    }
    expect(itemSentence(1, [500, 100, 3], { target: 'RIVAL' })).toBe('Buyers who sold RIVAL pay 5% less creator and holder fee, pay a 1% toll, and earn 3 raid points per unit of SOL raided.');
  });
  it('finds banned words', () => {
    expect(findBannedWords('high APY here')).toEqual(['apy']);
    expect(findBannedWords('a taxi')).toEqual([]);
  });
  it('no parameter carries an invented value', () => {
    for (const p of PARAMS) if (p.value !== null) expect(p.source).toBeTruthy();
  });
});
