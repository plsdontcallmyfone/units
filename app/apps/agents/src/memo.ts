// units memo format, version 1 (docs/spec/11-hook-economy.md section 4.2): a TypeScript port of
// crates/units-memo. One message has exactly one encoding; `parseMessage` refuses anything that
// would not re-encode to the same bytes, so a memo's sha256 is a function of its content.
import { createHash } from 'node:crypto';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';

export const MEMO_VERSION = 1;
export const MEMO_PROGRAM_ID = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');

export const KINDS = ['offer', 'counter', 'accept', 'listing', 'treaty', 'directive', 'status', 'ack'] as const;
export type Kind = (typeof KINDS)[number];

export type MemoErrorCode = 'Syntax' | 'Shape' | 'NotCanonical' | 'TooLong' | 'Version';
export class MemoError extends Error {
  readonly code: MemoErrorCode;
  constructor(code: MemoErrorCode) { super(`memo: ${code}`); this.code = code; }
}

/** A JSON value of the subset version 1 uses: no floats, no negative numbers, object key order kept. */
export type Value = null | boolean | bigint | string | Value[] | Obj;
export interface Obj { readonly obj: [string, Value][] }

export const obj = (entries: [string, Value][]): Obj => ({ obj: entries });
export function isObj(v: Value): v is Obj { return typeof v === 'object' && v !== null && !Array.isArray(v); }
export function get(v: Value, key: string): Value | undefined { return isObj(v) ? v.obj.find(([k]) => k === key)?.[1] : undefined; }

const U64_MAX = (1n << 64n) - 1n;

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
  return out + '"';
}

/** Canonical encoding: no whitespace, keys in stored order, minimal escapes. */
export function encodeValue(v: Value): string {
  if (v === null) return 'null';
  if (typeof v === 'boolean') return v ? 'true' : 'false';
  if (typeof v === 'bigint') {
    if (v < 0n || v > U64_MAX) throw new MemoError('Shape');
    return v.toString();
  }
  if (typeof v === 'string') return encodeStr(v);
  if (Array.isArray(v)) return `[${v.map(encodeValue).join(',')}]`;
  return `{${v.obj.map(([k, x]) => `${encodeStr(k)}:${encodeValue(x)}`).join(',')}}`;
}

const MAX_DEPTH = 8;

