// Changed by Hookwars: new file, the explorer's decoder (explorer v2).
/**
 * The explorer's decoder: every instruction, account and event of the units programs, read with the
 * generated IDLs, plus the memo, compute budget and system programs a units transaction carries.
 *
 * Nothing here talks to a cluster. `explainTransaction` takes what `getTransaction` returns (the
 * message's keys, its top-level instructions, the inner instructions and the logs) and returns the
 * instructions in order with their inner calls nested, every event in execution order, and a
 * "story": the units facts a reader looks for (slot items that ran on each transfer, pool items on
 * swaps, raid marks, settlements and their split, war, market, craft, book and agent actions,
 * memos), each line built only from a decoded field.
 */
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { IdlCoder, Cursor, camel, type Idl, type IdlAccountItem, type IdlType } from './idl.ts';
import { IDLS } from './from-idl.ts';
import bridgeIdl from '../../idl/bordrless_bridge.json' with { type: 'json' };
import halfLifeIdl from '../../idl/half_life.json' with { type: 'json' };
import taxHookIdl from '../../idl/tax_hook.json' with { type: 'json' };
import craftIdl from '../../idl/hookwars_craft.json' with { type: 'json' };
import bookIdl from '../../idl/hookwars_book.json' with { type: 'json' };

/** Every program the explorer decodes with an IDL, by the app's program name. */
export const EXPLORER_IDLS: Record<string, Idl> = {
  ...IDLS,
  bridge: bridgeIdl as unknown as Idl,
  halfLife: halfLifeIdl as unknown as Idl,
  taxHook: taxHookIdl as unknown as Idl,
  craft: craftIdl as unknown as Idl,
  book: bookIdl as unknown as Idl,
};

/** Programs a units transaction may carry that have no IDL here. */
export const KNOWN_PROGRAMS: Record<string, string> = {
  '11111111111111111111111111111111': 'system',
  ComputeBudget111111111111111111111111111111: 'computeBudget',
  MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr: 'memo',
  Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo: 'memo',
  AddressLookupTab1e1111111111111111111111111: 'addressLookupTable',
  TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA: 'splToken',
  TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb: 'token2022',
  ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL: 'associatedToken',
  BPFLoaderUpgradeab1e11111111111111111111111: 'bpfLoaderUpgradeable',
};

/** Human names of the programs, for page titles. */
export const PROGRAM_TITLES: Record<string, string> = {
  token: 'Token standard', swap: 'DEX', bridge: 'Bridge', launch: 'Launchpad', kit: 'Kit', companion: 'Companion',
  armory: 'Armory', items: 'Items', war: 'War', agents: 'Agents', market: 'Market', social: 'Social',
  craft: 'Craft', book: 'Order book', halfLife: 'Half-Life', taxHook: 'Example fee hook',
  system: 'System', computeBudget: 'Compute budget', memo: 'Memo', addressLookupTable: 'Lookup tables',
  splToken: 'SPL Token', token2022: 'Token-2022', associatedToken: 'Associated token accounts', bpfLoaderUpgradeable: 'Upgradeable loader',
};

const coders = new Map<string, IdlCoder>();
function coder(name: string): IdlCoder | null {
  const idl = EXPLORER_IDLS[name];
  if (!idl) return null;
  let c = coders.get(name);
  if (!c) { c = new IdlCoder(idl); coders.set(name, c); }
  return c;
}

let byId: Map<string, string> | null = null;
/** The app's program name for a program id, or null when the explorer does not know it. */
export function programName(id: string): string | null {
  if (!byId) {
    byId = new Map();
    for (const [name, idl] of Object.entries(EXPLORER_IDLS)) byId.set(idl.address, name);
  }
  return byId.get(id) ?? KNOWN_PROGRAMS[id] ?? null;
}

/** The program id for an app program name (IDL programs only). */
export function programIdOf(name: string): string | null {
  return EXPLORER_IDLS[name]?.address ?? null;
}

/** Values made JSON-safe: pubkeys as base58, integers as decimal strings, bytes as 0x-hex. */
export function plain(v: unknown): unknown {
  if (typeof v === 'bigint') return v.toString(10);
  if (v instanceof PublicKey) return v.toBase58();
  if (Buffer.isBuffer(v) || v instanceof Uint8Array) return `0x${Buffer.from(v).toString('hex')}`;
  if (Array.isArray(v)) return v.map(plain);
  if (v && typeof v === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, x] of Object.entries(v as Record<string, unknown>)) out[camel(k)] = plain(x);
    return out;
  }
  return v;
}

