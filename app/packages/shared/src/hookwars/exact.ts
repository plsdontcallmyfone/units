/**
 * Exact ports of the Rust the programs run, pinned by `vectors/hookwars-math.json` (rendered by
 * `programs/tests/tests/app_vectors.rs`): `hookwars_common::{shape, combine, window_read,
 * PerformanceRule::holds}`, `hookwars_war::state::{Season::score, bps_of}` and
 * `hookwars_war::instructions::loot::draw`. Integer math is bigint with Rust's floors, saturations
 * and wrapping where the Rust has them.
 */

export const PARAM_FIELDS = 11;
const BPS = 10_000n;
const U128 = 1n << 128n;
const U128_MAX = U128 - 1n;
const U64_MAX = (1n << 64n) - 1n;
const I128_MAX = (1n << 127n) - 1n;
const I128_MIN = -(1n << 127n);

export type ExactForgeRule = 'unused' | 'towardCeiling' | 'towardFloor' | 'keep' | 'floorWhenBothOn';
export interface Shape { id: number; kind: number; fieldCount: number; forgeable: boolean; rules: ExactForgeRule[]; zeroOff: boolean[] }

/** Slot kinds (00 4.1). */
const KIND = { fee: 0, defense: 2, relation: 3, pool: 4, war: 6 } as const;

/** `hookwars_common::shape`. */
export function shape(id: number): Shape | null {
  const T: Record<number, [number, ExactForgeRule[], boolean]> = {
    1: [KIND.pool, ['towardCeiling', 'towardCeiling', 'towardCeiling'], true],
    2: [KIND.pool, ['towardCeiling', 'towardCeiling', 'keep'], true],
    3: [KIND.defense, ['towardFloor'], true],
    4: [KIND.pool, ['keep', 'towardFloor', 'towardFloor', 'towardCeiling'], true],
    5: [KIND.relation, ['keep', 'keep', 'keep'], false],
    6: [KIND.relation, ['keep'], false],
    7: [KIND.fee, ['towardCeiling', 'towardCeiling', 'towardCeiling'], true],
    8: [KIND.fee, ['towardCeiling', 'floorWhenBothOn'], true],
    9: [KIND.war, Array<ExactForgeRule>(11).fill('keep'), false],
  };
  const t = T[id];
  if (!t) return null;
  const rules: ExactForgeRule[] = Array<ExactForgeRule>(PARAM_FIELDS).fill('unused');
  t[1].forEach((r, i) => { rules[i] = r; });
  const zeroOff = Array<boolean>(PARAM_FIELDS).fill(false);
  if (id === 8) zeroOff[1] = true;
  return { id, kind: t[0], fieldCount: t[1].length, forgeable: t[2], rules, zeroOff };
}

export type ParamsError = 'UnknownTemplate' | 'OutOfRange' | 'BadParams' | 'NotForgeable';

/** `hookwars_common::combine` (the forge of two items' params). */
export function combine(id: number, min: number[], max: number[], gainBps: number, a: number[], b: number[]): { out: number[] } | { error: ParamsError } {
  const s = shape(id);
  if (!s) return { error: 'UnknownTemplate' };
  if (!s.forgeable) return { error: 'NotForgeable' };
  const gain = BigInt(Math.min(gainBps, 10_000));
  const up = (m: number, c: number): number => {
    const d = BigInt(Math.max(0, c - m));
    return Math.min(0xffff_ffff, m + Number((d * gain) / BPS));
  };
  const down = (m: number, f: number): number => {
    const d = BigInt(Math.max(0, m - f));
    return Math.max(0, m - Number((d * gain) / BPS));
  };
  const out = Array<number>(PARAM_FIELDS).fill(0);
  for (let i = 0; i < PARAM_FIELDS; i++) {
    const ai = a[i] ?? 0, bi = b[i] ?? 0, lo = min[i] ?? 0, hi = max[i] ?? 0;
    switch (s.rules[i]) {
      case 'unused': out[i] = 0; break;
      case 'towardCeiling': out[i] = up(Math.max(ai, bi), hi); break;
      case 'towardFloor': out[i] = down(Math.min(ai, bi), lo); break;
      case 'keep': if (ai !== bi) return { error: 'NotForgeable' }; out[i] = ai; break;
      case 'floorWhenBothOn':
        if (ai > 0 && bi > 0) out[i] = down(Math.min(ai, bi), lo);
        else if (ai === bi) out[i] = ai;
        else return { error: 'NotForgeable' };
        break;
    }
    if (s.rules[i] !== 'unused' && !(s.zeroOff[i] && out[i] === 0)) out[i] = Math.min(Math.max(out[i]!, lo), hi);
  }
  return { out };
}

