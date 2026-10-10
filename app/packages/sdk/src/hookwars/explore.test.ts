// Changed by Hookwars: new file (explorer v2), synthetic vectors for the explorer's decoder built
// with the IDL encoders: execution order of self-CPI and log events, the story lines, the royalty
// split, memos, compute budget, failures and search classification.
import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { coderOf } from './from-idl.ts';
import * as explore from './explore.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const EVENT_TAG = Buffer.from([0xe4, 0x45, 0xa5, 0x2e, 0x51, 0xcb, 0x9a, 0x1d]);
const TOKEN = explore.programIdOf('token')!;
const ITEMS = explore.programIdOf('items')!;
const MEMO = 'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr';
const CB = 'ComputeBudget111111111111111111111111111111';

function event(program: string, name: string, over: Record<string, unknown>): Buffer {
  const c = coderOf(program);
  const disc = c.idl.events!.find((e) => e.name === name)!.discriminator;
  const zero = c.decodeEvent(Buffer.concat([Buffer.from(disc), Buffer.alloc(4000)]))!.data;
  return c.encodeEvent(name, { ...zero, ...over });
}

const mint = K(5), item = K(6), lessor = K(7);
const keys = [K(1).toBase58(), TOKEN, ITEMS, MEMO, CB, mint.toBase58(), K(8).toBase58(), K(9).toBase58()];
const memo = '{"u":1,"k":"offer","f":"P1","t":"*","th":"t-1","re":"","b":{"price":"5"},"x":0}';
const transferDisc = Buffer.from(coderOf('token').instruction('transfer').discriminator);
const amount = Buffer.alloc(8); amount.writeBigUInt64LE(100_000n);

const settle = event('items', 'EquipSettled', { mint, slot: 0, item, royaltyToken: 900n, royaltyQuote: 0n, amountToken: 0n, amountQuote: 0n, burned: 19_000n, bountyToken: 100n, bountyQuote: 0n });
const rent = event('items', 'LeaseRentPaid', { mint, slot: 0, item, lessor, rentToken: 100n, rentQuote: 0n });
const transferred = event('token', 'Transferred', {
  mint, sourceOwner: K(1), destinationOwner: K(9), amount: 100_000n,
  slotCuts: [{ slot: 0, item, cut: 20_000n }],
});

const src: explore.TxSource = {
  signature: bs58.encode(Buffer.alloc(64, 3)), slot: 10, blockTime: 1_700_000_000, err: null, fee: 5000, computeUnits: 1234,
  accountKeys: keys, signer: keys.map((_, i) => i === 0), writable: keys.map((_, i) => i === 0 || i >= 5),
  instructions: [
    { programIdIndex: 4, accounts: [], data: bs58.encode(Buffer.from([2, 0xc0, 0x5c, 0x15, 0x00])) },
    { programIdIndex: 3, accounts: [0], data: bs58.encode(Buffer.from(memo)) },
    { programIdIndex: 1, accounts: [0, 6, 7, 5], data: bs58.encode(Buffer.concat([transferDisc, amount])) },
  ],
  inner: [{ index: 2, instructions: [
    { programIdIndex: 2, accounts: [5], data: bs58.encode(Buffer.from([1, 2, 3, 4, 5, 6, 7, 8])), stackHeight: 2 },
    { programIdIndex: 1, accounts: [], data: bs58.encode(Buffer.concat([EVENT_TAG, transferred])), stackHeight: 2 },
  ] }],
  logs: [
    `Program ${CB} invoke [1]`, `Program ${CB} success`,
    `Program ${MEMO} invoke [1]`, `Program ${MEMO} success`,
    `Program ${TOKEN} invoke [1]`,
    `Program ${ITEMS} invoke [2]`,
    `Program data: ${settle.toString('base64')}`,
    `Program data: ${rent.toString('base64')}`,
    `Program ${ITEMS} success`,
    `Program ${TOKEN} invoke [2]`, `Program ${TOKEN} success`,
    `Program ${TOKEN} success`,
  ],
};