// ------------------------------------------------------------------------------ memos --

/** A units memo, version 1 (docs/spec/11-hook-economy.md section 4.2; crates/units-memo). */
export interface UnitsMemo {
  kind: string; from: string; to: string; thread: string; replyTo: string;
  body: unknown; expiresAt: number;
}
const MEMO_KEYS = ['u', 'k', 'f', 't', 'th', 're', 'b', 'x'];
const MEMO_KINDS = new Set(['offer', 'counter', 'accept', 'listing', 'treaty', 'directive', 'status', 'ack']);

/** Parses a units memo, version 1, or null. It accepts only the canonical key order and kinds. */
export function parseUnitsMemo(text: string): UnitsMemo | null {
  if (!text.startsWith('{"u":')) return null;
  let v: unknown;
  try { v = JSON.parse(text); } catch { return null; }
  if (!v || typeof v !== 'object' || Array.isArray(v)) return null;
  const o = v as Record<string, unknown>;
  const keys = Object.keys(o);
  if (keys.length !== MEMO_KEYS.length || keys.some((k, i) => k !== MEMO_KEYS[i])) return null;
  if (o.u !== 1 || typeof o.k !== 'string' || !MEMO_KINDS.has(o.k)) return null;
  for (const k of ['f', 't', 'th', 're']) if (typeof o[k] !== 'string') return null;
  if (typeof o.x !== 'number' || !Number.isInteger(o.x) || o.x < 0) return null;
  if (JSON.stringify(o) !== text) return null;
  return { kind: o.k, from: o.f as string, to: o.t as string, thread: o.th as string, replyTo: o.re as string, body: o.b, expiresAt: o.x };
}

// ---------------------------------------------------------------------- instructions --

export interface DecodedAccountRef { name: string; pubkey: string; writable: boolean; signer: boolean }
export interface DecodedInstruction {
  /** Position among the transaction's top-level instructions; inner calls carry the parent's. */
  index: number;
  /** Call depth: 1 for a top-level instruction. */
  depth: number;
  programId: string;
  program: string | null;
  name: string | null;
  args: Record<string, unknown> | null;
  accounts: DecodedAccountRef[];
  /** For a memo: its text, and the units message when it is one. */
  memo?: { text: string; units: UnitsMemo | null };
  /** True when this is an Anchor self-CPI event (decoded under `events` instead). */
  eventCpi?: boolean;
  /** Why the data could not be decoded, when it could not. */
  undecoded?: string;
  inner: DecodedInstruction[];
}

const EVENT_TAG = Buffer.from([0xe4, 0x45, 0xa5, 0x2e, 0x51, 0xcb, 0x9a, 0x1d]);

function flatAccounts(items: IdlAccountItem[]): IdlAccountItem[] {
  const out: IdlAccountItem[] = [];
  const walk = (xs: IdlAccountItem[]) => { for (const a of xs) (a.accounts ? walk(a.accounts) : out.push(a)); };
  walk(items);
  return out;
}

function decodeSystem(data: Buffer): { name: string; args: Record<string, unknown> } | null {
  if (data.length < 4) return null;
  const tag = data.readUInt32LE(0);
  if (tag === 2 && data.length >= 12) return { name: 'transfer', args: { lamports: data.readBigUInt64LE(4).toString() } };
  if (tag === 0 && data.length >= 52) return { name: 'createAccount', args: { lamports: data.readBigUInt64LE(4).toString(), space: data.readBigUInt64LE(12).toString(), owner: new PublicKey(data.subarray(20, 52)).toBase58() } };
  if (tag === 8) return { name: 'allocate', args: {} };
  if (tag === 1) return { name: 'assign', args: {} };
  return { name: `instruction ${tag}`, args: {} };
}

function decodeComputeBudget(data: Buffer): { name: string; args: Record<string, unknown> } | null {
  if (data.length < 1) return null;
  switch (data[0]) {
    case 1: return data.length >= 5 ? { name: 'requestHeapFrame', args: { bytes: data.readUInt32LE(1) } } : null;
    case 2: return data.length >= 5 ? { name: 'setComputeUnitLimit', args: { units: data.readUInt32LE(1) } } : null;
    case 3: return data.length >= 9 ? { name: 'setComputeUnitPrice', args: { microLamports: data.readBigUInt64LE(1).toString() } } : null;
    case 4: return data.length >= 5 ? { name: 'setLoadedAccountsDataSizeLimit', args: { bytes: data.readUInt32LE(1) } } : null;
    default: return null;
  }
}

