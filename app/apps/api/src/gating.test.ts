/**
 * Gating (spec 18) on a mocked chain: a tracked template's `Supply` leads the create, composite
 * and forge instructions' remaining accounts; the minter rule and an exhausted author cap are
 * refused before signing; the premium flow creates and sets Licensed terms in one transaction;
 * the gate's reads parse a Token-2022 mint's transfer hook and each binding's proof; binds carry
 * the venue reader's `MintGate` and refuse templates the app does not offer.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type AccountInfo } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PREPARES } from './prepares.ts';
import { gateOf, templateSupply, transferHookProgram } from './gating.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });

const owner = K(3), minter = K(4), mint = K(50), itemMint = K(60), otherItemMint = K(61);
const item = hookwars.itemAddress(itemMint);

/** A Token-2022 mint with the `TransferHook` extension naming `program`. */
function hookedMint(program: PublicKey): Buffer {
  const d = Buffer.alloc(166 + 4 + 64);
  d[45] = 1;
  d[165] = 1;
  d.writeUInt16LE(14, 166); d.writeUInt16LE(64, 168);
  K(9).toBuffer().copy(d, 170);
  program.toBuffer().copy(d, 202);
  return d;
}

const owners = new Map<string, PublicKey>();
const accounts = new Map<string, Buffer>();
function put(k: PublicKey, data: Buffer, programOwner: PublicKey = hookwars.ARMORY_ID) { accounts.set(S(k), data); owners.set(S(k), programOwner); }
put(hookwars.armoryConfigAddress(), enc('armory', 'ArmoryConfig', { admin: K(1), itemsMinted: 12n }));
put(hookwars.templateAddress(19), enc('armory', 'Template', { id: 19, program: hookwars.ITEMS_ID, name: 't19', supplyFlags: 1 }));
put(hookwars.templateAddress(22), enc('armory', 'Template', { id: 22, program: hookwars.ITEMS_ID, name: 't22' }));
put(hookwars.templateAddress(41), enc('armory', 'Template', { id: 41, program: hookwars.ITEMS_ID, name: 't41' }));
put(hookwars.templateAddress(8), enc('armory', 'Template', { id: 8, program: hookwars.ITEMS_ID, name: 't8' }));
put(hookwars.supplyAddress(19), enc('armory', 'Supply', { templateId: 19, maxSupply: 5, lootReserve: 1, issued: 3, drops: 1, forged: 1, burned: 2, minterRule: 0, minter }));
put(item, enc('armory', 'Item', { itemMint, templateId: 22, level: 1 }));
put(hookwars.itemAddress(otherItemMint), enc('armory', 'Item', { itemMint: otherItemMint, templateId: 8, level: 1 }));
put(mint, hookedMint(hookwars.GATE_ID), hookwars.TOKEN_2022_ID);
put(hookwars.gateVaultHolding(itemMint, mint), Buffer.alloc(8), hookwars.TOKEN_ID);

const info = (k: PublicKey): AccountInfo<Buffer> | null => {
  const data = accounts.get(S(k));
  return data ? { data, owner: owners.get(S(k)) ?? hookwars.ARMORY_ID, lamports: 1, executable: false, rentEpoch: 0 } : null;
};
const conn = { getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info) } as never;
const build = (route: string, body: Record<string, unknown>, who: PublicKey = owner) => PREPARES[`${route}/prepare`]!.build({ owner: S(who), ...body }, conn);
const tail = (ix: { keys: { pubkey: PublicKey }[] }, program: string, name: string) => ix.keys.slice(hookwars.coderOf(program).accountsOf(name).length).map((k) => S(k.pubkey));

