/**
 * App pass 5: the refresh tail of an equip change on a slot launch's Pool or Relation slot names,
 * after the four fixed accounts, the item registry of every slot that forwards pool callbacks once
 * the change lands (the launchpad's `append_pool_items` wants exactly those, in slot order). Lease
 * end, enforce_access and execute build it with the incoming item.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type AccountInfo } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { refreshTailOf } from './economy-prepares.ts';
import { PREPARES } from './prepares.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });

const mint = K(2), pool = K(9), owner = K(3);
const itemX = K(40), itemY = K(41), itemZ = K(42);
const zeroSlot = (zeroOf('token', 'Mint').slots as Record<string, unknown>[])[0]!;
const slot = (kind: number, item: PublicKey | null, poolFlags: number) => ({ ...zeroSlot, kind, item: item ?? PublicKey.default, program: item ? hookwars.ITEMS_ID : PublicKey.default, poolFlags });
const launchData = Buffer.alloc(200); pool.toBuffer().copy(launchData, 74);

function world(slots: Record<string, unknown>[], extra: [PublicKey, Buffer][] = []) {
  const accounts = new Map<string, Buffer>([
    [S(mint), enc('token', 'Mint', { slotCount: slots.length, slots: [...slots, ...Array(4 - slots.length).fill(zeroSlot)] })],
    [S(hookwars.launchAddr(mint)), launchData],
    [S(hookwars.templateAddress(1)), enc('armory', 'Template', { id: 1, program: hookwars.ITEMS_ID, name: 't1' })],
    [S(itemY), enc('armory', 'Item', { itemMint: K(51), templateId: 1, level: 1, manifest: { ...(zeroOf('armory', 'Item').manifest as object), poolFlags: 2 } })],
    [S(itemZ), enc('armory', 'Item', { itemMint: K(52), templateId: 1, level: 1 })],
    ...extra.map(([k, d]) => [S(k), d] as [string, Buffer]),
  ]);
  const info = (k: PublicKey): AccountInfo<Buffer> | null => { const data = accounts.get(S(k)); return data ? { data, owner: hookwars.ARMORY_ID, lamports: 1, executable: false, rentEpoch: 0 } : null; };
  return { getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info) } as never;
}
const tail = (registries: PublicKey[]) => hookwars.refreshTail(mint, pool, registries).map((m) => S(m.pubkey));

describe('refresh tail (app pass 5)', () => {
  const slots = [slot(3, itemX, 1), slot(4, null, 0), slot(1, itemZ, 0)];
  it('lists the forwarded slots after the change, the incoming item in its slot', async () => {
    const conn = world(slots);
    expect((await refreshTailOf(conn, mint, 1, itemY)).map((m) => S(m.pubkey))).toEqual(tail([hookwars.itemRegistryAddress(mint, itemX), hookwars.itemRegistryAddress(mint, itemY)]));
  });
  it('emptying a forwarding slot drops its registry; an item without pool callbacks adds none', async () => {
    const conn = world(slots);
    expect((await refreshTailOf(conn, mint, 0, null)).map((m) => S(m.pubkey))).toEqual(tail([]));
    expect((await refreshTailOf(conn, mint, 1, itemZ)).map((m) => S(m.pubkey))).toEqual(tail([hookwars.itemRegistryAddress(mint, itemX)]));
  });
  it('a slot that is neither Pool nor Relation has no tail', async () => {
    expect(await refreshTailOf(world(slots), mint, 2, itemY)).toEqual([]);
  });
  it('execute applies a passed proposal with the incoming item, its refresh tail last', async () => {
    const proposal = enc('armory', 'Proposal', { mint, slot: 1, nonce: 4n, proposer: owner, item: itemY, status: 1 });
    const conn = world(slots, [[hookwars.proposalAddress(mint, 1, 4n), proposal]]);
    const [ix] = await PREPARES['proposals/execute/prepare']!.build({ owner: S(owner), mint: S(mint), slot: 1, nonce: '4' }, conn);
    const n = hookwars.coderOf('armory').accountsOf('execute').length;
    expect(ix!.keys.slice(n).map((k) => S(k.pubkey))).toEqual(tail([hookwars.itemRegistryAddress(mint, itemX), hookwars.itemRegistryAddress(mint, itemY)]));
  });
  it('execute refuses a proposal that has not passed', async () => {
    const proposal = enc('armory', 'Proposal', { mint, slot: 1, nonce: 5n, proposer: owner, item: itemY, status: 0 });
    const conn = world(slots, [[hookwars.proposalAddress(mint, 1, 5n), proposal]]);
    await expect(PREPARES['proposals/execute/prepare']!.build({ owner: S(owner), mint: S(mint), slot: 1, nonce: '5' }, conn)).rejects.toThrow(/not passed/);
  });
});
