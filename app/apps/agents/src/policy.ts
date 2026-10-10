// The deterministic trading policy engine. A model may propose a trade; this code decides. Every
// cap and integrity rule is checked here before anything is signed, and the signer is the agent's
// policy vault (through `hookwars_agents::spend`), whose on-chain limits hold even if this code is
// wrong. Spot only: there is no leverage, borrowing or short side anywhere in this file.

export const DAY_SECS = 86_400;

/** Words the naming rules (docs/spec/00 section 4.5) keep out of anything an agent publishes. */
export const BANNED_WORDS = ['yield', 'apr', 'apy', 'reflection', 'reflections', 'tax', 'bet', 'bets', 'betting', 'odds'];

export interface TradeCaps {
  /** Highest cost basis (lamports) the vault may hold in one token after a buy. */
  maxPositionLamports: bigint;
  /** Highest lamports in (buy) or expected out (sell) of one trade. */
  maxTradeLamports: bigint;
  /** Trading halts for the rest of the day once equity is this many lamports under the day's start. */
  dailyLossHaltLamports: bigint;
  /** Trading halts once equity is this many basis points under its peak (cleared by the operator). */
  drawdownHaltBps: number;
  /** `minOut` may sit at most this many basis points under the quote. */
  maxSlippageBps: number;
  /** Seconds a bought token must be held before any of it may be sold. */
  minHoldSecs: number;
}

export interface Position { qty: bigint; costLamports: bigint; lastBuyAt: number }

export interface Book {
  positions: Record<string, Position>;
  dayStart: number;
  dayStartEquity: bigint;
  equityPeak: bigint;
  realizedTodayLamports: bigint;
  halt: { kind: 'daily-loss' | 'drawdown'; at: number } | null;
}

export interface TradeRequest {
  side: 'buy' | 'sell';
  mint: string;
  /** Lamports in for a buy, token base units in for a sell. */
  amountIn: bigint;
  /** The router's quote of what comes out. */
  quotedOut: bigint;
  /** The least the trade may return (the swap's own floor). */
  minOut: bigint;
  /** The public reason, published in the trade's memo. */
  reason: string;
}

/** What the chain says about the traded token, for the integrity rules. */
export interface MintFacts { creator: string; itemAuthors: string[] }

/** Who the agent is: its own keys and every key of its operator (all passports, agent keys and vaults). */
export interface Identity { agentKey: string; vault: string; operatorKeys: ReadonlySet<string> }

export interface Marks {
  vaultLamports: bigint;
  /** Lamports the vault would get for each position now (a sell quote); a missing mark counts at cost. */
  values: Record<string, bigint>;
}

export type RefusalCode =
  | 'Halted' | 'DailyLossHalt' | 'DrawdownHalt' | 'BadReason' | 'BadAmount' | 'SlippageCap' | 'TradeSize'
  | 'PositionCap' | 'NoPosition' | 'MinHold' | 'SameOperator' | 'OwnItemToken';

export interface Refusal { code: RefusalCode; detail: string }

export function emptyBook(now: number, equity: bigint): Book {
  return { positions: {}, dayStart: now - (now % DAY_SECS), dayStartEquity: equity, equityPeak: equity, realizedTodayLamports: 0n, halt: null };
}

export function equityOf(book: Book, marks: Marks): bigint {
  let e = marks.vaultLamports;
  for (const [mint, p] of Object.entries(book.positions)) e += marks.values[mint] ?? p.costLamports;
  return e;
}

/** Tighter of two caps sets (a directive may only tighten what the config allows). */
export function tighten(a: TradeCaps, b: Partial<TradeCaps>): TradeCaps {
  const minB = (x: bigint, y: bigint | undefined): bigint => (y === undefined || x < y ? x : y);
  const minN = (x: number, y: number | undefined): number => (y === undefined || x < y ? x : y);
  return {
    maxPositionLamports: minB(a.maxPositionLamports, b.maxPositionLamports),
    maxTradeLamports: minB(a.maxTradeLamports, b.maxTradeLamports),
    dailyLossHaltLamports: minB(a.dailyLossHaltLamports, b.dailyLossHaltLamports),
    drawdownHaltBps: minN(a.drawdownHaltBps, b.drawdownHaltBps),
    maxSlippageBps: minN(a.maxSlippageBps, b.maxSlippageBps),
    // A longer hold is the tighter rule.
    minHoldSecs: b.minHoldSecs === undefined || a.minHoldSecs > b.minHoldSecs ? a.minHoldSecs : b.minHoldSecs,
  };
}

