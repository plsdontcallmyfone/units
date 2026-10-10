/**
 * App pass 5 prepares on a mocked chain: coalition form and join name each member's Coalition item
 * (template 43 naming the id), its template and its war state; a token without one is refused;
 * market terms and the vault delegate revoke use the IDL's accounts.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type AccountInfo } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PREPARES } from './prepares.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });
const zeroSlot = (zeroOf('token', 'Mint').slots as Record<string, unknown>[])[0]!;
const mintWith = (item: PublicKey) => enc('token', 'Mint', { slotCount: 1, slots: [{ ...zeroSlot, kind: 2, item, program: hookwars.ITEMS_ID }, zeroSlot, zeroSlot, zeroSlot] });

const owner = K(3), a = K(20), b = K(21), lone = K(22), itemA = K(30), itemB = K(31), itemOther = K(32);
const params = (id: number) => [id, ...Array(10).fill(0)];
const accounts = new Map<string, Buffer>([
  [S(a), mintWith(itemA)], [S(b), mintWith(itemB)], [S(lone), mintWith(itemOther)],
  [S(itemA), enc('armory', 'Item', { templateId: 43, params: params(7) })],
  [S(itemB), enc('armory', 'Item', { templateId: 43, params: params(7) })],
  [S(itemOther), enc('armory', 'Item', { templateId: 43, params: params(8) })],
  [S(hookwars.warStateAddress(a)), Buffer.alloc(8)], [S(hookwars.warStateAddress(b)), Buffer.alloc(8)], [S(hookwars.warStateAddress(lone)), Buffer.alloc(8)],
]);
const info = (k: PublicKey): AccountInfo<Buffer> | null => { const data = accounts.get(S(k)); return data ? { data, owner: hookwars.ARMORY_ID, lamports: 1, executable: false, rentEpoch: 0 } : null; };
const conn = { getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info) } as never;
const build = (route: string, body: Record<string, unknown>) => PREPARES[`${route}/prepare`]!.build({ owner: S(owner), ...body }, conn);
const tailOf = (ix: { keys: { pubkey: PublicKey }[] }, program: string, name: string) => ix.keys.slice(hookwars.coderOf(program).accountsOf(name).length).map((k) => S(k.pubkey));

describe('app pass 5 prepares', () => {
  it('form names every member as [mint, Coalition item, template 43, war state]', async () => {
    const [ix] = await build('war/coalition/form', { id: 7, termSecs: '86400', members: [S(a), S(b)] });
    expect(S(ix!.programId)).toBe(S(hookwars.WAR_ID));
    expect(tailOf(ix!, 'war', 'form_coalition')).toEqual([a, itemA, hookwars.templateAddress(43), hookwars.warStateAddress(a), b, itemB, hookwars.templateAddress(43), hookwars.warStateAddress(b)].map(S));
  });
  it('a token whose Coalition item names another id cannot join', async () => {
    const id = Buffer.alloc(4); id.writeUInt32LE(7);
    accounts.set(S(PublicKey.findProgramAddressSync([Buffer.from('coalition'), id], hookwars.WAR_ID)[0]), Buffer.alloc(8));
    await expect(build('war/coalition/join', { id: 7, mint: S(lone) })).rejects.toThrow(/naming coalition 7/);
    await expect(build('war/coalition/join', { id: 9, mint: S(a) })).rejects.toThrow(/has not been formed/);
  });
  it('market terms and the vault revoke', async () => {
    const [p] = await build('book/market-terms/propose', { materialId: 2, tickLamports: '10', minSize: '1' });
    expect(S(p!.keys[0]!.pubkey)).toBe(S(owner));
    await expect(build('book/market-terms/apply', { materialId: 2 })).rejects.toThrow(/No market terms/);
    const [r] = await build('agents/vault/revoke', { passport: S(K(40)), mint: S(K(41)) });
    const vault = hookwars.agentVaultAddress(K(40));
    expect(r!.keys.map((k) => S(k.pubkey))).toContain(S(hookwars.holdingAddr(K(41), vault)));
  });
});
