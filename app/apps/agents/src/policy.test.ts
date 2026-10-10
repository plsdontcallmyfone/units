import { describe, expect, it } from 'vitest';
import { applyFill, checkReason, checkTrade, clearHalt, DAY_SECS, emptyBook, equityOf, observe, tighten, type Book, type Identity, type MintFacts, type TradeCaps, type TradeRequest } from './policy.ts';

// TEST values for the engine's tests only; they are not recommendations.
const caps: TradeCaps = { maxPositionLamports: 1_000n, maxTradeLamports: 400n, dailyLossHaltLamports: 300n, drawdownHaltBps: 2_000, maxSlippageBps: 100, minHoldSecs: 600 };
const T0 = 1_800_000_000;
const id: Identity = { agentKey: 'AGENT', vault: 'VAULT', operatorKeys: new Set(['OPERATOR', 'SISTER_AGENT', 'SISTER_VAULT']) };
const clean: MintFacts = { creator: 'SOMEONE', itemAuthors: ['OTHER_AUTHOR'] };
const buy = (over: Partial<TradeRequest> = {}): TradeRequest => ({ side: 'buy', mint: 'M', amountIn: 100n, quotedOut: 1_000n, minOut: 995n, reason: 'thin book after a royalty settle', ...over });
const sell = (over: Partial<TradeRequest> = {}): TradeRequest => ({ side: 'sell', mint: 'M', amountIn: 500n, quotedOut: 60n, minOut: 60n, reason: 'taking the position down', ...over });
const codes = (b: Book, r: TradeRequest, f: MintFacts = clean, now = T0) => checkTrade(b, caps, r, f, id, now).map((x) => x.code);
const fresh = () => emptyBook(T0, 10_000n);