/** Decodes one instruction's data and accounts. `accounts` are the instruction's account keys in order. */
export function decodeInstruction(programId: string, data: Buffer, accounts: { pubkey: string; writable: boolean; signer: boolean }[], index = 0, depth = 1): DecodedInstruction {
  const program = programName(programId);
  const base: DecodedInstruction = { index, depth, programId, program, name: null, args: null, accounts: accounts.map((a, i) => ({ name: `account ${i}`, ...a })), inner: [] };
  if (program === 'memo') {
    const text = data.toString('utf8');
    return { ...base, name: 'memo', memo: { text, units: parseUnitsMemo(text) } };
  }
  if (program === 'system') {
    const d = decodeSystem(data);
    return d ? { ...base, ...d } : { ...base, undecoded: 'unknown system instruction' };
  }
  if (program === 'computeBudget') {
    const d = decodeComputeBudget(data);
    return d ? { ...base, ...d } : { ...base, undecoded: 'unknown compute budget instruction' };
  }
  const c = program ? coder(program) : null;
  if (!c) return { ...base, undecoded: program ? 'no interface for this program' : 'unknown program' };
  if (data.length >= 8 && data.subarray(0, 8).equals(EVENT_TAG)) return { ...base, name: 'event', eventCpi: true };
  const ix = c.idl.instructions.find((x) => data.length >= 8 && data.subarray(0, 8).equals(Buffer.from(x.discriminator)));
  if (!ix) return { ...base, undecoded: 'unknown instruction discriminator' };
  let args: Record<string, unknown> | null = null;
  let undecoded: string | undefined;
  try {
    const r = new Cursor(data, 8);
    args = {};
    for (const a of ix.args) args[camel(a.name)] = plain(c.read(r, a.type as IdlType));
  } catch (e) { undecoded = `arguments: ${e instanceof Error ? e.message : String(e)}`; }
  const names = flatAccounts(ix.accounts);
  const refs = accounts.map((a, i) => ({ name: names[i] ? camel(names[i]!.name) : `remaining ${i - names.length}`, ...a }));
  return { ...base, name: camel(ix.name), args, accounts: refs, ...(undecoded ? { undecoded } : {}) };
}

// --------------------------------------------------------------------------- accounts --

export interface DecodedAccount { program: string | null; type: string | null; data: Record<string, unknown> | null; error?: string }

/** Decodes an account by its owner and discriminator. */
export function decodeAccount(owner: string, data: Buffer): DecodedAccount {
  const program = programName(owner);
  const c = program ? coder(program) : null;
  if (!c) return { program, type: null, data: null };
  const type = c.accountNameOf(data);
  if (!type) return { program, type: null, data: null };
  try { return { program, type, data: plain(c.decodeAccount(type, data)) as Record<string, unknown> }; }
  catch (e) { return { program, type, data: null, error: e instanceof Error ? e.message : String(e) }; }
}

// ----------------------------------------------------------------------------- events --

export interface ExplorerEvent { ordinal: number; program: string; name: string; data: Record<string, unknown>; via: 'cpi' | 'log'; /** The top-level instruction it ran under. */ instruction: number }

function decodeEventFor(program: string, body: Buffer): { name: string; data: Record<string, unknown> } | null {
  const c = coder(program);
  if (!c) return null;
  try {
    const ev = c.decodeEvent(body);
    return ev ? { name: ev.name, data: plain(ev.data) as Record<string, unknown> } : null;
  } catch { return null; }
}

// ------------------------------------------------------------------------ transactions --

/** A compiled instruction as `getTransaction` returns it (json encoding). */
export interface RawCompiledInstruction { programIdIndex: number; accounts: number[]; data: string; stackHeight?: number | null }
export interface RawInner { index: number; instructions: RawCompiledInstruction[] }
export interface TxSource {
  signature?: string;
  slot?: number | null;
  blockTime?: number | null;
  err?: unknown;
  fee?: number | null;
  computeUnits?: number | null;
  /** Static keys, then lookup-table writable, then lookup-table read-only (the message's account order). */
  accountKeys: string[];
  /** Writable and signer flags per key, when the caller has them. */
  writable?: boolean[];
  signer?: boolean[];
  instructions: RawCompiledInstruction[];
  inner: RawInner[];
  logs: string[];
}

export type StoryKind = 'transfer' | 'swap' | 'poolItems' | 'raid' | 'settle' | 'war' | 'market' | 'craft' | 'book' | 'agents' | 'social' | 'armory' | 'launch' | 'memo' | 'other';
export interface StoryLine { kind: StoryKind; title: string; facts: [string, string][]; event?: number }