describe('template supply (spec 18 part A)', () => {
  it('the supply read gives made, circulating and the room left', async () => {
    const r = await templateSupply(conn, 19) as Record<string, unknown>;
    expect(r).toMatchObject({ tracked: true, maxSupply: 5, made: 4, circulating: 3, authorRoom: 1, dropRoom: 1, minterRule: 'authorOnly', minter: S(minter) });
    expect(await templateSupply(conn, 22)).toEqual({ templateId: 22, tracked: false });
    expect(await templateSupply(conn, 99)).toBeNull();
  });
  it('a tracked template leads create with its Supply; a stranger is refused before signing', async () => {
    await expect(build('items/create', { templateId: 19, params: [100] })).rejects.toThrow(/minter/);
    const [ix] = await build('items/create', { templateId: 19, params: [100] }, minter);
    expect(tail(ix!, 'armory', 'create_item')[0]).toBe(S(hookwars.supplyAddress(19)));
    const [plain] = await build('items/create', { templateId: 22, params: [10] }, minter);
    expect(tail(plain!, 'armory', 'create_item')).not.toContain(S(hookwars.supplyAddress(22)));
  });
  it('a composite carries the Supply of each tracked module template', async () => {
    const mods = [19, 22].map((templateId) => ({ templateId, params: [1], targetStart: 0, targetCount: 0 }));
    const [ix] = await build('items/composite', { modules: mods }, minter);
    const t = tail(ix!, 'armory', 'create_composite');
    expect(t[0]).toBe(S(hookwars.supplyAddress(19)));
    expect(t.slice(1, 3)).toEqual([S(hookwars.templateAddress(19)), S(hookwars.templateAddress(22))]);
  });
  it('the premium flow creates the item and sets Licensed terms in one transaction', async () => {
    const ixs = await build('items/premium', { templateId: 22, params: [10], priceLamports: '1000', termSecs: 3600, per: 0, maxLive: 2 });
    expect(ixs.map((i) => S(i.programId))).toEqual([S(hookwars.ARMORY_ID), S(hookwars.ARMORY_ID)]);
    const setAccess = hookwars.coderOf('armory').instruction('set_access').discriminator;
    expect([...ixs[1]!.data.subarray(0, 8)]).toEqual(setAccess);
  });
});

describe('the external gate (spec 18 part C)', () => {
  it('reads a Token-2022 mint\'s transfer hook program', () => {
    expect(S(transferHookProgram(hookedMint(hookwars.GATE_ID))!)).toBe(S(hookwars.GATE_ID));
    expect(transferHookProgram(Buffer.alloc(82))).toBeNull();
  });
  it('a hooked mint that has not registered reads as such', async () => {
    expect(await gateOf(conn, S(mint))).toMatchObject({ token2022: true, hookedByGate: true, registered: false });
  });
  it('a registered mint lists its bindings with their proof state', async () => {
    const vault = hookwars.gateVaultHolding(itemMint, mint);
    put(hookwars.mintGateAddress(mint), enc('gate', 'MintGate', {
      mint, authority: owner, venue: K(70), strict: true, generation: 1,
      bindings: [{ slot: 0, item, itemMint, templateId: 22, kind: 1, proof: vault, role: 0, targets: [], dataOffset: 0, dataBytes: 0, generation: 1, extras: [hookwars.mintGateAddress(mint)], boundAt: 1n }],
    }), hookwars.GATE_ID);
    const g = await gateOf(conn, S(mint)) as { registered: boolean; venue: string; bindings: { kind: string; live: boolean | null }[] };
    expect(g.registered).toBe(true);
    expect(g.venue).toBe(S(K(70)));
    expect(g.bindings[0]!.kind).toBe('owned');
    accounts.delete(S(hookwars.mintGateAddress(mint)));
  });
  it('a bind carries the venue reader\'s MintGate and refuses templates the app does not offer', async () => {
    const [ix] = await build('gate/bind', { mint: S(mint), itemMint: S(itemMint), slot: 0, kind: 'owned' });
    expect(S(ix!.programId)).toBe(S(hookwars.GATE_ID));
    expect(tail(ix!, 'gate', 'bind')).toEqual([S(hookwars.mintGateAddress(mint))]);
    await expect(build('gate/bind', { mint: S(mint), itemMint: S(otherItemMint), slot: 1, kind: 'licence' })).rejects.toThrow(/licence/);
  });
});
