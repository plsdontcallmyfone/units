import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import {
  directiveMessage, encodeMessage, fromJson, MEMO_PROGRAM_ID, memoInstruction, MemoError, messageId, messageRef, obj, parseMessage, parseValue, readDirectiveBody, toJson, type Message,
} from './memo.ts';

// The same sample and expected bytes as crates/units-memo's `round_trips_canonically`.
const sample = (): Message => ({
  kind: 'offer', from: 'Pass1', to: '*', thread: '', re: '',
  body: obj([['item', 'It"em\n'], ['price', 42n], ['tags', [true, null]]]),
  expiresAt: 1_800_000_000n,
});
const RUST_BYTES = '{"u":1,"k":"offer","f":"Pass1","t":"*","th":"","re":"","b":{"item":"It\\"em\\n","price":42,"tags":[true,null]},"x":1800000000}';

const code = (f: () => unknown): string => { try { f(); return 'ok'; } catch (e) { return e instanceof MemoError ? e.code : 'other'; } };

describe('memo v1', () => {
  it('encodes exactly as the Rust crate and round trips', () => {
    const s = encodeMessage(sample());
    expect(s).toBe(RUST_BYTES);
    expect(parseMessage(Buffer.from(s), 1_000)).toEqual(sample());
  });

  it('refuses non-canonical and wrong shapes with the Rust error codes', () => {
    const s = encodeMessage(sample());
    expect(code(() => parseMessage(Buffer.from(s), 10))).toBe('TooLong');
    expect(code(() => parseMessage(Buffer.from(s.replace(':', ': ')), 1_000))).not.toBe('ok');
    expect(code(() => parseMessage(Buffer.from(s.replace('"u":1,"k":"offer"', '"k":"offer","u":1')), 1_000))).toBe('Shape');
    expect(code(() => parseMessage(Buffer.from(s.replace('"u":1', '"u":2')), 1_000))).toBe('Version');
    expect(code(() => parseMessage(Buffer.from(s.replace('offer', 'shout')), 1_000))).toBe('Shape');
    expect(code(() => parseMessage(Buffer.from(s.replace('"price":42', '"price":042')), 1_000))).not.toBe('ok');
    expect(code(() => parseMessage(Buffer.from(s.replace('Pass1', 'Pass\\u0031')), 1_000))).toBe('NotCanonical');
    expect(code(() => parseMessage(Buffer.from('not json'), 1_000))).toBe('Syntax');
    expect(code(() => parseMessage(Buffer.from('{}'), 1_000))).toBe('Shape');
    expect(code(() => parseValue(Buffer.from('{"a":1,"a":2}')))).toBe('Shape');
    expect(code(() => parseValue(Buffer.from('['.repeat(20) + ']'.repeat(20))))).toBe('Syntax');
    expect(code(() => parseValue(Buffer.from('18446744073709551616')))).toBe('Syntax');
    expect(code(() => parseValue(Buffer.from('1.5')))).toBe('Syntax');
    expect(code(() => parseValue(Buffer.from('-1')))).toBe('Syntax');
    expect(code(() => parseMessage(Buffer.from(s.replace('"b":{', '"b":[').replace(',"x"', ']') ), 1_000))).not.toBe('ok');
  });

  it('escapes control characters as \\u00XX and keeps other text as is', () => {
    const m = { ...sample(), body: obj([['t', 'a\u0001bé\u{1F600}']]) };
    const s = encodeMessage(m);
    expect(s).toContain('"a\\u0001bé\u{1F600}"');
    expect(parseMessage(Buffer.from(s), 1_000)).toEqual(m);
  });

  it('builds and reads directive bodies in their only key order', () => {
    const m = directiveMessage('PassX', 3n, 'https://r', 'ab12', 'cd34');
    const parsed = parseMessage(Buffer.from(encodeMessage(m)), 1_000);
    expect(parsed.kind).toBe('directive');
    expect(readDirectiveBody(parsed.body)).toEqual({ passport: 'PassX', seq: 3n, rulesUri: 'https://r', h: 'ab12', c: 'cd34' });
    expect(code(() => readDirectiveBody(obj([['passport', 'P'], ['seq', 1n], ['rules_uri', ''], ['h', '']])))).toBe('Shape');
    expect(code(() => readDirectiveBody(obj([['seq', 1n]])))).toBe('Shape');
    expect(code(() => readDirectiveBody(obj([['seq', 1n], ['passport', 'P'], ['rules_uri', ''], ['h', '']])))).toBe('Shape');
  });

  it('threads by message id and references by its sha256', () => {
    const id = messageId('5'.repeat(88), 2);
    expect(id).toBe(`${'5'.repeat(88)}:2`);
    expect(messageRef(id).equals(createHash('sha256').update(id).digest())).toBe(true);
  });

  it('makes a memo instruction signed by the listed keys', () => {
    const k = new PublicKey(new Uint8Array(32).fill(7));
    const ix = memoInstruction(sample(), [k]);
    expect(ix.programId.equals(MEMO_PROGRAM_ID)).toBe(true);
    expect(ix.keys).toEqual([{ pubkey: k, isSigner: true, isWritable: false }]);
    expect(ix.data.toString()).toBe(RUST_BYTES);
  });

  it('converts plain JSON and refuses floats and negatives', () => {
    expect(toJson(fromJson({ a: [1, 'x', true, null], b: { c: 2 } }))).toEqual({ a: [1, 'x', true, null], b: { c: 2 } });
    expect(code(() => fromJson({ a: 1.5 }))).toBe('Shape');
    expect(code(() => fromJson({ a: -1 }))).toBe('Shape');
  });
});
