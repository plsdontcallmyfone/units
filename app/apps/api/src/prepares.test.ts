/**
 * The prepares that the merged programs now allow, built against a mocked chain holding accounts
 * encoded with the IDL codecs (no LiteSVM, nothing deployed): each returns the instruction its
 * program's Rust client would build (the builders themselves are pinned to the Rust clients by
 * `packages/sdk/src/hookwars/instructions.vectors.test.ts`).
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type AccountInfo } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PREPARES } from './prepares.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();

const empty = (): hookwars.SlotData => ({
  kind: 0, equipRule: 0, bounds: { maxCutBps: 0, mayRefuse: false, mayWriteData: false, mayAnswerTouch: false, mayBurn: false }, dataOffset: 0, dataLen: 0,
  item: PublicKey.default, program: PublicKey.default, flags: 0, poolFlags: 0, equipVault: PublicKey.default, signerBump: 0, launchSignerBump: 0, dataEpoch: 0, extraCount: 0,
});

const mint = K(2), owner = K(3), warItem = K(30), raidItem = K(31), itemA = K(40), itemB = K(41);
const mintData = hookwars.idlAccountCodec<hookwars.SlotMintData>('token', 'Mint').encode({
  version: 1, decimals: 6, supply: 1n, maxSupply: 0n, mintAuthority: null, freezeAuthority: null, hookAuthority: null, metadataAuthority: null,
  hookProgram: null, hookFlags: 0, name: 'T', symbol: 'T', uri: '', createdAt: 0n, creator: K(9), hookSignerBump: 0, reserved: Buffer.alloc(31),
  slotAuthority: K(8), slotCount: 2,
  slots: [
    { ...empty(), kind: 6, item: warItem, program: hookwars.ITEMS_ID },
    { ...empty(), kind: 4, item: raidItem, program: hookwars.ITEMS_ID, flags: 1 | 256, dataOffset: 32, dataLen: 12 },
    empty(), empty(),
  ],
});
const item = (templateId: number, itemMint: PublicKey) => hookwars.itemCodec().encode({
  version: 1, bump: 255, itemMint, templateId, params: Array(hookwars.PARAM_FIELDS).fill(0),
  manifest: { kind: 0, tokenFlags: 0, poolFlags: 0, maxCutBuyBps: 0, maxCutSellBps: 0, maxCutTransferBps: 0, maxDiscountBps: 0, mayRefuse: false, mayBurn: false, dataBytes: 0, readsOtherPools: 0 },
  author: K(9), royaltyBps: 0, level: 1, source: 0, equippedCount: 0, royaltyOwnerBump: 255, createdAt: 0n, hasWear: false, reserved: Buffer.alloc(31),
});
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const armoryConfig = hookwars.armoryConfigCodec.encode({ ...(zeroOf('armory', 'ArmoryConfig') as unknown as hookwars.ArmoryConfigData), itemsMinted: 12n });
const warConfig = hookwars.warConfigCodec.encode({ ...(zeroOf('war', 'WarConfig') as unknown as hookwars.WarConfigData), randomnessProgram: K(50), protocolTreasury: K(60), lastWinner: K(70), lastWinnerSeason: 2, currentSeason: 3 });

const TOKEN_MINTS = new Set([S(mint), S(K(90))]);
const accounts = new Map<string, Buffer>([
  [S(mint), mintData], [S(warItem), item(9, K(32))], [S(raidItem), item(1, K(33))],
  [S(itemA), item(1, K(42))], [S(itemB), item(1, K(43))],
  [S(hookwars.armoryConfigAddress()), armoryConfig], [S(hookwars.WAR_CONFIG), warConfig],
]);
const info = (k: PublicKey): AccountInfo<Buffer> | null => {
  const data = accounts.get(S(k));
  if (!data) return null;
  const owner = TOKEN_MINTS.has(S(k)) ? hookwars.TOKEN_ID : hookwars.ARMORY_ID;
  return { data, owner, lamports: 1, executable: false, rentEpoch: 0 };
};
const conn = { getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info) } as never;

describe('prepares the merged programs allow', () => {
  it('claim bounty: War orders and the Raid slot come from the slot table', async () => {
    const [ix] = await PREPARES['bounties/prepare']!.build({ owner: S(owner), mint: S(mint) }, conn);
    expect(ix!.programId.equals(hookwars.WAR_ID)).toBe(true);
    const names = hookwars.coderOf('war').accountsOf('claim_bounty').map((a) => a.name);
    expect(S(ix!.keys[names.indexOf('orders_item')]!.pubkey)).toBe(S(warItem));
    expect(S(ix!.keys[names.indexOf('orders_template')]!.pubkey)).toBe(S(hookwars.templateAddress(9)));
    expect(ix!.data.subarray(8)).toEqual(Buffer.from([1])); // raid_slot 1
  });
  it('claim quest and roll use the Raid slot', async () => {
    const [q] = await PREPARES['quests/prepare']!.build({ owner: S(owner), mint: S(mint), season: 3, questId: 1, period: 4 }, conn);
    expect([...q!.data.subarray(8)]).toEqual([1, 4, 0, 0, 0, 1]);
    const [r] = await PREPARES['rolls/prepare']!.build({ owner: S(owner), mint: S(mint), nonce: 5, oracleAccount: S(K(51)) }, conn);
    const names = hookwars.coderOf('war').accountsOf('roll').map((a) => a.name);
    expect(S(r!.keys[names.indexOf('oracle_program')]!.pubkey)).toBe(S(K(50)));
  });
  it('forge derives the new item from items_minted and refuses mixed templates', async () => {
    const [f] = await PREPARES['forge/prepare']!.build({ owner: S(owner), itemA: S(itemA), itemB: S(itemB) }, conn);
    const names = hookwars.coderOf('armory').accountsOf('forge').map((a) => a.name);
    expect(S(f!.keys[names.indexOf('item_mint')]!.pubkey)).toBe(S(hookwars.itemMintAddress(12n)));
    await expect(PREPARES['forge/prepare']!.build({ owner: S(owner), itemA: S(itemA), itemB: S(warItem) }, conn)).rejects.toThrow(/same template/);
  });
  it('season and prize cranks read the war config', async () => {
    const [o] = await PREPARES['seasons/open/prepare']!.build({ owner: S(owner) }, conn);
    const on = hookwars.coderOf('war').accountsOf('open_season').map((a) => a.name);
    expect(S(o!.keys[on.indexOf('next')]!.pubkey)).toBe(S(hookwars.seasonAddress(4)));
    const [p] = await PREPARES['prize/split/prepare']!.build({ owner: S(owner) }, conn);
    const pn = hookwars.coderOf('war').accountsOf('split_protocol_fees').map((a) => a.name);
    expect(S(p!.keys[pn.indexOf('winner_chest')]!.pubkey)).toBe(S(hookwars.warChestAddress(K(70))));
    expect(S(p!.keys[pn.indexOf('treasury')]!.pubkey)).toBe(S(K(60)));
  });
  it('a token without a Raid item is refused with a sentence', async () => {
    const other = K(90);
    accounts.set(S(other), hookwars.idlAccountCodec<hookwars.SlotMintData>('token', 'Mint').encode({ ...hookwars.decodeSlotMint(mintData), slotCount: 0 }));
    await expect(PREPARES['bounties/prepare']!.build({ owner: S(owner), mint: S(other) }, conn)).rejects.toThrow(/no Raid item/);
  });
});