/** `hookwars_common::obs_layout`: byte offsets in an `Observations` account. */
export const OBS_LAYOUT = { LAST_PRICE: 42, LAST_TS: 58, INDEX: 66, FILLED: 68, ENTRIES: 70, ENTRY: 48 } as const;

export interface ExactWindowRead { twapQ64: bigint; quoteVolume: bigint; swaps: bigint; seconds: bigint }

const dv = (d: Uint8Array) => new DataView(d.buffer, d.byteOffset, d.byteLength);
const u128At = (d: Uint8Array, o: number): bigint => (dv(d).getBigUint64(o + 8, true) << 64n) | dv(d).getBigUint64(o, true);

/** `hookwars_common::window_read` over the raw account bytes. Null is "no signal". */
export function windowReadRaw(data: Uint8Array, poolQuoteVolume: bigint, poolSwapCount: bigint, now: bigint, window: bigint): ExactWindowRead | null {
  const L = OBS_LAYOUT;
  if (window <= 0n || data.length < L.ENTRIES) return null;
  const ring = Math.floor((data.length - L.ENTRIES) / L.ENTRY);
  const v = dv(data);
  const filled = Math.min(v.getUint16(L.FILLED, true), ring);
  if (filled === 0) return null;
  const index = v.getUint16(L.INDEX, true) % Math.max(ring, 1);
  const lastPrice = u128At(data, L.LAST_PRICE);
  const lastTs = v.getBigInt64(L.LAST_TS, true);
  const entry = (i: number) => {
    const o = L.ENTRIES + i * L.ENTRY;
    return { ts: v.getBigInt64(o, true), priceCumulative: u128At(data, o + 8), quoteVolume: u128At(data, o + 24), swapCount: v.getBigUint64(o + 40, true) };
  };
  const newest = entry((index + ring - 1) % ring);
  const dtTail = now - lastTs > 0n ? now - lastTs : 0n;
  const cumNow = (newest.priceCumulative + ((lastPrice * dtTail) & U128_MAX)) & U128_MAX;
  const target = now - window;
  let best: ReturnType<typeof entry> | null = null;
  for (let i = 0; i < filled; i++) {
    const e = entry(i);
    if (e.ts <= target && (best === null || e.ts > best.ts)) best = e;
  }
  if (!best) return null;
  const secs = now - best.ts;
  if (secs <= 0n) return null;
  const sat = (x: bigint): bigint => (x < 0n ? 0n : x);
  return {
    twapQ64: (((cumNow - best.priceCumulative) % U128) + U128) % U128 / secs,
    quoteVolume: sat(poolQuoteVolume - best.quoteVolume),
    swaps: sat(poolSwapCount - best.swapCount),
    seconds: secs,
  };
}

const satMul128 = (a: bigint, b: bigint): bigint => { const p = a * b; return p > U128_MAX ? U128_MAX : p; };