describe('policy engine', () => {
  it('allows a clean buy within every cap', () => {
    expect(codes(fresh(), buy())).toEqual([]);
  });

  it('caps the trade size (buy: lamports in; sell: lamports out)', () => {
    expect(codes(fresh(), buy({ amountIn: 401n, quotedOut: 4_010n, minOut: 4_000n }))).toContain('TradeSize');
    expect(codes(fresh(), buy({ amountIn: 400n }))).not.toContain('TradeSize');
    const b = applyFill(fresh(), buy({ amountIn: 400n }), 5_000n, T0);
    expect(codes(b, sell({ amountIn: 100n, quotedOut: 401n, minOut: 401n }), clean, T0 + 601)).toContain('TradeSize');
  });

  it('caps the position cost basis per token', () => {
    let b = fresh();
    b = applyFill(b, buy({ amountIn: 400n }), 4_000n, T0);
    b = applyFill(b, buy({ amountIn: 400n }), 4_000n, T0);
    expect(codes(b, buy({ amountIn: 200n, quotedOut: 2_000n, minOut: 1_990n }))).toEqual([]);
    expect(codes(b, buy({ amountIn: 201n, quotedOut: 2_010n, minOut: 2_000n }))).toContain('PositionCap');
  });

  it('caps slippage: minOut may sit at most maxSlippageBps under the quote', () => {
    expect(codes(fresh(), buy({ minOut: 990n }))).toEqual([]);
    expect(codes(fresh(), buy({ minOut: 989n }))).toContain('SlippageCap');
    expect(codes(fresh(), buy({ minOut: 1_001n }))).toContain('BadAmount');
    expect(codes(fresh(), buy({ amountIn: 0n }))).toContain('BadAmount');
  });

  it('holds a bought token for the minimum time and never sells more than it bought', () => {
    const b = applyFill(fresh(), buy(), 1_000n, T0);
    expect(codes(b, sell(), clean, T0 + 599)).toContain('MinHold');
    expect(codes(b, sell(), clean, T0 + 600)).toEqual([]);
    expect(codes(b, sell({ amountIn: 1_001n }), clean, T0 + 600)).toContain('NoPosition');
    expect(codes(fresh(), sell())).toContain('NoPosition');
  });

  it('never trades a token created by the agent or its operator', () => {
    for (const creator of ['AGENT', 'VAULT', 'OPERATOR', 'SISTER_AGENT', 'SISTER_VAULT']) {
      expect(codes(fresh(), buy(), { creator, itemAuthors: [] })).toContain('SameOperator');
    }
  });

  it('never trades a token carrying an item it or its operator authored', () => {
    expect(codes(fresh(), buy(), { creator: 'X', itemAuthors: ['Y', 'AGENT'] })).toContain('OwnItemToken');
    expect(codes(fresh(), buy(), { creator: 'X', itemAuthors: ['SISTER_AGENT'] })).toContain('OwnItemToken');
    const b = applyFill(fresh(), buy(), 1_000n, T0);
    expect(codes(b, sell(), { creator: 'X', itemAuthors: ['AGENT'] }, T0 + 601)).toContain('OwnItemToken');
  });

  it('requires a public reason within the naming rules', () => {
    expect(checkReason('')).not.toBeNull();
    expect(checkReason('x'.repeat(141))).not.toBeNull();
    for (const w of ['yield', 'APR', 'apy', 'reflections', 'tax', 'bet', 'odds']) expect(checkReason(`good ${w} here`)).not.toBeNull();
    expect(checkReason('alphabet and taxonomy are fine words')).toBeNull();
    expect(checkReason('a dash \u2014 here')).not.toBeNull();
    expect(codes(fresh(), buy({ reason: '' }))).toContain('BadReason');
  });

  it('halts for the day on the daily loss, and lifts it the next day', () => {
    let b = observe(fresh(), { vaultLamports: 10_000n, values: {} }, caps, T0);
    b = observe(b, { vaultLamports: 9_700n, values: {} }, caps, T0 + 60);
    expect(b.halt?.kind).toBe('daily-loss');
    expect(codes(b, buy())).toContain('DailyLossHalt');
    b = observe(b, { vaultLamports: 9_700n, values: {} }, caps, b.dayStart + DAY_SECS);
    expect(b.halt).toBeNull();
    expect(codes(b, buy())).toEqual([]);
  });

  it('halts on drawdown from the peak until the operator clears it', () => {
    const c = { ...caps, dailyLossHaltLamports: 1_000_000n };
    let b = observe(fresh(), { vaultLamports: 12_000n, values: {} }, c, T0);
    expect(b.equityPeak).toBe(12_000n);
    b = observe(b, { vaultLamports: 9_601n, values: {} }, c, T0 + 10);
    expect(b.halt).toBeNull();
    b = observe(b, { vaultLamports: 9_600n, values: {} }, c, T0 + 20);
    expect(b.halt?.kind).toBe('drawdown');
    b = observe(b, { vaultLamports: 9_600n, values: {} }, c, T0 + 2 * DAY_SECS);
    expect(b.halt?.kind).toBe('drawdown');
    expect(checkTrade(b, c, buy(), clean, id, T0).map((x) => x.code)).toContain('DrawdownHalt');
    b = clearHalt(b, 9_600n);
    expect(b.halt).toBeNull();
  });

  it('values positions at their mark, or at cost without one', () => {
    const b = applyFill(fresh(), buy(), 1_000n, T0);
    expect(equityOf(b, { vaultLamports: 50n, values: {} })).toBe(150n);
    expect(equityOf(b, { vaultLamports: 50n, values: { M: 80n } })).toBe(130n);
  });

  it('books fills: cost basis moves pro rata on a sell and the realised result is kept', () => {
    let b = applyFill(fresh(), buy({ amountIn: 100n }), 1_000n, T0);
    b = applyFill(b, sell({ amountIn: 400n }), 60n, T0 + 700);
    expect(b.positions.M).toEqual({ qty: 600n, costLamports: 60n, lastBuyAt: T0 });
    expect(b.realizedTodayLamports).toBe(20n);
    b = applyFill(b, sell({ amountIn: 600n }), 50n, T0 + 800);
    expect(b.positions.M).toBeUndefined();
  });

  it('lets a directive only tighten the caps', () => {
    const t = tighten(caps, { maxTradeLamports: 50n, maxSlippageBps: 500, minHoldSecs: 60, drawdownHaltBps: 100 });
    expect(t.maxTradeLamports).toBe(50n);
    expect(t.maxSlippageBps).toBe(100);
    expect(t.minHoldSecs).toBe(600);
    expect(t.drawdownHaltBps).toBe(100);
    expect(tighten(caps, { minHoldSecs: 6_000 }).minHoldSecs).toBe(6_000);
  });
});
