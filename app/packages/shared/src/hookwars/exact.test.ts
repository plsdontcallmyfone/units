/**
 * Every case of `vectors/hookwars-math.json` through `exact.ts`, with exact equality. The file is
 * rendered by `programs/tests/tests/app_vectors.rs` from the Rust the programs run, and that test
 * rewrites it and fails when the Rust changes, so this suite always reads what the programs compute.
 */
import { describe, expect, it } from 'vitest';
import raw from '../../vectors/hookwars-math.json' with { type: 'json' };
import { bpsOf, combine, lootDraw, performanceHolds, PARAM_FIELDS, seasonScoreExact, shape, windowReadRaw, type ExactWindowRead } from './exact.ts';
import { TEMPLATES } from './templates.ts';

const V = raw as unknown as {
  paramFields: number;
  shapes: { id: number; kind: number; fieldCount: number; forgeable: boolean; rules: string[]; zeroOff: boolean[] }[];
  combine: { id: number; gainBps: number; min: number[]; max: number[]; a: number[]; b: number[]; out: number[] | null; error: string | null }[];
  windowRead: { data: string; poolQuoteVolume: string; poolSwapCount: string; now: string; window: string; baseWindow: string; read: Record<string, string> | null; baseRead: Record<string, string> | null; rule: { metric: number; op: number; ratioBps: number }; holds: boolean }[];
  seasonScore: { weights: string[]; counters: Record<string, string | number>; penalizeBesieged: boolean; score: string | null }[];
  lootDraw: { entries: { templateId: number; weight: number; ranges: [number, number][] }[]; value: string; templateId: number | null; params: number[] | null }[];
  bpsOf: { amount: string; part: number; out: string }[];
};

const hex = (h: string): Uint8Array => Uint8Array.from(h.match(/../g) ?? [], (b) => parseInt(b, 16));
const read = (r: Record<string, string> | null): ExactWindowRead | null =>
  r ? { twapQ64: BigInt(r.twapQ64!), quoteVolume: BigInt(r.quoteVolume!), swaps: BigInt(r.swaps!), seconds: BigInt(r.seconds!) } : null;

describe('hookwars-math vectors', () => {
  it('PARAM_FIELDS matches the Rust', () => expect(PARAM_FIELDS).toBe(V.paramFields));

  it('template shapes match, and the site table agrees on forgeable fields', () => {
    for (const v of V.shapes) {
      expect(shape(v.id)).toEqual({ id: v.id, kind: v.kind, fieldCount: v.fieldCount, forgeable: v.forgeable, rules: v.rules, zeroOff: v.zeroOff });
      const t = TEMPLATES.find((x) => x.id === v.id)!;
      expect(t.fields.length).toBe(v.fieldCount);
      expect(t.forgeable).toBe(v.forgeable);
      if (v.forgeable) t.fields.forEach((f, i) => expect(f.forge).toBe(v.rules[i]));
    }
  });

  it(`combine: ${V.combine.length} cases`, () => {
    for (const v of V.combine) {
      const r = combine(v.id, v.min, v.max, v.gainBps, v.a, v.b);
      if (v.error) expect(r).toEqual({ error: v.error });
      else expect(r).toEqual({ out: v.out });
    }
  });

  it(`window_read and the performance rule: ${V.windowRead.length} cases`, () => {
    for (const v of V.windowRead) {
      const data = hex(v.data);
      const s = windowReadRaw(data, BigInt(v.poolQuoteVolume), BigInt(v.poolSwapCount), BigInt(v.now), BigInt(v.window));
      const b = windowReadRaw(data, BigInt(v.poolQuoteVolume), BigInt(v.poolSwapCount), BigInt(v.now), BigInt(v.baseWindow));
      expect(s).toEqual(read(v.read));
      expect(b).toEqual(read(v.baseRead));
      expect(performanceHolds(v.rule, s, b)).toBe(v.holds);
    }
  });

  it(`season score: ${V.seasonScore.length} cases`, () => {
    for (const v of V.seasonScore) {
      const c = v.counters;
      const score = seasonScoreExact(
        { raidVolumeWon: BigInt(c.raidVolumeWon!), sieges: BigInt(c.sieges!), siegeSpend: BigInt(c.siegeSpend!), timesBesieged: BigInt(c.timesBesieged!), counterStrikes: BigInt(c.counterStrikes!), treatySecs: BigInt(c.treatySecs!) },
        v.weights.map(BigInt) as [bigint, bigint, bigint, bigint, bigint, bigint],
        v.penalizeBesieged,
      );
      expect(score === null ? null : score.toString()).toBe(v.score);
    }
  });

  it(`loot draw: ${V.lootDraw.length} cases`, () => {
    for (const v of V.lootDraw) {
      const r = lootDraw(v.entries, hex(v.value));
      expect(r ? r.templateId : null).toBe(v.templateId);
      expect(r ? r.params : null).toEqual(v.params);
    }
  });

  it(`bps_of: ${V.bpsOf.length} cases`, () => {
    for (const v of V.bpsOf) expect(bpsOf(BigInt(v.amount), BigInt(v.part)).toString()).toBe(v.out);
  });
});
