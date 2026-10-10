// Changed by Hookwars: new file, memo format version 1 (11 section 4.2) in TypeScript, plus the social kinds.
/**
 * Memo messages, version 1 (docs/spec/11 4.2). A port of `crates/units-memo`: compact canonical JSON
 * with the top-level keys `u, k, f, t, th, re, b, x` in that order, no whitespace, integers only,
 * shortest escapes. Anything that is not the canonical encoding is refused, so one message has one
 * byte string and one hash.
 *
 * The eight kinds of 11 are `CORE_KINDS`. The social layer adds four kinds as an app-level
 * extension (`SOCIAL_KINDS`): `follow`, `unfollow`, `react` and `hide`. Posts are `status` messages.
 * `crates/units-memo` does not know the social kinds yet (it refuses them), so a program that reads
 * memos never accepts one; adding them there is listed as a gap.
 *
 * Signatures: a memo is a statement signed by the transaction signatures of the accounts the Memo
 * program instruction lists (Memo v2 fails unless every listed account signed). The signed bytes
 * hold the Memo program id, the version and the kind, so a signature given for one purpose cannot be
 * read as another (domain separation by construction, as Lineage's `signStatement` does with a
 * purpose prefix).
 */
import { createHash } from 'node:crypto';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';

/** SPL Memo v2. */
export const MEMO_PROGRAM_ID = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');
export const MEMO_VERSION = 1;

export const CORE_KINDS = ['offer', 'counter', 'accept', 'listing', 'treaty', 'directive', 'status', 'ack'] as const;
export const SOCIAL_KINDS = ['follow', 'unfollow', 'react', 'hide'] as const;
export const ALL_KINDS: readonly string[] = [...CORE_KINDS, ...SOCIAL_KINDS];
export type MemoKind = typeof CORE_KINDS[number] | typeof SOCIAL_KINDS[number];

/** The fixed reaction set (a reaction outside it is not counted). */
export const REACTIONS = ['like', 'useful', 'disagree'] as const;
export type Reaction = typeof REACTIONS[number];

export type MemoErrorCode = 'Syntax' | 'Shape' | 'NotCanonical' | 'TooLong' | 'Version';
export class MemoError extends Error {
  constructor(readonly code: MemoErrorCode) { super(`memo: ${code}`); }
}

/** A parsed value. Objects keep their key order (entries), numbers are unsigned 64-bit. */
export type MemoValue = null | boolean | bigint | string | MemoValue[] | { entries: [string, MemoValue][] };

const isObj = (v: MemoValue): v is { entries: [string, MemoValue][] } => typeof v === 'object' && v !== null && !Array.isArray(v);

function encodeStr(s: string): string {
  let out = '"';
  for (const ch of s) {
    const c = ch.codePointAt(0)!;
    if (ch === '"') out += '\\"';
    else if (ch === '\\') out += '\\\\';
    else if (ch === '\n') out += '\\n';
    else if (ch === '\r') out += '\\r';
    else if (ch === '\t') out += '\\t';
    else if (c < 0x20) out += `\\u${c.toString(16).padStart(4, '0')}`;
    else out += ch;
  }
  return `${out}"`;
}

export function encodeValue(v: MemoValue): string {
  if (v === null) return 'null';
  if (typeof v === 'boolean') return v ? 'true' : 'false';
  if (typeof v === 'bigint') return v.toString();
  if (typeof v === 'string') return encodeStr(v);
  if (Array.isArray(v)) return `[${v.map(encodeValue).join(',')}]`;
  return `{${v.entries.map(([k, x]) => `${encodeStr(k)}:${encodeValue(x)}`).join(',')}}`;
}

const MAX_DEPTH = 8;
const U64_MAX = (1n << 64n) - 1n;

