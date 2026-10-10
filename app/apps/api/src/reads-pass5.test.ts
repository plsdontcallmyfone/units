/**
 * App pass 5 reads against a mocked chain: an item's access (mode, policy, approvals naming it and
 * no other), what waits on a timelock (inactive proposals left out), Hook Lab submissions, and
 * coalitions trimmed to their member count. Nothing estimated: a missing item reads as null.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type GetProgramAccountsFilter } from '@solana/web3.js';
import bs58 from 'bs58';
import { hookwars } from '@hookwars/sdk';
import { access, coalitions, governanceQueue, templateSubmissions } from './reads-expansion.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });

type Acc = { program: PublicKey; key: PublicKey; data: Buffer };
function chain(list: Acc[]) {
  const byKey = new Map(list.map((a) => [a.key.toBase58(), a]));
  const info = (k: PublicKey) => { const a = byKey.get(k.toBase58()); return a ? { data: a.data, owner: a.program, lamports: 7, executable: false, rentEpoch: 0 } : null; };
  return {
    getAccountInfo: async (k: PublicKey) => info(k),
    getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info),
    getProgramAccounts: async (program: PublicKey, opts: { filters: GetProgramAccountsFilter[] }) => list
      .filter((a) => a.program.equals(program))
      .filter((a) => opts.filters.every((f) => 'memcmp' in f && Buffer.from(bs58.decode(f.memcmp.bytes)).equals(a.data.subarray(f.memcmp.offset, f.memcmp.offset + bs58.decode(f.memcmp.bytes).length))))
      .map((a) => ({ pubkey: a.key, account: { data: a.data, owner: a.program, lamports: 7, executable: false, rentEpoch: 0 } })),
  } as never;
}

const itemMint = K(10), item = hookwars.itemAddress(itemMint), other = hookwars.itemAddress(K(11));

describe('app pass 5 reads', () => {
  const conn = chain([
    { program: hookwars.ARMORY_ID, key: item, data: enc('armory', 'Item', { itemMint, templateId: 3, author: K(1), accessMode: 2, exclusive: true }) },
    { program: hookwars.ARMORY_ID, key: hookwars.accessPolicyAddress(item), data: enc('armory', 'AccessPolicy', { item, mode: 2, exclusive: true, licenceTerms: { priceLamports: 5n, termSecs: 60, per: 0, maxLive: 2 }, holderAtSet: K(1) }) },
    { program: hookwars.ARMORY_ID, key: K(20), data: enc('armory', 'Approval', { item, tokenMint: K(30), approvedBy: K(1), approvedAt: 9n }) },
    { program: hookwars.ARMORY_ID, key: K(21), data: enc('armory', 'Approval', { item: other, tokenMint: K(31), approvedBy: K(1) }) },
    { program: hookwars.ARMORY_ID, key: K(22), data: enc('armory', 'QueuedAction', { actionHash: Array(32).fill(1), admin: K(2), readyAt: 100n }) },
    { program: hookwars.ARMORY_ID, key: K(23), data: enc('armory', 'TemplateSubmission', { program: K(40), submitter: K(3), bond: 7n, codeHash: Array(32).fill(2), uriHash: Array(32).fill(3) }) },
    { program: hookwars.BOOK_ID, key: K(24), data: enc('book', 'PendingMarketTerms', { market: K(50), tickLamports: 10n, minSize: 2n, readyAt: 50n, active: true }) },
    { program: hookwars.BOOK_ID, key: K(25), data: enc('book', 'PendingMarketTerms', { market: K(51), tickLamports: 10n, minSize: 2n, readyAt: 50n, active: false }) },
    { program: hookwars.WAR_ID, key: K(26), data: enc('war', 'Coalition', { id: 4, count: 2, members: [K(60), K(61), ...Array(6).fill(PublicKey.default)].slice(0, (zeroOf('war', 'Coalition').members as unknown[]).length), endsAt: 500n }) },
  ]);

  it('access: mode, policy, only the approvals that name the item', async () => {
    const a = await access(conn, itemMint.toBase58()) as { mode: number; exclusive: boolean; policy: { licenceTerms: { maxLive: number } }; approvals: { tokenMint: string }[]; licenceOffer: unknown };
    expect(a).toMatchObject({ mode: 2, exclusive: true, licenceOffer: null });
    expect(a.policy.licenceTerms.maxLive).toBe(2);
    expect(a.approvals.map((x) => x.tokenMint)).toEqual([K(30).toBase58()]);
    expect(await access(conn, K(99).toBase58())).toBeNull();
  });

  it('governance queue: queued admin actions and active market terms only', async () => {
    const g = await governanceQueue(conn) as { armoryAdmin: unknown; queued: { readyAt: string }[]; marketTerms: { market: string }[] };
    expect(g.armoryAdmin).toBeNull();
    expect(g.queued.map((x) => x.readyAt)).toEqual(['100']);
    expect(g.marketTerms.map((x) => x.market)).toEqual([K(50).toBase58()]);
  });

  it('template submissions and coalitions', async () => {
    expect((await templateSubmissions(conn) as { bond: string }[]).map((x) => x.bond)).toEqual(['7']);
    const c = await coalitions(conn) as { coalitions: { id: number; members: string[] }[]; bossPools: unknown[] };
    expect(c.coalitions[0]).toMatchObject({ id: 4, members: [K(60).toBase58(), K(61).toBase58()] });
    expect(c.bossPools).toEqual([]);
  });
});
