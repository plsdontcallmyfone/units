// Operator directives (docs/spec/11-hook-economy.md section 4.3, R41). A directive is a memo signed
// by the operator plus a `Directive` account binding its sha256; the memo names an off-chain rules
// document by URI and hash. The runtime follows the latest directive and only ever tightens what the
// agent config allows: the chain constraints (spend limits, targets, frozen) hold regardless.
import { createHash } from 'node:crypto';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { directiveMessage, memoInstruction, MemoError, parseMessage, readDirectiveBody, type DirectiveBody } from './memo.ts';
import { tighten, type TradeCaps } from './policy.ts';

export const AGENTS_ID = hookwars.AGENTS_ID;

// The accounts and instructions come from the generated agents IDL (`@hookwars/sdk`); the
// discriminators are re-exported for the tests and the mock chain.
export const DIRECTIVE_ACCOUNT_DISC = hookwars.directiveCodec.disc;
export const MEMO_CONFIG_ACCOUNT_DISC = hookwars.memoConfigCodec.disc;
const ixDisc = (name: string): Buffer => Buffer.from(hookwars.coderOf('agents').instruction(name).discriminator);
export const SET_DIRECTIVE_DISC = ixDisc('set_directive');
export const COMMIT_DISC = ixDisc('commit');
export const POST_DISC = ixDisc('post');

export interface DirectiveConstraints {
  maxSpendPerAction: bigint;
  maxSpendPerDay: bigint;
  allowedTargets: PublicKey[];
  allowedAccessModes: number;
  maxLicencePrice: bigint;
  frozen: boolean;
}

export interface DirectiveAccount {
  passport: PublicKey;
  seq: number;
  memoHash: Buffer;
  constraints: DirectiveConstraints;
  postedAt: bigint;
  supersededBy: number | null;
  bump: number;
}

export interface MemoParams { memoMaxBytes: number; postageLamports: bigint; minProof: number }

export const directiveAddress = hookwars.directiveAddress;
export const memoConfigAddress = hookwars.memoConfigAddress;
export const commitmentAddress = hookwars.commitmentAddress;

export function decodeDirective(data: Buffer): DirectiveAccount {
  if (!data.subarray(0, 8).equals(DIRECTIVE_ACCOUNT_DISC)) throw new Error('not a Directive account');
  return hookwars.directiveCodec.decode(data) as unknown as DirectiveAccount;
}

export function encodeDirective(d: DirectiveAccount): Buffer {
  return hookwars.directiveCodec.encode(d as unknown as Parameters<typeof hookwars.directiveCodec.encode>[0]);
}

export function decodeMemoConfig(data: Buffer): MemoParams {
  if (!data.subarray(0, 8).equals(MEMO_CONFIG_ACCOUNT_DISC)) throw new Error('not a MemoConfig account');
  return hookwars.memoConfigCodec.decode(data).params as MemoParams;
}

/** The operator's two instructions: the directive memo (signed by the operator) and `set_directive`. */
export function setDirectiveInstructions(operator: PublicKey, passport: PublicKey, seq: number, constraints: DirectiveConstraints, rulesUri: string, rulesHashHex: string): TransactionInstruction[] {
  const memo = memoInstruction(directiveMessage(passport.toBase58(), BigInt(seq), rulesUri, rulesHashHex), [operator]);
  return [memo, hookwars.agentsSetDirective(operator, passport, seq, constraints)];
}

/** `commit(reference, hash)`: a binding record of an accepted offer (moves no money). */
export function commitInstruction(agent: PublicKey, passport: PublicKey, reference: Buffer, hash: Buffer): TransactionInstruction {
  return hookwars.agentsCommit(agent, passport, reference, hash);
}

/** `post(reference)`: pays the postage for a message. */
export function postInstruction(agent: PublicKey, passport: PublicKey, feeCollector: PublicKey, reference: Buffer): TransactionInstruction {
  return hookwars.agentsPost(agent, passport, feeCollector, reference);
}

/** The operator's rules document at `rules_uri`, version 1. Every field only narrows the agent config. */
export interface Rules {
  v: 1;
  roles?: string[];
  trade?: Partial<TradeCaps>;
  /** Tokens the trader may touch. */
  universe?: string[];
  /** Template ids the author may create items from. */
  templates?: number[];
  /** Free text the model reads (public: it is in the rules document). */
  notes?: string;
  /** Stop every discretionary action. */
  halt?: boolean;
  /** Clears a drawdown halt when this directive takes effect. */
  clearHalt?: boolean;
}

export class DirectiveError extends Error {}

/** Checks the memo bytes found in the directive's transaction against the account. */
export function verifyDirectiveMemo(memo: Uint8Array, d: DirectiveAccount, maxBytes: number): DirectiveBody {
  if (!createHash('sha256').update(memo).digest().equals(d.memoHash)) throw new DirectiveError('the memo does not hash to the Directive account');
  let m;
  try { m = parseMessage(memo, maxBytes); } catch (e) { throw new DirectiveError(`the directive memo is not memo v1 (${e instanceof MemoError ? e.code : 'error'})`); }
  if (m.kind !== 'directive') throw new DirectiveError('the memo is not a directive');
  const b = readDirectiveBody(m.body);
  if (b.passport !== d.passport.toBase58() || m.from !== b.passport || b.seq !== BigInt(d.seq)) throw new DirectiveError('the memo names another passport or sequence');
  if (!/^[0-9a-f]{64}$/.test(b.h)) throw new DirectiveError('the rules hash is not 64 hex characters');
  return b;
}