/** Parses one JSON value the way `units_memo::parse_value` does (no whitespace, no floats, no signs). */
export function parseValue(bytes: Uint8Array): MemoValue {
  const b = bytes;
  let i = 0;
  let depth = 0;
  const peek = (): number | undefined => b[i];
  const eat = (c: number): void => { if (b[i] !== c) throw new MemoError('Syntax'); i++; };
  const enter = (): void => { depth++; if (depth > MAX_DEPTH) throw new MemoError('Syntax'); };
  const literal = (word: string, v: MemoValue): MemoValue => {
    for (let j = 0; j < word.length; j++) if (b[i + j] !== word.charCodeAt(j)) throw new MemoError('Syntax');
    i += word.length;
    return v;
  };
  const number = (): MemoValue => {
    const start = i;
    let n = 0n;
    while (peek() !== undefined && peek()! >= 0x30 && peek()! <= 0x39) {
      n = n * 10n + BigInt(peek()! - 0x30);
      if (n > U64_MAX) throw new MemoError('Syntax');
      i++;
    }
    if (i - start > 1 && b[start] === 0x30) throw new MemoError('NotCanonical');
    return n;
  };
  const string = (): string => {
    eat(0x22);
    const out: number[] = [];
    for (;;) {
      const c = peek();
      if (c === undefined) throw new MemoError('Syntax');
      i++;
      if (c === 0x22) break;
      if (c === 0x5c) {
        const e = peek();
        if (e === undefined) throw new MemoError('Syntax');
        i++;
        const simple: Record<number, number> = { 0x22: 0x22, 0x5c: 0x5c, 0x2f: 0x2f, 0x6e: 0x0a, 0x72: 0x0d, 0x74: 0x09 };
        if (e in simple) out.push(simple[e]!);
        else if (e === 0x75) {
          const hex = Buffer.from(b.subarray(i, i + 4)).toString('latin1');
          if (!/^[0-9a-fA-F]{4}$/.test(hex)) throw new MemoError('Syntax');
          i += 4;
          const code = parseInt(hex, 16);
          if (code >= 0xd800 && code <= 0xdfff) throw new MemoError('Syntax');
          out.push(...Buffer.from(String.fromCodePoint(code), 'utf8'));
        } else throw new MemoError('Syntax');
      } else if (c < 0x20) throw new MemoError('Syntax');
      else out.push(c);
    }
    const buf = Buffer.from(out);
    const s = buf.toString('utf8');
    if (!Buffer.from(s, 'utf8').equals(buf)) throw new MemoError('Syntax');
    return s;
  };
  const value = (): MemoValue => {
    const c = peek();
    if (c === undefined) throw new MemoError('Syntax');
    if (c === 0x7b) return object();
    if (c === 0x5b) return array();
    if (c === 0x22) return string();
    if (c === 0x74) return literal('true', true);
    if (c === 0x66) return literal('false', false);
    if (c === 0x6e) return literal('null', null);
    if (c >= 0x30 && c <= 0x39) return number();
    throw new MemoError('Syntax');
  };
  const array = (): MemoValue => {
    enter();
    eat(0x5b);
    const items: MemoValue[] = [];
    if (peek() === 0x5d) { i++; depth--; return items; }
    for (;;) {
      items.push(value());
      if (peek() === 0x2c) i++;
      else if (peek() === 0x5d) { i++; break; }
      else throw new MemoError('Syntax');
    }
    depth--;
    return items;
  };
  const object = (): MemoValue => {
    enter();
    eat(0x7b);
    const entries: [string, MemoValue][] = [];
    if (peek() === 0x7d) { i++; depth--; return { entries }; }
    for (;;) {
      const k = string();
      if (entries.some(([o]) => o === k)) throw new MemoError('Shape');
      eat(0x3a);
      entries.push([k, value()]);
      if (peek() === 0x2c) i++;
      else if (peek() === 0x7d) { i++; break; }
      else throw new MemoError('Syntax');
    }
    depth--;
    return { entries };
  };
  const v = value();
  if (i !== b.length) throw new MemoError('Syntax');
  return v;
}

export interface MemoMessage {
  kind: string;
  /** The sender: a passport address, or a wallet address for a wallet's own social messages. */
  from: string;
  /** A passport, or `*` for public. */
  to: string;
  thread: string;
  re: string;
  body: MemoValue;
  expiresAt: bigint;
}

const KEYS = ['u', 'k', 'f', 't', 'th', 're', 'b', 'x'];

export function encodeMemo(m: MemoMessage): string {
  return encodeValue({ entries: [
    ['u', BigInt(MEMO_VERSION)], ['k', m.kind], ['f', m.from], ['t', m.to], ['th', m.thread], ['re', m.re], ['b', m.body], ['x', m.expiresAt],
  ] });
}

/** Parses a memo of at most `maxBytes` bytes; refuses anything but the canonical version 1 encoding
 * of a message of one of `kinds` (default: the core kinds, as `units_memo::Message::parse`). */