export interface ExplainedTransaction {
  signature: string | null; slot: number | null; blockTime: number | null; fee: number | null; computeUnits: number | null;
  ok: boolean; error: unknown;
  /** The program that failed and its error, by name when its interface lists it. */
  failure: { programId: string; program: string | null; code: number | null; name: string | null; message: string } | null;
  instructions: DecodedInstruction[];
  events: ExplorerEvent[];
  logsTruncated: boolean;
  story: StoryLine[];
  programs: string[];
}

function refs(tx: TxSource, idx: number[]): { pubkey: string; writable: boolean; signer: boolean }[] {
  return idx.map((i) => ({ pubkey: tx.accountKeys[i] ?? `#${i}`, writable: tx.writable?.[i] ?? false, signer: tx.signer?.[i] ?? false }));
}

/** Inner instructions nested by stack height under their top-level instruction. */
function nest(list: DecodedInstruction[]): DecodedInstruction[] {
  const roots: DecodedInstruction[] = [];
  const stack: DecodedInstruction[] = [];
  for (const ix of list) {
    while (stack.length && stack[stack.length - 1]!.depth >= ix.depth) stack.pop();
    if (stack.length) stack[stack.length - 1]!.inner.push(ix); else roots.push(ix);
    stack.push(ix);
  }
  return roots;
}

/** Decodes a whole transaction: instructions (with inner calls), events in execution order, and the story. */
export function explainTransaction(tx: TxSource): ExplainedTransaction {
  const instructions: DecodedInstruction[] = [];
  const innerOf: { d: DecodedInstruction; data: Buffer }[][] = [];
  tx.instructions.forEach((ix, i) => {
    const top = decodeInstruction(tx.accountKeys[ix.programIdIndex] ?? '', Buffer.from(bs58.decode(ix.data)), refs(tx, ix.accounts), i, 1);
    const inner = (tx.inner.find((g) => g.index === i)?.instructions ?? []).map((c) => {
      const programId = tx.accountKeys[c.programIdIndex] ?? '';
      const data = Buffer.from(bs58.decode(c.data));
      return { d: decodeInstruction(programId, data, refs(tx, c.accounts), i, c.stackHeight ?? 2), data };
    });
    top.inner = nest(inner.map((x) => x.d).filter((d) => !d.eventCpi));
    instructions.push(top);
    innerOf.push(inner);
  });
  // Execution order from the logs: every instruction run writes one "invoke [depth]" line, so the
  // n-th deeper invoke under top-level instruction i is inner instruction n of i. A self-CPI event
  // lands where its invoke is; an `emit!` event where its "Program data:" line is, under the
  // innermost running program.
  const all: ExplorerEvent[] = [];
  const push = (program: string, ev: { name: string; data: Record<string, unknown> } | null, via: 'cpi' | 'log', instruction: number) => {
    if (ev) all.push({ ordinal: all.length, program, name: ev.name, data: ev.data, via, instruction });
  };
  const used = innerOf.map(() => new Set<number>());
  const stack: string[] = [];
  let top = -1;
  let k = 0;
  for (const line of tx.logs) {
    const inv = /^Program (\w+) invoke \[(\d+)\]$/.exec(line);
    if (inv) {
      stack.push(inv[1]!);
      if (inv[2] === '1') { top++; k = 0; continue; }
      const c = innerOf[top]?.[k];
      if (c) { used[top]!.add(k); if (c.d.eventCpi && c.d.program) push(c.d.program, decodeEventFor(c.d.program, c.data.subarray(8)), 'cpi', top); }
      k++;
      continue;
    }
    if (/^Program (\w+) (success|failed)/.test(line)) { stack.pop(); continue; }
    const m = /^Program data: (.+)$/.exec(line);
    if (!m) continue;
    const program = programName(stack[stack.length - 1] ?? '');
    if (!program || !EXPLORER_IDLS[program]) continue;
    push(program, decodeEventFor(program, Buffer.from(m[1]!, 'base64')), 'log', Math.max(top, 0));
  }
  // Self-CPI events the logs did not reach (truncated logs): after everything else, in order.
  innerOf.forEach((inner, i) => inner.forEach((c, j) => {
    if (!used[i]!.has(j) && c.d.eventCpi && c.d.program) push(c.d.program, decodeEventFor(c.d.program, c.data.subarray(8)), 'cpi', i);
  }));
  const story: StoryLine[] = [];
  for (const e of all) { const s = storyOf(e); if (s) story.push({ ...s, event: e.ordinal }); }
  story.push(...royaltySplits(all));
  const walk = (xs: DecodedInstruction[]) => { for (const x of xs) { if (x.memo) story.push({ kind: 'memo', title: x.memo.units ? `Memo: ${x.memo.units.kind}` : 'Memo', facts: x.memo.units ? memoFacts(x.memo.units) : [['Text', x.memo.text]] }); walk(x.inner); } };
  walk(instructions);
  const programs = [...new Set(instructions.flatMap(function names(x): string[] { return [x.program ?? x.programId, ...x.inner.flatMap(names)]; }))];
  return {
    failure: failureOf(tx.logs),
    signature: tx.signature ?? null, slot: tx.slot ?? null, blockTime: tx.blockTime ?? null, fee: tx.fee ?? null, computeUnits: tx.computeUnits ?? null,
    ok: tx.err === null || tx.err === undefined, error: tx.err ?? null,
    instructions, events: all, logsTruncated: tx.logs.some((l) => l.startsWith('Log truncated')), story, programs,
  };
}

