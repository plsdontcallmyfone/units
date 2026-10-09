/**
 * The Hookwars math the site and backend mirror, each function citing the spec section whose
 * formula it implements. All integer math is in bigint with floors, as the programs do. Once the
 * programs land, each is pinned to the Rust by fixture vectors (INTEGRATION.md section 4), as
 * upstream pins `launch-fees.json`.
 */
import type { ForgeRule } from './templates.ts';

const BPS = 10_000n;

/** 04 2.7: the result of forging two values of one field. Returns null when `keep` values differ
 * (`NotForgeable`) or the field is not forgeable. Clamped to [floor, ceiling] as the armory clamps
 * again (02 9.1 step 2). */
export function forgeField(rule: ForgeRule, a: number, b: number, floor: number, ceiling: number, gainBps: number): number | null {
  let out: number;
  switch (rule) {
    case 'towardCeiling': {
      const m = Math.max(a, b);
      out = m + Math.floor(((ceiling - m) * gainBps) / 10_000);
      break;
    }
    case 'towardFloor': {
      const m = Math.min(a, b);
      out = m - Math.floor(((m - floor) * gainBps) / 10_000);
      break;
    }
    case 'keep':
      if (a !== b) return null;
      out = a;
      break;
    case 'floorWhenBothOn': {
      // 0 is "off": forge toward the floor only when both are on (hookwars_common::combine).
      if (a > 0 && b > 0) { const m = Math.min(a, b); out = m - Math.floor(((m - floor) * gainBps) / 10_000); }
      else if (a === b) return a;
      else return null;
      if (out === 0) return 0;
      break;
    }
    case 'none':
      return null;
  }
  return Math.min(Math.max(out, floor), ceiling);
}

export interface SettleSplit { royalty: bigint; bounty: bigint; rest: bigint }

/** 04 2.5: one side of `settle_equip`. */
export function settleSplit(balance: bigint, royaltyBps: number, crankBountyBps: number): SettleSplit {
  const royalty = (balance * BigInt(royaltyBps)) / BPS;
  const bounty = ((balance - royalty) * BigInt(crankBountyBps)) / BPS;
  return { royalty, bounty, rest: balance - royalty - bounty };
}

/** 06 2.4 royalty position of one item on one cut mint: unsettled (vault balance plus pool owed not
 * settled) and what settling it would pay as royalty now, by 04 2.5's formula on each side. */
export function royaltyPosition(vaultBalance: bigint, poolOwed: bigint, poolSettled: bigint, royaltyBps: number): { unsettled: bigint; settlesToRoyalty: bigint } {
  const pool = poolOwed > poolSettled ? poolOwed - poolSettled : 0n;
  return {
    unsettled: vaultBalance + pool,
    settlesToRoyalty: (vaultBalance * BigInt(royaltyBps)) / BPS + (pool * BigInt(royaltyBps)) / BPS,
  };
}

/** The observation ring in a pool account (03 3.1 as built in M3a, `bordrless_core::observations`).
 * `cumulative` is the header's running sum; `priceCumulative` wraps at 2^128. */
export interface Observation { ts: bigint; priceCumulative: bigint; quoteVolume: bigint; swapCount: bigint }
export interface ObservationsView { cumulative: bigint; lastPriceQ64: bigint; lastTs: bigint; index: number; filled: number; entries: Observation[] }
export interface WindowRead { twapQ64: bigint; quoteVolume: bigint; swaps: bigint }

const U128 = 1n << 128n;

/** `bordrless_core::observations::window_read` on a decoded ring: null is "no signal" (window
 * under the minimum, or no entry old enough), which every template and the Performance rule treat
 * as no effect. Walks newest first and uses the first entry at or before `now - window`. */
export function windowRead(obs: ObservationsView, poolQuoteVolume: bigint, poolSwapCount: bigint, now: bigint, window: bigint, minTwapSecs: bigint | null): WindowRead | null {
  if (window <= 0n || (minTwapSecs !== null && window < minTwapSecs)) return null;
  const len = obs.entries.length;
  if (len === 0 || obs.filled === 0) return null;
  const cumNow = now > obs.lastTs ? (obs.cumulative + obs.lastPriceQ64 * (now - obs.lastTs)) % U128 : obs.cumulative;
  const target = now - window;
  let i = (obs.index - 1 + len) % len;
  for (let k = 0; k < obs.filled; k++) {
    const e = obs.entries[i];
    if (e && e.ts <= target) {
      const span = now - e.ts;
      if (span <= 0n) return null;
      const sat = (x: bigint): bigint => (x < 0n ? 0n : x);
      return {
        twapQ64: ((cumNow - e.priceCumulative) % U128 + U128) % U128 / span,
        quoteVolume: sat(poolQuoteVolume - e.quoteVolume),
        swaps: sat(poolSwapCount - e.swapCount),
      };
    }
    i = (i - 1 + len) % len;
  }
  return null;
}

/** Q64.64 to a decimal string with `digits` decimals (for display only). */
export function q64ToDecimal(q: bigint, digits = 9): string {
  const scale = 10n ** BigInt(digits);
  const v = (q * scale) >> 64n;
  const s = v.toString().padStart(digits + 1, '0');
  return `${s.slice(0, -digits)}.${s.slice(-digits)}`.replace(/\.?0+$/, '');
}