const toBig = (x: unknown, what: string): bigint => {
  if (typeof x === 'string' && /^\d+$/.test(x)) return BigInt(x);
  if (typeof x === 'number' && Number.isSafeInteger(x) && x >= 0) return BigInt(x);
  throw new DirectiveError(`rules: ${what} must be a non-negative integer`);
};
const toNum = (x: unknown, what: string, max: number): number => {
  if (typeof x === 'number' && Number.isSafeInteger(x) && x >= 0 && x <= max) return x;
  throw new DirectiveError(`rules: ${what} must be an integer from 0 to ${max}`);
};

/** Parses the rules document after checking its sha256 against the memo's `h`. */
export function parseRules(bytes: Uint8Array, hHex: string): Rules {
  if (createHash('sha256').update(bytes).digest('hex') !== hHex) throw new DirectiveError('the rules document does not match the hash in the directive');
  let j: Record<string, unknown>;
  try { j = JSON.parse(Buffer.from(bytes).toString('utf8')); } catch { throw new DirectiveError('the rules document is not JSON'); }
  if (!j || typeof j !== 'object' || j.v !== 1) throw new DirectiveError('the rules document is not version 1');
  const known = new Set(['v', 'roles', 'trade', 'universe', 'templates', 'notes', 'halt', 'clearHalt']);
  for (const k of Object.keys(j)) if (!known.has(k)) throw new DirectiveError(`rules: unknown field "${k}"`);
  const r: Rules = { v: 1 };
  const strs = (x: unknown, what: string): string[] => { if (!Array.isArray(x) || !x.every((s) => typeof s === 'string')) throw new DirectiveError(`rules: ${what} must be a list of strings`); return x as string[]; };
  if (j.roles !== undefined) r.roles = strs(j.roles, 'roles');
  if (j.universe !== undefined) r.universe = strs(j.universe, 'universe');
  if (j.templates !== undefined) { if (!Array.isArray(j.templates)) throw new DirectiveError('rules: templates must be a list'); r.templates = j.templates.map((t) => toNum(t, 'a template id', 0xffff)); }
  if (j.notes !== undefined) { if (typeof j.notes !== 'string' || j.notes.length > 4_000) throw new DirectiveError('rules: notes must be text under 4,000 characters'); r.notes = j.notes; }
  if (j.halt !== undefined) r.halt = j.halt === true;
  if (j.clearHalt !== undefined) r.clearHalt = j.clearHalt === true;
  if (j.trade !== undefined) {
    const t = j.trade as Record<string, unknown>;
    if (!t || typeof t !== 'object') throw new DirectiveError('rules: trade must be an object');
    const out: Partial<TradeCaps> = {};
    if (t.maxPositionLamports !== undefined) out.maxPositionLamports = toBig(t.maxPositionLamports, 'maxPositionLamports');
    if (t.maxTradeLamports !== undefined) out.maxTradeLamports = toBig(t.maxTradeLamports, 'maxTradeLamports');
    if (t.dailyLossHaltLamports !== undefined) out.dailyLossHaltLamports = toBig(t.dailyLossHaltLamports, 'dailyLossHaltLamports');
    if (t.drawdownHaltBps !== undefined) out.drawdownHaltBps = toNum(t.drawdownHaltBps, 'drawdownHaltBps', 10_000);
    if (t.maxSlippageBps !== undefined) out.maxSlippageBps = toNum(t.maxSlippageBps, 'maxSlippageBps', 10_000);
    if (t.minHoldSecs !== undefined) out.minHoldSecs = toNum(t.minHoldSecs, 'minHoldSecs', 365 * 86_400);
    r.trade = out;
  }
  return r;
}

export const ROLES = ['author', 'market', 'diplomat', 'reporter', 'cranker', 'trader'] as const;
export type Role = (typeof ROLES)[number];

/** What the agent may do this tick: the agent config narrowed by the chain constraints and the rules. */
export interface Effective {
  seq: number;
  roles: Role[];
  caps: TradeCaps | null;
  universe: string[] | null;
  templates: number[] | null;
  notes: string;
  halted: string | null;
  clearHalt: boolean;
  constraints: DirectiveConstraints;
}

export interface ConfigLimits { roles: Role[]; caps: TradeCaps | null; universe: string[] | null; templates: number[] | null }

const narrow = <T>(a: T[] | null, b: T[] | undefined): T[] | null => (b === undefined ? a : a === null ? [...b] : a.filter((x) => b.includes(x)));

export function effective(cfg: ConfigLimits, d: DirectiveAccount, rules: Rules): Effective {
  const roles = rules.roles ? cfg.roles.filter((r) => rules.roles!.includes(r)) : [...cfg.roles];
  const halted = d.constraints.frozen ? 'the directive freezes the agent' : rules.halt ? 'the rules halt the agent' : null;
  return {
    seq: d.seq,
    roles,
    caps: cfg.caps && rules.trade ? tighten(cfg.caps, rules.trade) : cfg.caps,
    universe: narrow(cfg.universe, rules.universe),
    templates: narrow(cfg.templates, rules.templates),
    notes: rules.notes ?? '',
    halted,
    clearHalt: rules.clearHalt === true,
    constraints: d.constraints,
  };
}