/** The last "failed" log line, with an Anchor custom error named from the program's interface. */
export function failureOf(logs: string[]): ExplainedTransaction['failure'] {
  for (let i = logs.length - 1; i >= 0; i--) {
    const m = /^Program (\w+) failed: (.+)$/.exec(logs[i]!);
    if (!m) continue;
    const program = programName(m[1]!);
    const hex = /custom program error: 0x([0-9a-f]+)/i.exec(m[2]!);
    const code = hex ? parseInt(hex[1]!, 16) : null;
    const e = code !== null && program ? EXPLORER_IDLS[program]?.errors?.find((x) => x.code === code) : undefined;
    return { programId: m[1]!, program, code, name: e?.name ?? null, message: e?.msg ?? m[2]! };
  }
  return null;
}

// ------------------------------------------------------------------------------ story --

const s = (v: unknown): string => (v === null || v === undefined ? '' : typeof v === 'object' ? JSON.stringify(v) : String(v));
const short = (k: unknown): string => { const t = s(k); return t.length > 12 ? `${t.slice(0, 4)}...${t.slice(-4)}` : t; };
/** Lamports as SOL with up to 9 decimals, trailing zeros trimmed; plain digits otherwise. */
export function solOf(lamports: unknown): string {
  const t = s(lamports);
  if (!/^\d+$/.test(t)) return t;
  const n = BigInt(t);
  const whole = n / 1_000_000_000n;
  const frac = (n % 1_000_000_000n).toString().padStart(9, '0').replace(/0+$/, '');
  return frac ? `${whole}.${frac} SOL` : `${whole} SOL`;
}
const bps = (v: unknown): string => `${(Number(s(v)) / 100).toString()}%`;
/** `direction == 1` is a buy (bordrless_swap swap.rs). */
const side = (v: unknown): string => (s(v) === '1' ? 'buy' : s(v) === '0' ? 'sell' : s(v));

function memoFacts(m: UnitsMemo): [string, string][] {
  const out: [string, string][] = [['Kind', m.kind], ['From', m.from], ['To', m.to === '*' ? 'everyone' : m.to], ['Thread', m.thread]];
  if (m.replyTo) out.push(['Reply to', m.replyTo]);
  if (m.expiresAt) out.push(['Expires', new Date(m.expiresAt * 1000).toISOString()]);
  out.push(['Body', JSON.stringify(m.body)]);
  return out;
}