class Parser {
  i = 0;
  depth = 0;
  readonly b: Uint8Array;
  constructor(b: Uint8Array) { this.b = b; }
  peek(): number | undefined { return this.b[this.i]; }
  eat(c: number): void { if (this.peek() !== c) throw new MemoError('Syntax'); this.i++; }
  value(): Value {
    const c = this.peek();
    if (c === undefined) throw new MemoError('Syntax');
    if (c === 0x7b) return this.object();
    if (c === 0x5b) return this.array();
    if (c === 0x22) return this.string();
    if (c === 0x74) return this.literal('true', true);
    if (c === 0x66) return this.literal('false', false);
    if (c === 0x6e) return this.literal('null', null);
    if (c >= 0x30 && c <= 0x39) return this.number();
    throw new MemoError('Syntax');
  }
  literal(word: string, v: Value): Value {
    for (let k = 0; k < word.length; k++) if (this.b[this.i + k] !== word.charCodeAt(k)) throw new MemoError('Syntax');
    this.i += word.length;
    return v;
  }
  number(): Value {
    const start = this.i;
    let n = 0n;
    for (let c = this.peek(); c !== undefined && c >= 0x30 && c <= 0x39; c = this.peek()) {
      n = n * 10n + BigInt(c - 0x30);
      if (n > U64_MAX) throw new MemoError('Syntax');
      this.i++;
    }
    if (this.i - start > 1 && this.b[start] === 0x30) throw new MemoError('NotCanonical');
    return n;
  }
  string(): string {
    this.eat(0x22);
    const out: number[] = [];
    for (;;) {
      const c = this.peek();
      if (c === undefined) throw new MemoError('Syntax');
      this.i++;
      if (c === 0x22) break;
      if (c === 0x5c) {
        const e = this.peek();
        if (e === undefined) throw new MemoError('Syntax');
        this.i++;
        const simple: Record<number, number> = { 0x22: 0x22, 0x5c: 0x5c, 0x2f: 0x2f, 0x6e: 0x0a, 0x72: 0x0d, 0x74: 0x09 };
        if (simple[e] !== undefined) out.push(simple[e]!);
        else if (e === 0x75) {
          const hex = Buffer.from(this.b.subarray(this.i, this.i + 4)).toString('latin1');
          if (!/^[0-9a-fA-F]{4}$/.test(hex)) throw new MemoError('Syntax');
          this.i += 4;
          const code = parseInt(hex, 16);
          if (code >= 0xd800 && code <= 0xdfff) throw new MemoError('Syntax');
          out.push(...Buffer.from(String.fromCodePoint(code), 'utf8'));
        } else throw new MemoError('Syntax');
      } else if (c < 0x20) throw new MemoError('Syntax');
      else out.push(c);
    }
    const bytes = Buffer.from(out);
    const s = bytes.toString('utf8');
    if (!Buffer.from(s, 'utf8').equals(bytes)) throw new MemoError('Syntax');
    return s;
  }
  enter(): void { if (++this.depth > MAX_DEPTH) throw new MemoError('Syntax'); }
  array(): Value {
    this.enter();
    this.eat(0x5b);
    const items: Value[] = [];
    if (this.peek() === 0x5d) { this.i++; this.depth--; return items; }
    for (;;) {
      items.push(this.value());
      const c = this.peek();
      if (c === 0x2c) this.i++;
      else if (c === 0x5d) { this.i++; break; } else throw new MemoError('Syntax');
    }
    this.depth--;
    return items;
  }
  object(): Value {
    this.enter();
    this.eat(0x7b);
    const kv: [string, Value][] = [];
    if (this.peek() === 0x7d) { this.i++; this.depth--; return obj(kv); }
    for (;;) {
      const k = this.string();
      if (kv.some(([o]) => o === k)) throw new MemoError('Shape');
      this.eat(0x3a);
      kv.push([k, this.value()]);
      const c = this.peek();
      if (c === 0x2c) this.i++;
      else if (c === 0x7d) { this.i++; break; } else throw new MemoError('Syntax');
    }
    this.depth--;
    return obj(kv);
  }
}

/** Parses any supported JSON value, refusing trailing bytes. */
export function parseValue(bytes: Uint8Array): Value {
  const p = new Parser(bytes);
  const v = p.value();
  if (p.i !== bytes.length) throw new MemoError('Syntax');
  return v;
}

/** A version 1 message. */
export interface Message {
  kind: Kind;
  /** `f`: the sending passport (base58). */
  from: string;
  /** `t`: the recipient passport or `*`. */
  to: string;
  /** `th`: thread id (the first message's id; empty starts a thread). */
  thread: string;
  /** `re`: the message this replies to. */
  re: string;
  /** `b`: the kind's body (an object). */
  body: Obj;
  /** `x`: unix time after which the message is stale (0 = never). */
  expiresAt: bigint;
}

const KEYS = ['u', 'k', 'f', 't', 'th', 're', 'b', 'x'];

export function encodeMessage(m: Message): string {
  return encodeValue(obj([
    ['u', BigInt(MEMO_VERSION)], ['k', m.kind], ['f', m.from], ['t', m.to], ['th', m.thread], ['re', m.re], ['b', m.body], ['x', m.expiresAt],
  ]));
}

export function messageBytes(m: Message): Buffer { return Buffer.from(encodeMessage(m), 'utf8'); }

/** Parses a memo of at most `maxBytes` bytes; refuses anything not the canonical encoding. */
export function parseMessage(bytes: Uint8Array, maxBytes: number): Message {
  if (bytes.length > maxBytes) throw new MemoError('TooLong');
  const v = parseValue(bytes);
  if (!isObj(v)) throw new MemoError('Shape');
  const kv = v.obj;
  if (kv.length !== KEYS.length || kv.some(([k], i) => k !== KEYS[i])) throw new MemoError('Shape');
  if (kv[0]![1] !== BigInt(MEMO_VERSION)) throw new MemoError('Version');
  const s = (i: number): string => { const x = kv[i]![1]; if (typeof x !== 'string') throw new MemoError('Shape'); return x; };
  const kind = s(1);
  if (!(KINDS as readonly string[]).includes(kind)) throw new MemoError('Shape');
  const body = kv[6]![1];
  if (!isObj(body)) throw new MemoError('Shape');
  const x = kv[7]![1];
  if (typeof x !== 'bigint') throw new MemoError('Shape');
  const m: Message = { kind: kind as Kind, from: s(2), to: s(3), thread: s(4), re: s(5), body, expiresAt: x };
  if (!Buffer.from(encodeMessage(m), 'utf8').equals(Buffer.from(bytes))) throw new MemoError('NotCanonical');
  return m;
}

