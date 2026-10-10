import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import {
  AGENTS_ID, decodeDirective, decodeMemoConfig, directiveAddress, effective, encodeDirective, MEMO_CONFIG_ACCOUNT_DISC, parseRules, SET_DIRECTIVE_DISC, setDirectiveInstructions, verifyDirectiveMemo, type DirectiveAccount,
} from './directive.ts';
import { tick } from './loop.ts';
import { MEMO_PROGRAM_ID, parseMessage } from './memo.ts';
import { replay } from './models/stub.ts';
import { kit, OPEN_CONSTRAINTS, OPERATOR, testConfig } from './testkit.ts';

const P = new PublicKey(new Uint8Array(32).fill(9));
const sha = (s: string | Buffer) => createHash('sha256').update(s).digest('hex');

describe('Directive accounts', () => {
  it('decodes what it encodes (Borsh layout of hookwars_agents::Directive)', () => {
    const d: DirectiveAccount = { passport: P, seq: 3, memoHash: Buffer.alloc(32, 5), constraints: { ...OPEN_CONSTRAINTS(), allowedAccessModes: 6, maxLicencePrice: 77n, frozen: true }, postedAt: 1_800_000_000n, supersededBy: 4, bump: 254 };
    expect(decodeDirective(encodeDirective(d))).toEqual(d);
    expect(decodeDirective(encodeDirective({ ...d, supersededBy: null }))).toEqual({ ...d, supersededBy: null });
    expect(() => decodeDirective(Buffer.alloc(200))).toThrow();
  });

  it('decodes the memo config', () => {
    const b = Buffer.alloc(8 + 1 + 2 + 8 + 1 + 1 + 8);
    MEMO_CONFIG_ACCOUNT_DISC.copy(b); b[8] = 255; b.writeUInt16LE(600, 9); b.writeBigUInt64LE(5_000n, 11); b[19] = 1;
    expect(decodeMemoConfig(b)).toEqual({ memoMaxBytes: 600, postageLamports: 5_000n, minProof: 1 });
  });

  it('builds the operator transaction: the memo signed by the operator, then set_directive', () => {
    const passport = hookwars.passportAddress(OPERATOR.publicKey, 0);
    const [memo, set] = setDirectiveInstructions(OPERATOR.publicKey, passport, 0, OPEN_CONSTRAINTS(), 'https://r', 'a'.repeat(64));
    expect(memo!.programId.equals(MEMO_PROGRAM_ID)).toBe(true);
    expect(memo!.keys).toEqual([{ pubkey: OPERATOR.publicKey, isSigner: true, isWritable: false }]);
    const m = parseMessage(memo!.data, 600);
    expect(m.kind).toBe('directive');
    expect(m.from).toBe(passport.toBase58());
    expect(set!.programId.equals(AGENTS_ID)).toBe(true);
    expect(set!.data.subarray(0, 8).equals(SET_DIRECTIVE_DISC)).toBe(true);
    expect(set!.data.readUInt32LE(8)).toBe(0);
    expect(set!.keys[0]).toEqual({ pubkey: OPERATOR.publicKey, isSigner: true, isWritable: true });
    expect(set!.keys[5]!.pubkey.equals(directiveAddress(passport, 0))).toBe(true);
    expect(set!.keys[6]!.pubkey.equals(AGENTS_ID)).toBe(true); // no previous at seq 0
    expect(set!.keys[7]!.pubkey.equals(SYSVAR_INSTRUCTIONS_PUBKEY)).toBe(true);
    const [, set1] = setDirectiveInstructions(OPERATOR.publicKey, passport, 1, OPEN_CONSTRAINTS(), 'https://r', 'a'.repeat(64));
    expect(set1!.keys[6]).toEqual({ pubkey: directiveAddress(passport, 0), isSigner: false, isWritable: true });
  });
});