/** One story line per event the explorer explains; generic for the rest. */
export function storyOf(e: { program: string; name: string; data: Record<string, unknown> }): Omit<StoryLine, 'event'> | null {
  const d = e.data;
  switch (`${e.program}.${e.name}`) {
    case 'token.Transferred': {
      const facts: [string, string][] = [['Mint', s(d.mint)], ['From', s(d.sourceOwner)], ['To', s(d.destinationOwner)], ['Amount', s(d.amount)]];
      const cuts = (d.slotCuts as Record<string, unknown>[] | undefined) ?? [];
      // 01: the slot items that ran on this transfer, in slot order, each with the cut it took.
      for (const c of cuts) facts.push([`Slot ${s(c.slot)} cut`, `${s(c.cut)} by item ${short(c.item)}`]);
      for (const dl of (d.deltas as Record<string, unknown>[] | undefined) ?? []) facts.push(['Delta', `${s(dl.amount)} to ${short(dl.owner)} (holding ${short(dl.holding)})`]);
      return { kind: 'transfer', title: cuts.length ? `Transfer with ${cuts.length} slot cut${cuts.length === 1 ? '' : 's'}` : 'Transfer', facts };
    }
    case 'token.HookDataWritten':
      return { kind: 'transfer', title: 'Holder data written', facts: [['Mint', s(d.mint)], ['Holding', s(d.holding)], ['Owner', s(d.owner)]] };
    case 'token.SlotEquipped':
      return { kind: 'armory', title: `Slot ${s(d.slot)} equipped`, facts: [['Mint', s(d.mint)], ['Old item', s(d.oldItem)], ['New item', s(d.newItem)], ['Program', s(d.program)]] };
    case 'swap.Swapped':
      return { kind: 'swap', title: `Swap: ${side(d.direction)}`, facts: [['Pool', s(d.pool)], ['Trader', s(d.trader)], ['In', s(d.amountIn)], ['Received by the pool', s(d.receivedIn)], ['Out', s(d.amountOut)], ['Delivered', s(d.deliveredOut)], ['Cuts', `${s(d.cutsIn)} in, ${s(d.cutsOut)} out`], ['Burned', `${s(d.burnIn)} in, ${s(d.burnOut)} out`], ['LP fee', s(d.lpFee)], ['Protocol fee', s(d.protocolFee)]] };
    case 'swap.RouteSwapped':
      return { kind: 'swap', title: `Route of ${((d.pools as unknown[]) ?? []).length} hops`, facts: [['Trader', s(d.trader)], ['From mint', s(d.routeInputMint)], ['To mint', s(d.routeOutputMint)], ['In', s(d.amountIn)], ['Out', s(d.amountOut)]] };
    case 'launch.PoolItemCuts': {
      const facts: [string, string][] = [['Mint', s(d.mint)], ['Side', side(d.side)], ['Discount', bps(d.discountBps)], ['Into pool cuts', s(d.poolCutsDelta)], ['Burned', s(d.burned)]];
      for (const p of (d.parts as Record<string, unknown>[] | undefined) ?? []) facts.push([`Slot ${s(p.slot)}`, `item ${short(p.item)}: discount ${bps(p.discountBps)}, cut ${s(p.cut)}, burn ${s(p.burn)}`]);
      return { kind: 'poolItems', title: 'Pool items on this swap', facts };
    }
    case 'launch.LaunchCreated':
      return { kind: 'launch', title: `Launch: ${s(d.name)} (${s(d.symbol)})`, facts: [['Mint', s(d.mint)], ['Creator', s(d.creator)], ['Pool', s(d.pool)], ['Creator fee', bps(d.creatorFeeBps)], ['Virtual SOL', solOf(d.virtualQuote)], ['Graduates at', solOf(d.graduationQuote)]] };
    case 'launch.Graduated':
      return { kind: 'launch', title: 'Graduated', facts: [['Mint', s(d.mint)], ['Cranker', s(d.cranker)], ['Reserve burned', s(d.burned)], ['LP minted', s(d.lpMinted)]] };
    case 'items.RaidMarked':
      return { kind: 'raid', title: 'Raid marked', facts: [['Mint', s(d.mint)], ['Rival', s(d.rival)], ['Trader', s(d.trader)], ['Volume', solOf(d.volume)], ['Points', s(d.points)], ['Loot ticket', d.lootTicket ? 'yes' : 'no']] };
    case 'items.ItemCut':
      return { kind: 'poolItems', title: `Item cut in slot ${s(d.slot)}`, facts: [['Mint', s(d.mint)], ['Item', s(d.item)], ['Module', s(d.module)], ['Side', side(d.side)], ['Amount', s(d.amount)]] };
    case 'items.EquipSettled':
      // 11 section 2: what the settle paid out of the equip vault. Each figure is the event's own field.
      return { kind: 'settle', title: `Settlement of slot ${s(d.slot)}`, facts: [['Mint', s(d.mint)], ['Item', s(d.item)],
        ['Item royalty (token)', s(d.royaltyToken)], ['Item royalty (SOL)', solOf(d.royaltyQuote)],
        ['Module destination (token)', s(d.amountToken)], ['Module destination (SOL)', solOf(d.amountQuote)],
        ['Burned', s(d.burned)], ['Settle bounty (token)', s(d.bountyToken)], ['Settle bounty (SOL)', solOf(d.bountyQuote)]] };
    case 'items.LeaseRentPaid':
      return { kind: 'settle', title: `Lessor rent, slot ${s(d.slot)}`, facts: [['Mint', s(d.mint)], ['Item', s(d.item)], ['Lessor', s(d.lessor)], ['Rent (token)', s(d.rentToken)], ['Rent (SOL)', solOf(d.rentQuote)]] };
    case 'items.ShieldTaken':
      return { kind: 'raid', title: 'Shield taken', facts: [['Mint', s(d.mint)], ['Owner', s(d.owner)], ['Cut', s(d.cut)]] };
    case 'market.ProtocolFee':
      return { kind: 'settle', title: 'Protocol fee', facts: [['Source', s(d.source)], ['Mint', s(d.mint)], ['Amount', s(d.amount)]] };
    case 'market.LicenceBought':
      return { kind: 'market', title: d.renewal ? 'Licence renewed' : 'Licence bought', facts: [['Item', s(d.item)], ['Token', s(d.tokenMint)], ['Payer', s(d.payer)], ['Holder', s(d.holder)], ['Price', solOf(d.price)], ['Protocol fee', solOf(d.protocol)], ['Author share', solOf(d.author)], ['To the holder', solOf(d.toHolder)], ['Ends', new Date(Number(s(d.endsAt)) * 1000).toISOString()]] };
    case 'war.SiegeExecuted':
      return { kind: 'war', title: 'Siege', facts: [['Mint', s(d.mint)], ['Rival', s(d.rivalMint)], ['Spent', solOf(d.spent)], ['Bought', s(d.bought)], ['Cranker bounty', solOf(d.bounty)], ['Captured in total', s(d.capturedTotal)]] };
    case 'war.SiegeWaited':
      return { kind: 'war', title: 'Siege waited', facts: Object.entries(d).map(([k, v]) => [k, s(v)]) };
    case 'war.CounterStrikeExecuted':
      return { kind: 'war', title: 'Counter-strike', facts: [['Mint', s(d.mint)], ['Spent', solOf(d.spent)], ['Burned', s(d.burned)], ['Cranker bounty', solOf(d.bounty)]] };
    case 'war.Razed':
      return { kind: 'war', title: 'Raze', facts: [['Mint', s(d.mint)], ['Rival', s(d.rivalMint)], ['Sold', s(d.sold)], ['Got', solOf(d.got)], ['Captured left', s(d.capturedLeft)]] };
    case 'war.BountyClaimed':
      return { kind: 'war', title: 'Bounty claimed', facts: [['Mint', s(d.mint)], ['Owner', s(d.owner)], ['Points', s(d.points)], ['Paid', solOf(d.paid)]] };
    case 'war.RollRevealed':
      return { kind: 'war', title: 'Loot revealed', facts: [['Mint', s(d.mint)], ['Owner', s(d.owner)], ['Item', s(d.item)], ['Template', s(d.templateId)], ['Season', s(d.season)]] };
    case 'market.Sold':
      return { kind: 'market', title: 'Item sold', facts: [['Item', s(d.item)], ['Seller', s(d.seller)], ['Buyer', s(d.buyer)], ['Price', solOf(d.price)], ['Protocol fee', solOf(d.fee)], ['Resale royalty', solOf(d.resale)]] };
    case 'market.LeaseStarted':
      return { kind: 'market', title: 'Lease started', facts: [['Item', s(d.item)], ['Payer', s(d.payer)], ['Starts', new Date(Number(s(d.startsAt)) * 1000).toISOString()], ['Ends', new Date(Number(s(d.endsAt)) * 1000).toISOString()]] };
    case 'market.Listed':
      return { kind: 'market', title: 'Item listed', facts: [['Item', s(d.item)], ['Seller', s(d.seller)], ['Price', solOf(d.priceLamports)]] };
    default:
      return generic(e);
  }
}