/** Rolls the day and updates the peak and halts from the current equity. Returns the new book. */
export function observe(book: Book, marks: Marks, caps: TradeCaps, now: number): Book {
  const b: Book = { ...book, positions: { ...book.positions } };
  const equity = equityOf(b, marks);
  if (now >= b.dayStart + DAY_SECS) {
    b.dayStart = now - (now % DAY_SECS);
    b.dayStartEquity = equity;
    b.realizedTodayLamports = 0n;
    if (b.halt?.kind === 'daily-loss') b.halt = null;
  }
  if (equity > b.equityPeak) b.equityPeak = equity;
  if (!b.halt) {
    if (b.dayStartEquity - equity >= caps.dailyLossHaltLamports) b.halt = { kind: 'daily-loss', at: now };
    else if (b.equityPeak > 0n && (b.equityPeak - equity) * 10_000n >= BigInt(caps.drawdownHaltBps) * b.equityPeak) b.halt = { kind: 'drawdown', at: now };
  }
  return b;
}

export function checkReason(reason: string): string | null {
  const r = reason.trim();
  if (!r) return 'a trade needs a public reason';
  if (r.length > 140) return 'the reason is longer than 140 characters';
  if (/[\u0000-\u001f\u2014]/.test(r)) return 'the reason has control characters or an em dash';
  const words = r.toLowerCase().split(/[^a-z]+/);
  const bad = BANNED_WORDS.find((w) => words.includes(w));
  if (bad) return `the reason uses "${bad}", which the naming rules forbid`;
  return null;
}

/** Every refusal that applies to `req` (empty = allowed). `book` must already be `observe`d. */
export function checkTrade(book: Book, caps: TradeCaps, req: TradeRequest, facts: MintFacts, id: Identity, now: number): Refusal[] {
  const out: Refusal[] = [];
  if (book.halt) out.push({ code: book.halt.kind === 'daily-loss' ? 'DailyLossHalt' : 'DrawdownHalt', detail: `trading halted (${book.halt.kind}) since ${book.halt.at}` });
  const bad = checkReason(req.reason);
  if (bad) out.push({ code: 'BadReason', detail: bad });
  if (req.amountIn <= 0n || req.quotedOut <= 0n || req.minOut <= 0n) out.push({ code: 'BadAmount', detail: 'amounts must be above zero' });
  if (req.minOut > req.quotedOut) out.push({ code: 'BadAmount', detail: 'minOut is above the quote' });
  if (req.minOut * 10_000n < req.quotedOut * BigInt(10_000 - caps.maxSlippageBps)) out.push({ code: 'SlippageCap', detail: `minOut is more than ${caps.maxSlippageBps} bps under the quote` });
  const lamports = req.side === 'buy' ? req.amountIn : req.quotedOut;
  if (lamports > caps.maxTradeLamports) out.push({ code: 'TradeSize', detail: `${lamports} lamports is over the trade cap ${caps.maxTradeLamports}` });
  const pos = book.positions[req.mint];
  if (req.side === 'buy') {
    const after = (pos?.costLamports ?? 0n) + req.amountIn;
    if (after > caps.maxPositionLamports) out.push({ code: 'PositionCap', detail: `position would cost ${after} lamports, over ${caps.maxPositionLamports}` });
  } else {
    if (!pos || pos.qty < req.amountIn) out.push({ code: 'NoPosition', detail: 'selling more than the vault bought' });
    else if (now - pos.lastBuyAt < caps.minHoldSecs) out.push({ code: 'MinHold', detail: `held ${now - pos.lastBuyAt} s, the minimum is ${caps.minHoldSecs} s` });
  }
  const mine = new Set([id.agentKey, id.vault, ...id.operatorKeys]);
  if (mine.has(facts.creator)) out.push({ code: 'SameOperator', detail: 'the token was created by this agent or its operator' });
  if (facts.itemAuthors.some((a) => mine.has(a))) out.push({ code: 'OwnItemToken', detail: 'an item authored by this agent or its operator is equipped on the token' });
  return out;
}

/** The book after a fill (`out` = what actually came back). */
export function applyFill(book: Book, req: TradeRequest, out: bigint, now: number): Book {
  const b: Book = { ...book, positions: { ...book.positions } };
  const pos = b.positions[req.mint] ?? { qty: 0n, costLamports: 0n, lastBuyAt: 0 };
  if (req.side === 'buy') {
    b.positions[req.mint] = { qty: pos.qty + out, costLamports: pos.costLamports + req.amountIn, lastBuyAt: now };
  } else {
    const costSold = pos.qty === 0n ? 0n : (pos.costLamports * req.amountIn) / pos.qty;
    const qty = pos.qty - req.amountIn;
    b.realizedTodayLamports += out - costSold;
    if (qty <= 0n) delete b.positions[req.mint];
    else b.positions[req.mint] = { qty, costLamports: pos.costLamports - costSold, lastBuyAt: pos.lastBuyAt };
  }
  return b;
}

/** The operator clears a drawdown halt (a new directive, or `units-agents clear-halt`). */
export function clearHalt(book: Book, equity: bigint): Book {
  return { ...book, halt: null, equityPeak: equity };
}