/** `PerformanceRule::holds`: metric 0 quote volume, 1 TWAP, 2 swap count; op 0 below, 1 above. */
export function performanceHolds(rule: { metric: number; op: number; ratioBps: number }, short: ExactWindowRead | null, base: ExactWindowRead | null): boolean {
  if (!short || !base) return false;
  const r = BigInt(rule.ratioBps);
  let lhs: bigint, rhs: bigint;
  switch (rule.metric) {
    case 0:
      lhs = satMul128(satMul128(short.quoteVolume, base.seconds), BPS);
      rhs = satMul128(satMul128(base.quoteVolume, short.seconds), r);
      break;
    case 1:
      lhs = satMul128(short.twapQ64, BPS);
      rhs = satMul128(base.twapQ64, r);
      break;
    case 2:
      lhs = satMul128(satMul128(short.swaps, base.seconds), BPS);
      rhs = satMul128(satMul128(base.swaps, short.seconds), r);
      break;
    default: return false;
  }
  return rule.op === 0 ? lhs < rhs : lhs > rhs;
}

export interface ExactSeasonCounters { raidVolumeWon: bigint; sieges: bigint; siegeSpend: bigint; timesBesieged: bigint; counterStrikes: bigint; treatySecs: bigint }
/** Weights in the program's field order: raid volume won, sieges, siege spend, times besieged, counter-strikes, treaty seconds. */
export type ExactScoreWeights = [bigint, bigint, bigint, bigint, bigint, bigint];

/** `Season::score`: checked i128; null on overflow. */
export function seasonScoreExact(c: ExactSeasonCounters, w: ExactScoreWeights, penalizeBesieged: boolean): bigint | null {
  const ok = (v: bigint) => v <= I128_MAX && v >= I128_MIN;
  const term = (v: bigint, weight: bigint) => { const t = v * weight; return ok(t) ? t : null; };
  const besieged = term(c.timesBesieged, w[3]);
  if (besieged === null) return null;
  let s: bigint | null = term(c.raidVolumeWon, w[0]);
  for (const t of [term(c.sieges, w[1]), term(c.siegeSpend, w[2]), term(c.counterStrikes, w[4]), term(c.treatySecs, w[5])]) {
    if (s === null || t === null) return null;
    s += t;
    if (!ok(s)) return null;
  }
  if (s === null) return null;
  const r = penalizeBesieged ? s - besieged : s + besieged;
  return ok(r) ? r : null;
}

export interface LootEntryInput { templateId: number; weight: number; ranges: [number, number][] }

/** `loot::draw`: the template and params a randomness value picks from a season's loot table. */
export function lootDraw(entries: LootEntryInput[], value: Uint8Array): { templateId: number; params: number[] } | null {
  const total = entries.reduce((n, e) => n + BigInt(e.weight), 0n);
  if (total === 0n) return null;
  const v = dv(value);
  let pick = v.getBigUint64(0, true) % total;
  const entry = entries.find((e) => {
    const w = BigInt(e.weight);
    if (pick < w) return true;
    pick -= w;
    return false;
  });
  if (!entry) return null;
  const params = Array<number>(PARAM_FIELDS).fill(0);
  for (let i = 0; i < PARAM_FIELDS; i++) {
    const r = BigInt(v.getUint16(8 + 2 * i, true));
    const [min, max] = entry.ranges[i] ?? [0, 0];
    const span = BigInt(max - min) + 1n;
    params[i] = min + Number((r * span) >> 16n);
  }
  return { templateId: entry.templateId, params };
}

/** `bps_of(amount, part)`: `part` basis points of `amount`, rounded down, as u64. */
export function bpsOf(amount: bigint, part: bigint): bigint {
  return ((amount * part) / BPS) & U64_MAX;
}

/** War orders field indices (`hookwars_war::constants::orders`). */
export const WAR_ORDERS = {
  SIEGE_THRESHOLD: 0, SIEGE_SPEND_BPS: 1, SIEGE_TWAP_SECS: 2, COUNTER_DROP_BPS: 3, COUNTER_SHORT_SECS: 4, COUNTER_LONG_SECS: 5,
  COUNTER_INTERVAL_SECS: 6, COUNTER_SPEND_BPS: 7, RAZE_ENABLED: 8, BOUNTY_RATE: 9, CRANK_BOUNTY_BPS: 10,
} as const;
/** The Treaty template's `returns_captured` field (`hookwars_war::constants::TREATY_RETURNS_CAPTURED`). */
export const TREATY_RETURNS_CAPTURED = 2;