/** 11 section 2 and R32: a settle pays the lessor's rent out of the item royalty, so the royalty
 * the item earned is the event's royalty plus the rent paid in the same settle (settle.rs emits the
 * royalty after the rent). One line per settle that paid rent. */
export function royaltySplits(events: { ordinal: number; program: string; name: string; data: Record<string, unknown> }[]): StoryLine[] {
  const out: StoryLine[] = [];
  for (const e of events) {
    if (e.program !== 'items' || e.name !== 'EquipSettled') continue;
    const r = events.find((x) => x.ordinal > e.ordinal && x.program === 'items' && x.name === 'LeaseRentPaid' && x.data.mint === e.data.mint && x.data.slot === e.data.slot && x.data.item === e.data.item);
    if (!r) continue;
    const sum = (a: unknown, b: unknown) => (BigInt(s(a) || '0') + BigInt(s(b) || '0')).toString();
    out.push({ kind: 'settle', title: `Royalty split, slot ${s(e.data.slot)}`, event: e.ordinal, facts: [
      ['Royalty earned (token)', sum(e.data.royaltyToken, r.data.rentToken)], ['Royalty earned (SOL)', solOf(sum(e.data.royaltyQuote, r.data.rentQuote))],
      ['Lessor rent (token)', s(r.data.rentToken)], ['Lessor rent (SOL)', solOf(r.data.rentQuote)],
      ['Item holder (token)', s(e.data.royaltyToken)], ['Item holder (SOL)', solOf(e.data.royaltyQuote)],
    ] });
  }
  return out;
}