/** 05 2.5 and 04 2.9: rolling inbound raid volume at `t`. */
export interface RaidWindow { windowStart: bigint; volume: bigint; prevVolume: bigint }
export function rollingRaidVolume(w: RaidWindow, t: bigint, windowSecs: bigint): bigint {
  if (windowSecs <= 0n) return w.volume;
  const end = w.windowStart + windowSecs;
  if (t >= w.windowStart + 2n * windowSecs) return 0n;
  if (t >= end) {
    // The current window ended; it becomes the previous one for the next window.
    const overlap = w.windowStart + 2n * windowSecs - t;
    return (w.volume * overlap) / windowSecs;
  }
  const overlap = end - t;
  return w.volume + (w.prevVolume * overlap) / windowSecs;
}

/** 04 2.9 adding `v` at `now` to an entry (the program's write rule, mirrored for quotes). */
export function raidWindowAdd(w: RaidWindow | null, v: bigint, now: bigint, windowSecs: bigint): RaidWindow {
  if (w === null) return { windowStart: now, volume: v, prevVolume: 0n };
  const k = (now - w.windowStart) / windowSecs;
  let { volume, prevVolume } = w;
  if (k === 1n) { prevVolume = volume; volume = 0n; }
  if (k >= 2n) { prevVolume = 0n; volume = 0n; }
  return { windowStart: w.windowStart + k * windowSecs, volume: volume + v, prevVolume };
}

/** 05 7: what `claim_bounty` pays and how many points it spends. */
export function bounty(points: bigint, rate: bigint, maxPerClaim: bigint | null, chestBalance: bigint): { pay: bigint; spent: bigint } {
  if (points <= 0n || rate <= 0n) return { pay: 0n, spent: 0n };
  let pay = points * rate;
  if (maxPerClaim !== null && pay > maxPerClaim) pay = maxPerClaim;
  if (pay > chestBalance) pay = chestBalance;
  let spent = (pay + rate - 1n) / rate;
  if (spent > points) spent = points;
  return { pay, spent };
}

/** 05 6.2 and 6.3: a siege or counter-strike spend, and the crank bounty out of it (6.1 step 4). */
export function warSpend(chestBalance: bigint, spendBps: number, quoteSide: bigint, poolShareBps: number, crankBountyBps: number): { spend: bigint; bounty: bigint; moved: bigint } {
  const byChest = (chestBalance * BigInt(spendBps)) / BPS;
  const byPool = (quoteSide * BigInt(poolShareBps)) / BPS;
  const spend = byChest < byPool ? byChest : byPool;
  const b = (spend * BigInt(crankBountyBps)) / BPS;
  return { spend, bounty: b, moved: spend - b };
}

/** 05 6.3 trigger: `twap(short) * 10_000 <= twap(long) * (10_000 - counter_drop_bps)`. */
export function counterStrikeDue(shortTwap: bigint, longTwap: bigint, dropBps: number): boolean {
  return shortTwap * BPS <= longTwap * (BPS - BigInt(dropBps));
}

/** 05 10.2 season counters and weights. */
export interface SeasonCounters { raidVolumeWon: bigint; sieges: bigint; siegeSpend: bigint; timesBesieged: bigint; counterStrikes: bigint; treatySecs: bigint }
export interface ScoreWeights { raid: bigint; sieges: bigint; spend: bigint; besieged: bigint; counter: bigint; treaty: bigint }

const I128_MAX = (1n << 127n) - 1n;
const I128_MIN = -(1n << 127n);

/** 05 10.2: the season score in i128 with checked arithmetic. Throws on overflow (MathOverflow). */
export function seasonScore(c: SeasonCounters, w: ScoreWeights, penalizeBesieged: boolean): bigint {
  const check = (v: bigint): bigint => { if (v > I128_MAX || v < I128_MIN) throw new RangeError('MathOverflow'); return v; };
  let s = 0n;
  s = check(s + check(c.raidVolumeWon * w.raid));
  s = check(s + check(c.sieges * w.sieges));
  s = check(s + check(c.siegeSpend * w.spend));
  s = check(s + check(c.counterStrikes * w.counter));
  s = check(s + check(c.treatySecs * w.treaty));
  const b = check(c.timesBesieged * w.besieged);
  s = check(penalizeBesieged ? s - b : s + b);
  return s;
}

/** 05 10.3: whether `candidate` beats `leader` (strictly greater, or equal with a lower mint key). */
export function beatsLeader(candidateScore: bigint, candidateMint: string, leaderScore: bigint | null, leaderMintBytes: Uint8Array | null, candidateMintBytes: Uint8Array): boolean {
  if (leaderScore === null || leaderMintBytes === null) return true;
  if (candidateScore !== leaderScore) return candidateScore > leaderScore;
  for (let i = 0; i < 32; i++) {
    const a = candidateMintBytes[i] ?? 0, b = leaderMintBytes[i] ?? 0;
    if (a !== b) return a < b;
  }
  void candidateMint;
  return false;
}

/** 01 3.2 per-slot and token-wide checks of a merged answer, mirrored for quotes: every cut within
 * its slot's bound, and the sum no more than the amount. Returns the delivered amount or a reason. */
export function mergeSlotCuts(amount: bigint, cuts: { slot: number; cut: bigint; maxCutBps: number }[]): { delivered: bigint } | { error: string } {
  let total = 0n;
  for (const c of cuts) {
    if (c.cut * BPS > amount * BigInt(c.maxCutBps)) return { error: `slot ${c.slot} cut above its bound` };
    total += c.cut;
  }
  if (total > amount) return { error: 'cuts above the amount' };
  return { delivered: amount - total };
}
