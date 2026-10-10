// Changed by Hookwars: new file (explorer v2). The explorer reads against a mocked chain: a real v0
// message read back into the decoder's shape, search, an address page with and without the
// indexer, and a program page.
import { describe, expect, it } from 'vitest';
import { Keypair, PublicKey, TransactionInstruction, TransactionMessage, type VersionedTransactionResponse } from '@solana/web3.js';
import bs58 from 'bs58';
import { hookwars } from '@hookwars/sdk';
import { route, txSourceOf } from './explorer.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const MEMO = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');
const memo = '{"u":1,"k":"status","f":"P1","t":"*","th":"t","re":"","b":{},"x":0}';

function response(): { sig: string; r: VersionedTransactionResponse } {
  const payer = Keypair.generate().publicKey;
  const msg = new TransactionMessage({
    payerKey: payer, recentBlockhash: bs58.encode(Buffer.alloc(32, 1)),
    instructions: [new TransactionInstruction({ programId: MEMO, keys: [{ pubkey: payer, isSigner: true, isWritable: true }], data: Buffer.from(memo) })],
  }).compileToV0Message();
  const sig = bs58.encode(Buffer.alloc(64, 7));
  const r = {
    slot: 42, blockTime: 1_700_000_000, version: 0,
    transaction: { message: msg, signatures: [sig] },
    meta: { err: null, fee: 5000, computeUnitsConsumed: 300, innerInstructions: [], logMessages: [`Program ${MEMO.toBase58()} invoke [1]`, `Program ${MEMO.toBase58()} success`], loadedAddresses: { writable: [], readonly: [] }, preBalances: [], postBalances: [] },
  } as unknown as VersionedTransactionResponse;
  return { sig, r };
}

type Acc = { owner: PublicKey; data: Buffer; executable?: boolean };
function chain(accounts: Record<string, Acc>, tx?: VersionedTransactionResponse) {
  const info = (k: PublicKey) => { const a = accounts[k.toBase58()]; return a ? { data: a.data, owner: a.owner, lamports: 11, executable: a.executable ?? false, rentEpoch: 0 } : null; };
  return {
    getAccountInfo: async (k: PublicKey) => info(k),
    getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info),
    getSignaturesForAddress: async () => [{ signature: bs58.encode(Buffer.alloc(64, 9)), slot: 40, blockTime: 1, err: null, memo: null }],
    getTransaction: async () => tx ?? null,
  } as never;
}

describe('explorer reads', () => {
  it('reads a v0 transaction into the decoder and decodes its memo', async () => {
    const { sig, r } = response();
    const src = txSourceOf(sig, r);
    expect(src.signer?.[0]).toBe(true);
    expect(src.writable?.[0]).toBe(true);
    const out = await route(chain({}, r), null, `/v1/explorer/tx/${sig}`, new URLSearchParams());
    expect(out?.status).toBe(200);
    const t = out!.body as hookwars.explore.ExplainedTransaction;
    expect(t.instructions[0]!.memo?.units?.kind).toBe('status');
    expect(t.story[0]!.title).toBe('Memo: status');
  });

  it('answers 404 for an unknown transaction and 400 for a bad one', async () => {
    const sig = bs58.encode(Buffer.alloc(64, 2));
    expect((await route(chain({}), null, `/v1/explorer/tx/${sig}`, new URLSearchParams()))?.status).toBe(404);
    await expect(route(chain({}), null, `/v1/explorer/tx/${bs58.encode(Buffer.alloc(32, 2))}`, new URLSearchParams())).rejects.toMatchObject({ status: 400 });
  });

  it('searches signatures, programs, templates and addresses', async () => {
    const q = (s: string) => route(chain({}), null, '/v1/explorer/search', new URLSearchParams({ q: s })).then((r) => r!.body as { kind: string; href: string | null });
    expect((await q(bs58.encode(Buffer.alloc(64, 2)))).kind).toBe('transaction');
    expect((await q(hookwars.TOKEN_ID.toBase58())).href).toBe(`/program/${hookwars.TOKEN_ID.toBase58()}`);
    expect((await q('template 7')).href).toBe(`/address/${hookwars.templateAddress(7).toBase58()}`);
    expect((await q(K(3).toBase58())).kind).toBe('empty');
    expect((await q('not base58 0OIl')).kind).toBe('invalid');
  });

  it('decodes an account by its owner and adds indexed history only when the indexer answers', async () => {
    const c = hookwars.coderOf('armory');
    const zero = c.decodeAccount<Record<string, unknown>>('Template', Buffer.concat([c.accountDisc('Template'), Buffer.alloc(4000)]));
    const data = c.encodeAccount('Template', { ...zero, id: 7 });
    const key = hookwars.templateAddress(7);
    const conn = chain({ [key.toBase58()]: { owner: hookwars.ARMORY_ID, data } });
    const bare = (await route(conn, null, `/v1/explorer/address/${key.toBase58()}`, new URLSearchParams()))!.body as Record<string, unknown>;
    expect(bare.kind).toBe('template');
    expect((bare.decoded as { type: string }).type).toBe('Template');
    expect(bare.indexed).toBeNull();
    const db = { query: async (sql: string) => ({ rows: sql.includes('from events') ? [{ signature: 's', ordinal: 0, slot: '5', program: 'war', name: 'RollRevealed', data: { mint: 'm', templateId: 7 } }] : [{ template_id: 7, name: 'Treaty' }] }) } as never;
    const full = (await route(conn, db, `/v1/explorer/address/${key.toBase58()}`, new URLSearchParams()))!.body as { indexed: { events: { story: { title: string } }[]; template: unknown } };
    expect(full.indexed.events[0]!.story.title).toBe('Loot revealed');
    expect(full.indexed.template).toEqual({ template_id: 7, name: 'Treaty' });
  });

  it('lists a program page with its interface and recent signatures', async () => {
    const id = hookwars.ITEMS_ID.toBase58();
    const body = (await route(chain({ [id]: { owner: K(2), data: Buffer.alloc(0), executable: true } }), null, `/v1/explorer/program/${id}`, new URLSearchParams()))!.body as { name: string; deployed: boolean; events: string[]; recent: unknown[] };
    expect(body.name).toBe('items');
    expect(body.deployed).toBe(true);
    expect(body.events).toContain('RaidMarked');
    expect(body.recent).toHaveLength(1);
  });
});