describe('explainTransaction', () => {
  const t = explore.explainTransaction(src);
  it('decodes every top-level instruction, nests the inner calls and hides the event self-CPI', () => {
    expect(t.instructions.map((x) => [x.program, x.name])).toEqual([['computeBudget', 'setComputeUnitLimit'], ['memo', 'memo'], ['token', 'transfer']]);
    expect(t.instructions[0]!.args).toEqual({ units: 1_400_000 });
    expect(t.instructions[2]!.args).toEqual({ amount: '100000' });
    expect(t.instructions[2]!.accounts.map((a) => a.name).slice(0, 4)).toEqual(['authority', 'source', 'destination', 'mint']);
    expect(t.instructions[2]!.inner.map((x) => x.program)).toEqual(['items']);
    expect(t.instructions[2]!.inner[0]!.undecoded).toBe('unknown instruction discriminator');
  });
  it('orders events as they ran: the items log events before the token self-CPI', () => {
    expect(t.events.map((e) => `${e.program}.${e.name}:${e.via}`)).toEqual(['items.EquipSettled:log', 'items.LeaseRentPaid:log', 'token.Transferred:cpi']);
    expect(t.events.every((e) => e.instruction === 2)).toBe(true);
  });
  it('tells the story from decoded fields only', () => {
    const tr = t.story.find((s) => s.kind === 'transfer')!;
    expect(tr.title).toBe('Transfer with 1 slot cut');
    expect(tr.facts).toContainEqual(['Slot 0 cut', `20000 by item ${item.toBase58().slice(0, 4)}...${item.toBase58().slice(-4)}`]);
    const split = t.story.find((s) => s.title.startsWith('Royalty split'))!;
    expect(split.facts).toContainEqual(['Royalty earned (token)', '1000']);
    expect(split.facts).toContainEqual(['Lessor rent (token)', '100']);
    expect(split.facts).toContainEqual(['Item holder (token)', '900']);
    const m = t.story.find((s) => s.kind === 'memo')!;
    expect(m.title).toBe('Memo: offer');
    expect(m.facts).toContainEqual(['To', 'everyone']);
    expect(t.ok).toBe(true);
    expect(t.failure).toBeNull();
  });
  it('names a failure from the program interface', () => {
    const f = explore.failureOf([`Program ${ITEMS} invoke [1]`, `Program ${ITEMS} failed: custom program error: 0x1770`]);
    expect(f).toMatchObject({ program: 'items', code: 6000, name: 'BadParams' });
  });
});

describe('memos, search and the ring', () => {
  it('accepts only canonical units memos', () => {
    expect(explore.parseUnitsMemo(memo)).toMatchObject({ kind: 'offer', to: '*', thread: 't-1' });
    expect(explore.parseUnitsMemo(memo.replace('"k":"offer"', '"k":"gift"'))).toBeNull();
    expect(explore.parseUnitsMemo('{"k":"offer","u":1}')).toBeNull();
    expect(explore.parseUnitsMemo('hello')).toBeNull();
  });
  it('classifies queries by their decoded length', () => {
    expect(explore.classifyQuery(src.signature!).kind).toBe('signature');
    expect(explore.classifyQuery(TOKEN).kind).toBe('program');
    expect(explore.classifyQuery(K(9).toBase58()).kind).toBe('address');
    expect(explore.classifyQuery('0OIl').kind).toBe('invalid');
  });
  it('averages the ring over its filled window, oldest first after a wrap', () => {
    const Q = 1n << 64n;
    const e = (ts: number, cum: bigint) => ({ ts: BigInt(ts), priceCumulative: cum, quoteVolume: 0n, swapCount: 0n });
    const r = explore.ringSummary({ cumulative: 0n, lastPriceQ64: 2n * Q, lastTs: 300n, index: 0, filled: 3, len: 3, spacing: 60,
      entries: [e(300, 400n * Q), e(100, 0n), e(200, 200n * Q)] });
    expect(r.entries.map((x) => x.ts)).toEqual(['100', '200', '300']);
    expect(r.windowSecs).toBe('200');
    expect(explore.q64ToDecimal(r.twapQ64!)).toBe('2');
  });
  it('writes SOL amounts exactly', () => {
    expect(explore.solOf('1500000000')).toBe('1.5 SOL');
    expect(explore.solOf('1')).toBe('0.000000001 SOL');
  });
});