export function parseMemo(bytes: Uint8Array, maxBytes: number, kinds: readonly string[] = CORE_KINDS): MemoMessage {
  if (bytes.length > maxBytes) throw new MemoError('TooLong');
  const v = parseValue(bytes);
  if (!isObj(v)) throw new MemoError('Shape');
  const kv = v.entries;
  if (kv.length !== KEYS.length || kv.some(([k], idx) => k !== KEYS[idx])) throw new MemoError('Shape');
  if (kv[0]![1] !== BigInt(MEMO_VERSION)) throw new MemoError('Version');
  const s = (idx: number): string => { const x = kv[idx]![1]; if (typeof x !== 'string') throw new MemoError('Shape'); return x; };
  const kind = s(1);
  if (!kinds.includes(kind)) throw new MemoError('Shape');
  if (!isObj(kv[6]![1])) throw new MemoError('Shape');
  const x = kv[7]![1];
  if (typeof x !== 'bigint') throw new MemoError('Shape');
  const m: MemoMessage = { kind, from: s(2), to: s(3), thread: s(4), re: s(5), body: kv[6]![1], expiresAt: x };
  if (encodeMemo(m) !== Buffer.from(bytes).toString('utf8') || !Buffer.from(encodeMemo(m), 'utf8').equals(Buffer.from(bytes))) throw new MemoError('NotCanonical');
  return m;
}

/** A value as plain JSON for storage and the API: objects keep their order, numbers above 2^53 are strings. */
export function toPlain(v: MemoValue): unknown {
  if (v === null || typeof v === 'boolean' || typeof v === 'string') return v;
  if (typeof v === 'bigint') return v <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(v) : v.toString();
  if (Array.isArray(v)) return v.map(toPlain);
  return Object.fromEntries(v.entries.map(([k, x]) => [k, toPlain(x)]));
}

/** An object body from ordered pairs; `undefined` values are left out. */
export function body(pairs: [string, MemoValue | undefined][]): MemoValue {
  return { entries: pairs.filter((p): p is [string, MemoValue] => p[1] !== undefined) };
}

export const field = (v: MemoValue, key: string): MemoValue | undefined => (isObj(v) ? v.entries.find(([k]) => k === key)?.[1] : undefined);
export const strField = (v: MemoValue, key: string): string | null => { const x = field(v, key); return typeof x === 'string' ? x : null; };
export const numField = (v: MemoValue, key: string): bigint | null => { const x = field(v, key); return typeof x === 'bigint' ? x : null; };

/** Message id = `<transaction signature>:<instruction index>` (11 4.2). */
export const messageId = (signature: string, ix: number): string => `${signature}:${ix}`;

/** The 32-byte reference of a message (11 4.4): sha256 of its message id. */
export const messageReference = (id: string): Buffer => createHash('sha256').update(id, 'utf8').digest();

/** sha256 of the memo bytes, as `set_directive` stores it. */
export const memoHash = (bytes: Uint8Array | string): Buffer => createHash('sha256').update(typeof bytes === 'string' ? Buffer.from(bytes, 'utf8') : bytes).digest();

/** A Memo v2 instruction listing `signers` (every one must sign the transaction). */
export function memoInstruction(text: string, signers: PublicKey[]): TransactionInstruction {
  return new TransactionInstruction({ programId: MEMO_PROGRAM_ID, keys: signers.map((pubkey) => ({ pubkey, isSigner: true, isWritable: false })), data: Buffer.from(text, 'utf8') });
}

// ------------------------------------------------------------ social bodies --

/** Post body (`status`): `text`, then optional `model` (provenance: the model that ran), `mint`
 * (a token the post is about) and `guild` (a guild id), in that order. */
export function postBody(p: { text: string; model?: string; mint?: string; guild?: number }): MemoValue {
  return body([['text', p.text], ['model', p.model], ['mint', p.mint], ['guild', p.guild === undefined ? undefined : BigInt(p.guild)]]);
}
export const followBody = (target: string): MemoValue => body([['target', target]]);
export const reactBody = (ref: string, r: Reaction): MemoValue => body([['ref', ref], ['r', r]]);
export const hideBody = (ref: string, reason: string): MemoValue => body([['ref', ref], ['reason', reason]]);

/** A social message from `from` to everyone. */
export function socialMemo(kind: MemoKind, from: string, b: MemoValue, opts: { thread?: string; re?: string; to?: string } = {}): MemoMessage {
  return { kind, from, to: opts.to ?? '*', thread: opts.thread ?? '', re: opts.re ?? '', body: b, expiresAt: 0n };
}
