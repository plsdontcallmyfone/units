// Changed by Hookwars: new file, memo v1 vectors (the same strings as crates/units-memo's tests).
import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { Keypair } from '@solana/web3.js';
import {
  ALL_KINDS, CORE_KINDS, MEMO_PROGRAM_ID, MemoError, body, encodeMemo, followBody, memoHash, memoInstruction, messageId, messageReference,
  parseMemo, parseValue, postBody, reactBody, hideBody, socialMemo, toPlain, SOCIAL_KINDS, type MemoMessage, type Reaction,
} from './memo.ts';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const sample = (): MemoMessage => ({
  kind: 'offer', from: 'Pass1', to: '*', thread: '', re: '',
  body: body([['item', 'It"em\n'], ['price', 42n], ['tags', [true, null]]]),
  expiresAt: 1_800_000_000n,
});
const code = (f: () => unknown): string => { try { f(); return 'ok'; } catch (e) { return e instanceof MemoError ? e.code : 'other'; } };
const b = (s: string) => Buffer.from(s, 'utf8');

describe('memo v1 (the units-memo vectors)', () => {
  it('round-trips canonically to the exact Rust string', () => {
    const s = encodeMemo(sample());
    expect(s).toBe('{"u":1,"k":"offer","f":"Pass1","t":"*","th":"","re":"","b":{"item":"It\\"em\\n","price":42,"tags":[true,null]},"x":1800000000}');
    expect(parseMemo(b(s), 1000)).toEqual(sample());
  });

  it('refuses what units-memo refuses, with the same error', () => {
    const s = encodeMemo(sample());
    expect(code(() => parseMemo(b(s), 10))).toBe('TooLong');
    expect(code(() => parseMemo(b(s.replace(':', ': ')), 1000))).not.toBe('ok');
    expect(code(() => parseMemo(b(s.replace('"u":1,"k":"offer"', '"k":"offer","u":1')), 1000))).toBe('Shape');
    expect(code(() => parseMemo(b(s.replace('"u":1', '"u":2')), 1000))).toBe('Version');
    expect(code(() => parseMemo(b(s.replace('offer', 'shout')), 1000))).toBe('Shape');
    expect(code(() => parseMemo(b(s.replace('"price":42', '"price":042')), 1000))).not.toBe('ok');
    expect(code(() => parseMemo(b(s.replace('Pass1', 'Pass\\u0031')), 1000))).toBe('NotCanonical');
    expect(code(() => parseMemo(b('not json'), 1000))).toBe('Syntax');
    expect(code(() => parseMemo(b('{}'), 1000))).toBe('Shape');
    expect(code(() => parseValue(b('{"a":1,"a":2}')))).toBe('Shape');
    expect(code(() => parseValue(b('['.repeat(20) + ']'.repeat(20))))).toBe('Syntax');
  });

  it('keeps the eight kinds strict and accepts the social kinds only when asked', () => {
    expect([...CORE_KINDS]).toEqual(['offer', 'counter', 'accept', 'listing', 'treaty', 'directive', 'status', 'ack']);
    const f = encodeMemo(socialMemo('follow', 'W', followBody('So11111111111111111111111111111111111111112')));
    expect(f).toBe('{"u":1,"k":"follow","f":"W","t":"*","th":"","re":"","b":{"target":"So11111111111111111111111111111111111111112"},"x":0}');
    expect(code(() => parseMemo(b(f), 1000))).toBe('Shape');
    expect(parseMemo(b(f), 1000, ALL_KINDS).kind).toBe('follow');
  });

  it('writes post and reaction bodies in their only order', () => {
    expect(encodeMemo(socialMemo('status', 'W', postBody({ text: 'hi', model: 'm1', guild: 3 })))).toBe('{"u":1,"k":"status","f":"W","t":"*","th":"","re":"","b":{"text":"hi","model":"m1","guild":3},"x":0}');
    expect(encodeMemo(socialMemo('react', 'W', reactBody('sig:0', 'like')))).toContain('"b":{"ref":"sig:0","r":"like"}');
    expect(toPlain(parseMemo(b(encodeMemo(socialMemo('status', 'W', postBody({ text: 'é "q"' })))), 1000).body)).toEqual({ text: 'é "q"' });
  });

  it('escapes control characters the canonical way and refuses floats and signs', () => {
    expect(encodeMemo(socialMemo('status', 'W', postBody({ text: '\u0001\t' })))).toContain('"text":"\\u0001\\t"');
    expect(code(() => parseValue(b('1.5')))).toBe('Syntax');
    expect(code(() => parseValue(b('-1')))).toBe('Syntax');
    expect(code(() => parseValue(b('18446744073709551616')))).toBe('Syntax');
    expect(parseValue(b('18446744073709551615'))).toBe(18446744073709551615n);
  });

  it('message ids, references and the memo instruction', () => {
    const id = messageId('5sig', 2);
    expect(id).toBe('5sig:2');
    expect(messageReference(id).toString('hex')).toBe(createHash('sha256').update('5sig:2').digest('hex'));
    expect(messageReference(id)).toHaveLength(32);
    expect(memoHash('abc').toString('hex')).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
    const k = Keypair.generate().publicKey;
    const ix = memoInstruction('x', [k]);
    expect(ix.programId.equals(MEMO_PROGRAM_ID)).toBe(true);
    expect(ix.keys).toEqual([{ pubkey: k, isSigner: true, isWritable: false }]);
  });
});

describe('the shared social vectors (crates/units-memo/vectors/social.json, pass 4a)', () => {
  const file = fileURLToPath(new URL('../../../../../crates/units-memo/vectors/social.json', import.meta.url));
  const cases = (JSON.parse(readFileSync(file, 'utf8')) as { cases: { kind: string; from: string; args: string[]; memo: string }[] }).cases;
  it('encodes every case byte for byte as the Rust crate does, and parses it back', () => {
    expect(cases.length).toBeGreaterThan(0);
    for (const c of cases) {
      const b2 = c.kind === 'react' ? reactBody(c.args[0]!, c.args[1] as Reaction) : c.kind === 'hide' ? hideBody(c.args[0]!, c.args[1]!) : followBody(c.args[0]!);
      expect(encodeMemo(socialMemo(c.kind as never, c.from, b2)), c.kind).toBe(c.memo);
      const m = parseMemo(Buffer.from(c.memo, 'utf8'), 4096, [...CORE_KINDS, ...SOCIAL_KINDS]);
      expect(m.kind).toBe(c.kind);
      expect(code(() => parseMemo(Buffer.from(c.memo, 'utf8'), 4096))).not.toBe('ok');
    }
  });
});