const KIND_OF: Record<string, StoryKind> = { token: 'transfer', swap: 'swap', launch: 'launch', items: 'settle', war: 'war', market: 'market', craft: 'craft', book: 'book', agents: 'agents', social: 'social', armory: 'armory' };
const TITLE_SPLIT = /([a-z])([A-Z])/g;

/** Every other event: its name as words, and its fields as they were decoded. */
function generic(e: { program: string; name: string; data: Record<string, unknown> }): Omit<StoryLine, 'event'> {
  const words = e.name.replace(TITLE_SPLIT, '$1 $2');
  const title = words.charAt(0) + words.slice(1).toLowerCase();
  return { kind: KIND_OF[e.program] ?? 'other', title: `${PROGRAM_TITLES[e.program] ?? e.program}: ${title}`, facts: Object.entries(e.data).map(([k, v]) => [k, s(v)]) };
}

// ------------------------------------------------------------------------------- ring --

/** A pool's price ring (03 3.1) as the explorer shows it: the filled entries oldest first and the
 * time-weighted average over the whole filled window, `(cumulative last - cumulative first) /
 * (ts last - ts first)`, a Q64.64 quote-per-base price as the ring stores it. Null when the window
 * is empty or has no width. */
export function ringSummary(r: { cumulative: bigint; lastPriceQ64: bigint; lastTs: bigint; index: number; filled: number; len: number; spacing: number; entries: { ts: bigint; priceCumulative: bigint; quoteVolume: bigint; swapCount: bigint }[] }) {
  const n = Math.min(r.filled, r.len);
  // Entries oldest first: when the ring has wrapped, the oldest is the one after `index`.
  const order = n < r.len ? r.entries.slice(0, n) : [...r.entries.slice(r.index + 1), ...r.entries.slice(0, r.index + 1)];
  const first = order[0];
  const last = order[order.length - 1];
  let twapQ64: string | null = null;
  let windowSecs: string | null = null;
  if (first && last && last.ts > first.ts) {
    twapQ64 = ((last.priceCumulative - first.priceCumulative) / (last.ts - first.ts)).toString();
    windowSecs = (last.ts - first.ts).toString();
  }
  return {
    filled: n, len: r.len, spacing: r.spacing, lastPriceQ64: r.lastPriceQ64.toString(), lastTs: r.lastTs.toString(), twapQ64, windowSecs,
    entries: order.map((e) => ({ ts: e.ts.toString(), priceCumulative: e.priceCumulative.toString(), quoteVolume: e.quoteVolume.toString(), swapCount: e.swapCount.toString() })),
  };
}

/** A Q64.64 price as a decimal string with `digits` places (exact integer arithmetic). */
export function q64ToDecimal(q: string, digits = 12): string {
  const v = BigInt(q);
  const scale = 10n ** BigInt(digits);
  const scaled = (v * scale) >> 64n;
  const whole = scaled / scale;
  const frac = (scaled % scale).toString().padStart(digits, '0').replace(/0+$/, '');
  return frac ? `${whole}.${frac}` : whole.toString();
}

// ----------------------------------------------------------------------------- search --

export type SearchKind = 'signature' | 'address' | 'program' | 'invalid';
/** What a search string is, before any read: a signature (64 bytes) or an address (32 bytes). */
export function classifyQuery(q: string): { kind: SearchKind; value: string } {
  const t = q.trim();
  if (!/^[1-9A-HJ-NP-Za-km-z]+$/.test(t)) return { kind: 'invalid', value: t };
  let n: number;
  try { n = bs58.decode(t).length; } catch { return { kind: 'invalid', value: t }; }
  if (n === 64) return { kind: 'signature', value: t };
  if (n === 32) return { kind: programName(t) ? 'program' : 'address', value: t };
  return { kind: 'invalid', value: t };
}
