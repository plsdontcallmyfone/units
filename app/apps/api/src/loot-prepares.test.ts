/**
 * D-4 (17-randomness): loot rolls and reveals on Switchboard randomness, against a mocked chain and
 * a mocked Switchboard gateway (no network): Switchboard's commit sits before `roll` and its reveal
 * before `reveal`, and the reveal names the template its value draws.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { PublicKey, TransactionInstruction, type AccountInfo } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PREPARES } from './prepares.ts';
import { setRandomnessGateway, switchboardGateway, type RandomnessGateway } from './switchboard.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();
const SB = hookwars.SWITCHBOARD_DEVNET;

const empty = (): hookwars.SlotData => ({
  kind: 0, equipRule: 0, bounds: { maxCutBps: 0, mayRefuse: false, mayWriteData: false, mayAnswerTouch: false, mayBurn: false }, dataOffset: 0, dataLen: 0,
  item: PublicKey.default, program: PublicKey.default, flags: 0, poolFlags: 0, equipVault: PublicKey.default, signerBump: 0, launchSignerBump: 0, dataEpoch: 0, extraCount: 0,
});
const mint = K(2), owner = K(3), raidItem = K(31), rng = K(51), revealer = K(4), stranger = K(5);
const mintData = hookwars.idlAccountCodec<hookwars.SlotMintData>('token', 'Mint').encode({
  version: 1, decimals: 6, supply: 1n, maxSupply: 0n, mintAuthority: null, freezeAuthority: null, hookAuthority: null, metadataAuthority: null,
  hookProgram: null, hookFlags: 0, name: 'T', symbol: 'T', uri: '', createdAt: 0n, creator: K(9), hookSignerBump: 0, reserved: Buffer.alloc(31),
  slotAuthority: K(8), slotCount: 2,
  slots: [empty(), { ...empty(), kind: 4, item: raidItem, program: hookwars.ITEMS_ID, flags: 1 | 256, dataOffset: 32, dataLen: 12 }, empty(), empty()],
});
const item = (templateId: number, itemMint: PublicKey) => hookwars.itemCodec().encode({
  version: 1, bump: 255, itemMint, templateId, params: Array(hookwars.PARAM_FIELDS).fill(0),
  manifest: { kind: 0, tokenFlags: 0, poolFlags: 0, maxCutBuyBps: 0, maxCutSellBps: 0, maxCutTransferBps: 0, maxDiscountBps: 0, mayRefuse: false, mayBurn: false, dataBytes: 0, readsOtherPools: 0 },
  author: K(9), royaltyBps: 0, level: 1, source: 0, equippedCount: 0, royaltyOwnerBump: 255, createdAt: 0n, hasWear: false, accessMode: 0, exclusive: false, reserved: Buffer.alloc(29),
});
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(8000)]));
const armoryConfig = hookwars.armoryConfigCodec.encode({ ...(zeroOf('armory', 'ArmoryConfig') as unknown as hookwars.ArmoryConfigData), itemsMinted: 12n });
const warConfig = (program: PublicKey) => hookwars.warConfigCodec.encode({ ...(zeroOf('war', 'WarConfig') as unknown as hookwars.WarConfigData), randomnessProgram: program, currentSeason: 3 });
const range = (min: number, max: number) => ({ min, max });
const F = hookwars.PARAM_FIELDS;
const table = hookwars.idlAccountCodec<hookwars.LootTableData>('war', 'LootTable').encode({
  ...(zeroOf('war', 'LootTable') as unknown as hookwars.LootTableData), season: 3, count: 2,
  entries: [
    { templateId: 1, weight: 1, ranges: Array.from({ length: F }, () => range(10, 20)) },
    { templateId: 7, weight: 3, ranges: Array.from({ length: F }, () => range(0, 0)) },
    ...Array.from({ length: (zeroOf('war', 'LootTable') as unknown as hookwars.LootTableData).entries.length - 2 }, () => ({ templateId: 0, weight: 0, ranges: Array.from({ length: F }, () => range(0, 0)) })),
  ],
});
const holding = hookwars.holdingAddr(mint, owner);
const rollKey = hookwars.rollAddress(holding, 5n);
const rollData = (program: PublicKey) => hookwars.rollRequestCodec.encode({
  ...(zeroOf('war', 'RollRequest') as unknown as hookwars.RollRequestData), owner, mint, holding, nonce: 5n, season: 3, requestedSlot: 99n, oracleProgram: program, oracleAccount: rng,
});

let accounts = new Map<string, { data: Buffer; owner: PublicKey }>();
function chain(program: PublicKey, authority = owner) {
  accounts = new Map([
    [S(mint), { data: mintData, owner: hookwars.TOKEN_ID }], [S(raidItem), { data: item(1, K(33)), owner: hookwars.ARMORY_ID }],
    [S(hookwars.armoryConfigAddress()), { data: armoryConfig, owner: hookwars.ARMORY_ID }],
    [S(hookwars.WAR_CONFIG), { data: warConfig(program), owner: hookwars.WAR_ID }],
    [S(hookwars.lootTableAddress(3)), { data: table, owner: hookwars.WAR_ID }],
    [S(rollKey), { data: rollData(program), owner: hookwars.WAR_ID }],
    [S(rng), { data: Buffer.from(hookwars.encodeSbRandomness({ authority, seedSlot: 99n, revealSlot: 0n, value: new Uint8Array(32) })), owner: SB }],
  ]);
}
const info = (k: PublicKey): AccountInfo<Buffer> | null => {
  const a = accounts.get(S(k));
  return a ? { data: a.data, owner: a.owner, lamports: 1, executable: false, rentEpoch: 0 } : null;
};
const conn = { getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info) } as never;

const marker = (tag: number, keys: PublicKey[] = []) => new TransactionInstruction({ programId: SB, keys: keys.map((pubkey) => ({ pubkey, isSigner: false, isWritable: true })), data: Buffer.from([tag]) });
const calls: string[] = [];
// Value: bytes 0..8 = 1 (pick 1 of total weight 4 -> template 7), the rest 0.
const value = new Uint8Array(32); value[0] = 1;
const mock: RandomnessGateway = {
  create: async (_c, program, randomness, o) => { calls.push(`create ${S(program)} ${S(randomness)} ${S(o)}`); return marker(1, [randomness]); },
  commit: async (_c, program, randomness, o) => { calls.push(`commit ${S(program)} ${S(randomness)} ${S(o)}`); return marker(2, [randomness]); },
  reveal: async (_c, program, randomness, payer) => { calls.push(`reveal ${S(program)} ${S(randomness)} ${S(payer)}`); return { ix: marker(3, [randomness]), value }; },
};

afterEach(() => { calls.length = 0; setRandomnessGateway(switchboardGateway); });

describe('loot rolls on Switchboard randomness (D-4)', () => {
  it('the SDK reads the Switchboard layout and draws like the war program', () => {
    const r = { authority: owner, seedSlot: 41n, revealSlot: 42n, value: Uint8Array.from({ length: 32 }, (_, i) => i) };
    const bytes = hookwars.encodeSbRandomness(r);
    expect(bytes.length).toBe(408);
    expect([...bytes.subarray(0, 8)]).toEqual([10, 66, 229, 135, 220, 239, 217, 114]);
    expect(hookwars.decodeSbRandomness(bytes)).toEqual(r);
    expect(() => hookwars.decodeSbRandomness(bytes.subarray(0, 400))).toThrow();
    const entries = hookwars.activeLoot(hookwars.decodeLootTable(table));
    expect(hookwars.drawLoot(entries, new Uint8Array(32))).toEqual({ templateId: 1, params: Array(F).fill(10) });
    expect(hookwars.drawLoot(entries, value)?.templateId).toBe(7);
    // Field 0 from bytes 8..10: 0xffff spans to the top of the range.
    const top = new Uint8Array(32); top[8] = 0xff; top[9] = 0xff;
    expect(hookwars.drawLoot(entries, top)?.params[0]).toBe(20);
    expect(hookwars.isSwitchboard(hookwars.SWITCHBOARD_MAINNET) && hookwars.isSwitchboard(SB) && !hookwars.isSwitchboard(hookwars.WAR_ID)).toBe(true);
  });

  it('creating a randomness account is a stage the browser co-signs, and needs Switchboard configured', async () => {
    setRandomnessGateway(mock); chain(SB);
    const stages = await PREPARES['rolls/randomness/prepare']!.staged!({ owner: S(owner), randomness: S(rng) }, conn);
    expect(stages).toHaveLength(1);
    expect(stages[0]!.extraSigners).toEqual(['randomness']);
    expect(calls).toEqual([`create ${S(SB)} ${S(rng)} ${S(owner)}`]);
    chain(K(50));
    await expect(PREPARES['rolls/randomness/prepare']!.staged!({ owner: S(owner), randomness: S(rng) }, conn)).rejects.toThrow(/does not use Switchboard/);
  });

  it('a roll is Switchboard commit then roll, on the owner\'s own randomness account', async () => {
    setRandomnessGateway(mock); chain(SB);
    const ixs = await PREPARES['rolls/prepare']!.build({ owner: S(owner), mint: S(mint), nonce: 5, oracleAccount: S(rng) }, conn);
    expect(ixs.map((i) => S(i.programId))).toEqual([S(SB), S(hookwars.WAR_ID)]);
    const names = hookwars.coderOf('war').accountsOf('roll').map((a) => a.name);
    expect(S(ixs[1]!.keys[names.indexOf('oracle_program')]!.pubkey)).toBe(S(SB));
    expect(S(ixs[1]!.keys[names.indexOf('oracle_account')]!.pubkey)).toBe(S(rng));
    chain(SB, stranger);
    await expect(PREPARES['rolls/prepare']!.build({ owner: S(owner), mint: S(mint), nonce: 5, oracleAccount: S(rng) }, conn)).rejects.toThrow(/another wallet/);
    accounts.delete(S(rng));
    await expect(PREPARES['rolls/prepare']!.build({ owner: S(owner), mint: S(mint), nonce: 5, oracleAccount: S(rng) }, conn)).rejects.toThrow(/randomness account first/);
  });

  it('the adapter path (the test stub) keeps a single roll instruction', async () => {
    setRandomnessGateway(mock); chain(K(50));
    const ixs = await PREPARES['rolls/prepare']!.build({ owner: S(owner), mint: S(mint), nonce: 5, oracleAccount: S(rng) }, conn);
    expect(ixs.map((i) => S(i.programId))).toEqual([S(hookwars.WAR_ID)]);
    expect(calls).toEqual([]);
  });

  it('a reveal is Switchboard reveal then reveal, minting the template the value draws', async () => {
    setRandomnessGateway(mock); chain(SB);
    const ixs = await PREPARES['rolls/reveal/prepare']!.build({ revealer: S(revealer), owner: S(owner), mint: S(mint), nonce: 5 }, conn);
    expect(ixs.map((i) => S(i.programId))).toEqual([S(SB), S(hookwars.WAR_ID)]);
    expect(calls).toEqual([`reveal ${S(SB)} ${S(rng)} ${S(revealer)}`]);
    const reveal = ixs[1]!;
    const names = hookwars.coderOf('war').accountsOf('reveal').map((a) => a.name);
    expect(S(reveal.keys[names.indexOf('roll_request')]!.pubkey)).toBe(S(rollKey));
    expect(S(reveal.keys[names.indexOf('oracle_account')]!.pubkey)).toBe(S(rng));
    const rest = reveal.keys.slice(names.length).map((k) => S(k.pubkey));
    expect(rest).toContain(S(hookwars.templateAddress(7)));
    expect(rest).toContain(S(hookwars.itemMintAddress(12n)));
    expect(rest).toContain(S(hookwars.holdingAddr(hookwars.itemMintAddress(12n), owner)));
    expect(reveal.keys.slice(names.length).every((k) => !k.isSigner)).toBe(true);
    await expect(PREPARES['rolls/reveal/prepare']!.build({ revealer: S(revealer), owner: S(owner), mint: S(mint), nonce: 6 }, conn)).rejects.toThrow(/no open roll/);
  });

  it('an adapter reveal waits for a fulfilled value', async () => {
    setRandomnessGateway(mock); chain(K(50));
    const stub = Buffer.alloc(89);
    accounts.set(S(rng), { data: stub, owner: K(50) });
    await expect(PREPARES['rolls/reveal/prepare']!.build({ revealer: S(revealer), owner: S(owner), mint: S(mint), nonce: 5 }, conn)).rejects.toThrow(/not ready/);
    stub[48] = 1; stub[57] = 0;
    const ixs = await PREPARES['rolls/reveal/prepare']!.build({ revealer: S(revealer), owner: S(owner), mint: S(mint), nonce: 5 }, conn);
    expect(ixs.map((i) => S(i.programId))).toEqual([S(hookwars.WAR_ID)]);
    expect(calls).toEqual([]);
  });
});