/** The body of a `directive` (11 section 4.3), exact keys in order. */
/** Pass 5 (review 3 L-6): `c` is the hex sha256 of the Borsh-encoded constraints `set_directive` writes. */
export interface DirectiveBody { passport: string; seq: bigint; rulesUri: string; h: string; c: string }

export function directiveBodyValue(b: DirectiveBody): Obj {
  return obj([['passport', b.passport], ['seq', b.seq], ['rules_uri', b.rulesUri], ['h', b.h], ['c', b.c]]);
}

export function readDirectiveBody(v: Value): DirectiveBody {
  if (!isObj(v)) throw new MemoError('Shape');
  const keys = ['passport', 'seq', 'rules_uri', 'h', 'c'];
  if (v.obj.length !== keys.length || v.obj.some(([k], i) => k !== keys[i])) throw new MemoError('Shape');
  const [p, seq, uri, h, c] = v.obj.map(([, x]) => x);
  if (typeof p !== 'string' || typeof seq !== 'bigint' || typeof uri !== 'string' || typeof h !== 'string' || typeof c !== 'string') throw new MemoError('Shape');
  return { passport: p, seq, rulesUri: uri, h, c };
}

/** A directive memo for `passport` at `seq`, from the passport itself to everyone. */
export function directiveMessage(passport: string, seq: bigint, rulesUri: string, rulesHashHex: string, constraintsHashHex: string): Message {
  return { kind: 'directive', from: passport, to: '*', thread: '', re: '', body: directiveBodyValue({ passport, seq, rulesUri, h: rulesHashHex, c: constraintsHashHex }), expiresAt: 0n };
}

/** Message id = `<transaction signature>:<instruction index>` (assigned by the indexer). */
export function messageId(signature: string, index: number): string { return `${signature}:${index}`; }

/** The 32-byte reference settling instructions and `commit` carry: sha256 of the message id. */
export function messageRef(id: string): Buffer { return createHash('sha256').update(id, 'utf8').digest(); }

/** A memo instruction. The memo program requires every listed account to sign. */
export function memoInstruction(m: Message, signers: PublicKey[]): TransactionInstruction {
  return new TransactionInstruction({ programId: MEMO_PROGRAM_ID, keys: signers.map((pubkey) => ({ pubkey, isSigner: true, isWritable: false })), data: messageBytes(m) });
}

/** Plain JSON (numbers as safe integers or strings) into a memo value; refuses floats and negatives. */
export function fromJson(x: unknown, depth = 0): Value {
  if (depth > MAX_DEPTH) throw new MemoError('Syntax');
  if (x === null) return null;
  if (typeof x === 'boolean' || typeof x === 'string') return x;
  if (typeof x === 'bigint') return x;
  if (typeof x === 'number') {
    if (!Number.isSafeInteger(x) || x < 0) throw new MemoError('Shape');
    return BigInt(x);
  }
  if (Array.isArray(x)) return x.map((v) => fromJson(v, depth + 1));
  if (typeof x === 'object') return obj(Object.entries(x as Record<string, unknown>).map(([k, v]) => [k, fromJson(v, depth + 1)]));
  throw new MemoError('Shape');
}

/** A memo value as plain JSON (u64 numbers as strings when above the safe integer range). */
export function toJson(v: Value): unknown {
  if (typeof v === 'bigint') return v <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(v) : v.toString();
  if (Array.isArray(v)) return v.map(toJson);
  if (v !== null && typeof v === 'object') return Object.fromEntries(v.obj.map(([k, x]) => [k, toJson(x)]));
  return v;
}