describe('directive memo and rules', () => {
  const k = kit();
  const d = (k.chain.directives.get(`${k.passport.toBase58()}:0`))!;
  const memo = k.chain.directiveMemos.get(`${k.passport.toBase58()}:0`)!;

  it('accepts the memo the account hashes, and nothing else', () => {
    expect(verifyDirectiveMemo(memo, d, 600).seq).toBe(0n);
    const other = Buffer.from(memo.toString().replace('"seq":0', '"seq":1'));
    expect(() => verifyDirectiveMemo(other, d, 600)).toThrow(/does not hash/);
    expect(() => verifyDirectiveMemo(other, { ...d, memoHash: createHash('sha256').update(other).digest() }, 600)).toThrow(/another passport or sequence/);
    expect(() => verifyDirectiveMemo(memo, d, 10)).toThrow(/not memo v1/);
  });

  it('parses rules only when they match the hash, and refuses unknown fields', () => {
    const doc = '{"v":1,"roles":["reporter"],"trade":{"maxTradeLamports":"5"},"templates":[42]}';
    expect(parseRules(Buffer.from(doc), sha(doc)).roles).toEqual(['reporter']);
    expect(() => parseRules(Buffer.from(doc), sha(doc + ' '))).toThrow(/does not match/);
    const bad = '{"v":1,"spendMore":true}';
    expect(() => parseRules(Buffer.from(bad), sha(bad))).toThrow(/unknown field/);
    const v2 = '{"v":2}';
    expect(() => parseRules(Buffer.from(v2), sha(v2))).toThrow(/version 1/);
  });

  it('only narrows the config: roles, caps, universe, templates; frozen and halt stop everything', () => {
    const cfg = testConfig();
    const limits = { roles: cfg.roles, caps: cfg.trade!.caps, universe: null, templates: cfg.author!.templates };
    const e = effective(limits, d, { v: 1, roles: ['reporter', 'trader', 'notarole'], trade: { maxTradeLamports: 5n }, templates: [42, 99], universe: ['M1'] });
    expect(e.roles).toEqual(['reporter', 'trader']);
    expect(e.caps!.maxTradeLamports).toBe(5n);
    expect(e.templates).toEqual([42]);
    expect(e.universe).toEqual(['M1']);
    expect(effective(limits, { ...d, constraints: { ...d.constraints, frozen: true } }, { v: 1 }).halted).toMatch(/freezes/);
    expect(effective(limits, d, { v: 1, halt: true }).halted).toMatch(/halt/);
  });
});

describe('directive enforcement in the loop', () => {
  const createItem = { actions: [{ type: 'create_item', templateId: 42, params: [1], royaltyBps: 100 }] };

  it('does nothing before the operator posts a directive', async () => {
    const k = kit(replay([createItem]));
    k.chain.directives.clear();
    const r = await tick(k.rt);
    expect(r.halted).toMatch(/no directive/);
    expect(k.model.calls).toHaveLength(0);
    expect(k.sender.submitted).toHaveLength(0);
  });

  it('halts when the memo or the rules document does not match its hash', async () => {
    const k = kit(replay([createItem]));
    k.chain.directiveMemos.set(`${k.passport.toBase58()}:0`, Buffer.from('{"tampered":true}'));
    expect((await tick(k.rt)).halted).toMatch(/does not hash/);
    const k2 = kit(replay([createItem]));
    k2.chain.rules.set('https://rules.example/scout/0.json', Buffer.from('{"v":1,"notes":"something else"}'));
    expect((await tick(k2.rt)).halted).toMatch(/does not match the hash/);
    expect(k2.sender.submitted).toHaveLength(0);
  });

  it('follows the latest directive in the chain of sequences', async () => {
    const k = kit(replay([createItem, createItem]));
    expect((await tick(k.rt)).directiveSeq).toBe(0);
    k.chain.postDirective(k.passport, 1, OPEN_CONSTRAINTS(), 'https://rules.example/scout/1.json', '{"v":1,"roles":["reporter"]}');
    const r = await tick(k.rt);
    expect(r.directiveSeq).toBe(1);
    expect(r.results[0]).toMatchObject({ type: 'create_item', ok: false });
    expect(r.results[0]!.refusals![0]).toMatch(/author role is not allowed/);
  });

  it('refuses a template the rules leave out, and trades over the tightened cap', async () => {
    const trade = { type: 'trade', side: 'buy', mint: P.toBase58(), amountIn: '6', minOut: '60', reason: 'testing the cap' };
    const k = kit(replay([{ actions: [{ ...createItem.actions[0], templateId: 43 }, trade] }]), {}, '{"v":1,"templates":[42],"trade":{"maxTradeLamports":"5"}}');
    k.chain.mints.set(P.toBase58(), { creator: 'X', itemAuthors: [] });
    k.router.quotes.set(P.toBase58(), 60n);
    const r = await tick(k.rt);
    expect(r.results[0]!.refusals![0]).toMatch(/template 43 is not allowed/);
    expect(r.results[1]!.refusals!.join()).toMatch(/TradeSize/);
    expect(k.sender.submitted).toHaveLength(0);
  });

  it('stops every action when the directive freezes the agent', async () => {
    const k = kit(replay([createItem]));
    k.chain.postDirective(k.passport, 1, { ...OPEN_CONSTRAINTS(), frozen: true }, 'https://rules.example/scout/1.json', '{"v":1}');
    const r = await tick(k.rt);
    expect(r.halted).toMatch(/freezes/);
    expect(k.model.calls).toHaveLength(0);
  });
});
