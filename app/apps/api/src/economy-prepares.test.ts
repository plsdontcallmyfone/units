/**
 * The economy prepares (app pass v3: specs 12 section 7 and 13 section 6, runtime gaps R-1 and
 * R-2), built against a mocked chain holding accounts encoded with the IDL codecs: each returns
 * the IDL's instruction with its account list, and the optional suffixes in the order the
 * programs split them (`[..., rent, craft, fee, social, agents]`), present only when they apply.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type AccountInfo, type AccountMeta, type TransactionInstruction } from '@solana/web3.js';
import { FIXED_ADDRESSES, LP_FEE_BPS, PROGRAM_IDS } from '@hookwars/shared';
import { hookwars, launchPoolAddress } from '@hookwars/sdk';
import { PREPARES } from './prepares.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();
const QUOTE = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });
const keysOf = (ms: AccountMeta[]) => ms.map((m) => S(m.pubkey));

const owner = K(3), mint = K(2), admin = K(80), registrant = K(81), lessor = K(82), holder = K(83), treasury = K(84), seasonPool = K(85);
const itemMintA = K(100), itemMintB = K(101), wearing = K(102), maker1 = K(110), maker2 = K(111), worst = K(112);
const itemA = hookwars.itemAddress(itemMintA), itemB = hookwars.itemAddress(itemMintB), wearItem = hookwars.itemAddress(wearing);
const NOW = BigInt(Math.floor(Date.now() / 1000));

const itemData = (templateId: number, itemMint: PublicKey, hasWear = false) => enc('armory', 'Item', { itemMint, templateId, level: 1, hasWear });
const tmpl = (id: number, over: Record<string, unknown>) => enc('armory', 'Template', { id, program: hookwars.ITEMS_ID, registeredBy: registrant, name: `t${id}`, ...over });
const order = (id: number, o: PublicKey, price: bigint, size: bigint, expiresAt = 0n) => ({ id: BigInt(id), owner: o, price, size, quoteLocked: 0n, bounty: 0n, expiresAt });

const accounts = new Map<string, Buffer>([
  [S(hookwars.armoryConfigAddress()), enc('armory', 'ArmoryConfig', { admin, itemsMinted: 12n, itemProtocolBps: 100 })],
  [S(hookwars.templateAddress(1)), tmpl(1, {})],
  [S(hookwars.templateAddress(7)), tmpl(7, { chargesOnCreate: 3, authorBps: 500 })],
  [S(hookwars.templateAddress(41)), tmpl(41, {})],
  [S(hookwars.authorCounterAddress(owner)), enc('armory', 'AuthorCounter', { wallet: owner })],
  [S(hookwars.profileAddress(owner)), enc('social', 'Profile', { wallet: owner })],
  [S(hookwars.craftConfigAddress()), enc('craft', 'CraftConfig', { treasury, seasonPool, outputProgram: hookwars.ARMORY_ID })],
  [S(hookwars.recipeAddress(5)), enc('craft', 'Recipe', { id: 5, terms: { kind: 0, inputs: [{ materialId: 2, amount: 1n }, { materialId: 3, amount: 4n }], feeLamports: 0n, templateId: 7, paramMin: Array(11).fill(0), paramMax: Array(11).fill(0), chargesRestored: 0, minLevel: 0, active: true } })],
  [S(hookwars.recipeAddress(6)), enc('craft', 'Recipe', { id: 6, terms: { kind: 1, inputs: [{ materialId: 2, amount: 1n }], feeLamports: 0n, templateId: 7, paramMin: Array(11).fill(0), paramMax: Array(11).fill(0), chargesRestored: 2, minLevel: 0, active: true } })],
  [S(hookwars.dropRuleAddress(0)), enc('craft', 'DropRule', { source: 0, materialId: 2 })],
  [S(hookwars.presetAddress(3)), enc('armory', 'Preset', { id: 3, name: 'p', templateIds: [1, 7] })],
  [S(itemA), itemData(1, itemMintA)], [S(itemB), itemData(7, itemMintB)], [S(wearItem), itemData(7, wearing, true)],
  [S(hookwars.bookConfigAddress()), enc('book', 'BookConfig', { treasury, params: { takerBps: 0, makerBps: 0, slots: 2, matchMax: 4, createLevel: 0, orderBountyLamports: 0n, adminTimelockSecs: 0, tickMinLamports: 1n, tickMaxLamports: 1_000_000_000n, minSizeMax: 1_000_000n, skillMinFeeLamports: 0n } })],
  [S(hookwars.bookMarketAddress(hookwars.materialMintAddress(2))), enc('book', 'BookMarket', {
    baseMint: hookwars.materialMintAddress(2), materialId: 2, tickLamports: 1n, minSize: 1n,
    asks: [order(1, maker1, 10n, 3n), order(2, K(113), 11n, 1n, 1n), order(3, maker2, 12n, 5n)],
    bids: [order(4, K(114), 9n, 1n), order(5, worst, 8n, 1n)],
  })],
  [S(hookwars.equipStateAddress(mint, 0)), enc('items', 'EquipState', { mint, slot: 0, item: wearItem, templateId: 7, runs: 5n, runsAtSettle: 2n, config: { targets: [], role: 0 } })],
  [S(hookwars.leaseAddress(wearItem)), enc('market', 'Lease', { lessor, item: wearItem, itemMint: wearing, tokenMint: mint, slot: 0, state: 1 })],
]);
const info = (k: PublicKey): AccountInfo<Buffer> | null => {
  const data = accounts.get(S(k));
  return data ? { data, owner: hookwars.ARMORY_ID, lamports: 1, executable: false, rentEpoch: 0 } : null;
};
const holding = hookwars.holdingCodec.encode({ ...(zeroOf('token', 'Holding') as unknown as Parameters<typeof hookwars.holdingCodec.encode>[0]), mint: wearing, owner: holder, amount: 1n });
const conn = {
  getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info),
  getProgramAccounts: async () => [{ pubkey: hookwars.holdingAddr(wearing, holder), account: { data: holding, owner: hookwars.TOKEN_ID, lamports: 1, executable: false, rentEpoch: 0 } }],
} as never;

async function one(route: string, body: Record<string, unknown>): Promise<TransactionInstruction> {
  const ixs = await PREPARES[route]!.build({ owner: S(owner), ...body }, conn);
  return ixs[ixs.length - 1]!;
}
/** The accounts after the IDL's named ones. */
const tail = (ix: TransactionInstruction, program: string, name: string) => ix.keys.slice(hookwars.coderOf(program).accountsOf(name).length);
const named = (ix: TransactionInstruction, program: string, name: string) => Object.fromEntries(hookwars.coderOf(program).accountsOf(name).map((a, i) => [a.name, S(ix.keys[i]!.pubkey)]));

describe('item creation carries the economy tail (12 I-5, 13 E-3, E-6)', () => {
  it('a plain template: the author counter, then the social suffix', async () => {
    const ix = await one('items/create/prepare', { templateId: 1, params: [1, 2], royaltyBps: 100 });
    expect(named(ix, 'armory', 'create_item').item_mint).toBe(S(hookwars.itemMintAddress(12n)));
    expect(keysOf(tail(ix, 'armory', 'create_item'))).toEqual([S(hookwars.authorCounterAddress(owner)), ...keysOf(hookwars.socialSuffix(hookwars.ARMORY_ID, owner))]);
  });
  it('a wearing template adds the init-wear suffix before the social one, and needs craft', async () => {
    const ix = await one('items/create/prepare', { templateId: 7, params: [], royaltyBps: 0 });
    const item = hookwars.itemAddress(hookwars.itemMintAddress(12n));
    const t = keysOf(tail(ix, 'armory', 'create_item'));
    expect(t).toEqual([S(hookwars.authorCounterAddress(owner)), ...keysOf(hookwars.initWearSuffix(hookwars.ARMORY_ID, item)), ...keysOf(hookwars.socialSuffix(hookwars.ARMORY_ID, owner))]);
    expect(t[1]).toBe(S(hookwars.CRAFT_ID));
    expect(t[5]).toBe(S(hookwars.wearAddress(item)));
  });
  it('composites, fuse and presets take their templates then the same tail', async () => {
    const mods = [{ templateId: 1, params: [], targetStart: 0, targetCount: 0 }, { templateId: 7, params: [], targetStart: 0, targetCount: 0 }];
    const c = await one('items/composite/prepare', { modules: mods, royaltyBps: 0 });
    expect(keysOf(tail(c, 'armory', 'create_composite')).slice(0, 3)).toEqual([S(hookwars.templateAddress(1)), S(hookwars.templateAddress(7)), S(hookwars.authorCounterAddress(owner))]);
    const f = await one('items/fuse/prepare', { components: [{ itemMint: S(itemMintA), start: 0, count: 0 }, { itemMint: S(itemMintB), start: 0, count: 0 }], royaltyBps: 0 });
    const ft = keysOf(tail(f, 'armory', 'fuse'));
    expect(ft.slice(0, 8)).toEqual([
      S(hookwars.templateAddress(1)), S(hookwars.templateAddress(7)),
      S(itemA), S(itemMintA), S(hookwars.holdingAddr(itemMintA, owner)), S(itemB), S(itemMintB), S(hookwars.holdingAddr(itemMintB, owner)),
    ]);
    expect([...f.data.subarray(8, 16)]).toEqual([2, 0, 0, 0, 0, 0, 0, 0]); // two FuseTarget entries: (0, 0), (0, 0)
    const p = await one('presets/mint/prepare', { presetId: 3, modules: mods, royaltyBps: 0 });
    expect(S(tail(p, 'armory', 'mint_composite')[0]!.pubkey)).toBe(S(hookwars.presetAddress(3)));
    await expect(one('presets/mint/prepare', { presetId: 3, modules: [...mods].reverse(), royaltyBps: 0 })).rejects.toThrow(/preset's templates in its order/);
  });
});

describe('proposals and leases (12 sections 2 and 3)', () => {
  it('propose appends the lease gate when it names an item', () => {
    const ix = hookwars.propose(owner, mint, 0, 4n, itemA, [], 0);
    expect(keysOf(tail(ix, 'armory', 'propose'))).toEqual([S(hookwars.leaseAddress(itemA))]);
    expect(tail(hookwars.propose(owner, mint, 0, 4n, null, [], 0), 'armory', 'propose')).toEqual([]);
  });
  it('close_proposal passes the bond mark and, when one exists, its bond', async () => {
    const proposal = hookwars.proposalAddress(mint, 1, 2n);
    accounts.set(S(proposal), enc('armory', 'Proposal', { mint, slot: 1, nonce: 2n, proposer: owner }));
    const a = named(await one('proposals/close/prepare', { mint: S(mint), slot: 1, nonce: '2' }), 'armory', 'close_proposal');
    expect([a.bond_mark, a.bond]).toEqual([S(hookwars.bondMarkAddress(proposal)), S(hookwars.ARMORY_ID)]);
    accounts.set(S(hookwars.bondMarkAddress(proposal)), enc('agents', 'BondMark', { bond: K(55) }));
    expect(named(await one('proposals/close/prepare', { mint: S(mint), slot: 1, nonce: '2' }), 'armory', 'close_proposal').bond).toBe(S(K(55)));
  });
  it('end_lease reverts the slot to its launch item through the market caller', () => {
    const metas = hookwars.revertForLeaseEndMetas(owner, mint, 0, wearItem, { oldItem: wearItem });
    expect(S(metas[0]!.pubkey)).toBe(S(hookwars.marketCallerAddress()));
    expect(metas[0]!.isSigner).toBe(false);
    expect(S(metas[metas.length - 1]!.pubkey)).toBe(S(hookwars.ARMORY_ID));
    expect(metas.filter((m) => m.isSigner).map((m) => S(m.pubkey))).toEqual([S(owner)]);
  });
});

describe('settle_equip suffixes (12 I-7, 13 E-2 to E-4)', () => {
  it('rent, craft and fee, in that order, each only when it applies', async () => {
    const ix = await one('settle/prepare', { mint: S(mint), slot: 0 });
    const t = keysOf(tail(ix, 'items', 'settle_equip').slice(2)); // after the one module's two destinations
    const rent = keysOf(hookwars.rentSuffix(wearItem, lessor, mint, QUOTE));
    const craft = keysOf(hookwars.settleCraftSuffix({ item: wearItem, itemMint: wearing, recipient: holder, materialId: 2 }));
    const fee = keysOf(hookwars.feeSuffix(7, admin, registrant, mint, QUOTE));
    expect(t).toEqual([...rent, ...craft, ...fee]);
    expect(craft).toHaveLength(12);
    expect(craft[11]).toBe(S(hookwars.holdingAddr(wearing, holder)));
  });
  it('no craft suffix once the item has not run since the last settle', async () => {
    accounts.set(S(hookwars.equipStateAddress(mint, 0)), enc('items', 'EquipState', { mint, slot: 0, item: wearItem, templateId: 7, runs: 5n, runsAtSettle: 5n, config: { targets: [], role: 0 } }));
    const ix = await one('settle/prepare', { mint: S(mint), slot: 0 });
    expect(tail(ix, 'items', 'settle_equip').slice(2)).toHaveLength(4 + 4);
  });
});

describe('craft and the book (11 sections 5 and 6)', () => {
  it('craft burns the inputs and forwards mint_crafted after the craft signer, with the wear suffix', async () => {
    const ix = await one('craft/prepare', { recipeId: 5 });
    const t = tail(ix, 'craft', 'craft');
    expect(keysOf(t.slice(0, 6))).toEqual(keysOf(hookwars.recipeInputMetas(owner, [2, 3])));
    const mc = hookwars.coderOf('armory').accountsOf('mint_crafted').length;
    expect(t).toHaveLength(6 + mc - 1 + 6);
    expect(S(t[6]!.pubkey)).toBe(S(owner)); // mint_crafted's payer
    expect(named(ix, 'craft', 'craft').treasury).toBe(S(treasury));
    await expect(one('craft/prepare', { recipeId: 6 })).rejects.toThrow(/repairs/);
  });
  it('repair names the Wear and refuses an item of another template', async () => {
    const ix = await one('craft/repair/prepare', { recipeId: 6, itemMint: S(wearing) });
    expect(named(ix, 'craft', 'repair').wear).toBe(S(hookwars.wearAddress(wearItem)));
    await expect(one('craft/repair/prepare', { recipeId: 6, itemMint: S(itemMintA) })).rejects.toThrow(/template 7 only/);
  });
  it('a bid crossing two asks passes both makers, skipping the expired one; resting on a full side passes the evicted owner', async () => {
    const ix = await one('book/place/prepare', { materialId: 2, side: 'bid', price: '12', size: '6' });
    const base = hookwars.materialMintAddress(2);
    expect(keysOf(tail(ix, 'book', 'place'))).toEqual([S(maker1), S(hookwars.holdingAddr(base, maker1)), S(maker2), S(hookwars.holdingAddr(base, maker2))]);
    const rest = await one('book/place/prepare', { materialId: 2, side: 'bid', price: '9', size: '2' });
    expect(keysOf(tail(rest, 'book', 'place'))).toEqual([S(worst), S(hookwars.holdingAddr(base, worst))]);
  });
  it('the crank passes the owners of expired orders only', async () => {
    const ix = await one('book/crank/prepare', { materialId: 2 });
    expect(keysOf(tail(ix, 'book', 'crank'))).toEqual([S(K(113)), S(hookwars.holdingAddr(hookwars.materialMintAddress(2), K(113)))]);
    expect(ix.data.readUInt8(8)).toBe(1);
  });
});

describe('agents and social from the generated IDLs', () => {
  it('set_directive, commit and post use the IDL discriminators and accounts', () => {
    const passport = K(30);
    const set = hookwars.agentsSetDirective(owner, passport, 1, { maxSpendPerAction: 1n, maxSpendPerDay: 2n, allowedTargets: [], allowedAccessModes: 0, maxLicencePrice: 0n, frozen: false });
    expect([...set.data.subarray(0, 8)]).toEqual(hookwars.coderOf('agents').instruction('set_directive').discriminator);
    expect(named(set, 'agents', 'set_directive').previous).toBe(S(hookwars.directiveAddress(passport, 0)));
    const ref = Buffer.alloc(32, 7);
    expect(named(hookwars.agentsCommit(owner, passport, ref, ref), 'agents', 'commit').commitment).toBe(S(hookwars.commitmentAddress(passport, ref)));
    expect(named(hookwars.agentsPost(owner, passport, K(31), ref), 'agents', 'post').fee_collector).toBe(S(K(31)));
    expect(named(hookwars.socialOpenProfile(owner, K(32)), 'social', 'open_profile').profile).toBe(S(hookwars.profileAddress(K(32))));
  });
  it('the record suffix is the five accounts the programs split', () => {
    const r = hookwars.recordSuffix(hookwars.ARMORY_ID, K(30), owner);
    expect(keysOf(r)).toEqual([S(hookwars.AGENTS_ID), S(hookwars.agentsCallerAddress(hookwars.ARMORY_ID)), S(hookwars.eventAuthorityOf(hookwars.AGENTS_ID)), S(K(30)), S(owner)]);
    expect(r.map((m) => m.isWritable)).toEqual([false, false, false, true, false]);
    expect(keysOf(hookwars.suffixes({ agents: r, social: hookwars.socialSuffix(hookwars.MARKET_ID, owner) })).slice(0, 1)).toEqual([S(hookwars.SOCIAL_ID)]);
  });
});

describe('the runtime gaps: sells (R-1) and the war cranks (R-2)', () => {
  const empty = (): hookwars.SlotData => ({
    kind: 0, equipRule: 0, bounds: { maxCutBps: 0, mayRefuse: false, mayWriteData: false, mayAnswerTouch: false, mayBurn: false }, dataOffset: 0, dataLen: 0,
    item: PublicKey.default, program: PublicKey.default, flags: 0, poolFlags: 0, equipVault: PublicKey.default, signerBump: 0, launchSignerBump: 0, dataEpoch: 0, extraCount: 0,
  });
  const tokenMint = (slots: hookwars.SlotData[]) => hookwars.idlAccountCodec<hookwars.SlotMintData>('token', 'Mint').encode({
    version: 1, decimals: 6, supply: 1n, maxSupply: 0n, mintAuthority: null, freezeAuthority: null, hookAuthority: null, metadataAuthority: null,
    hookProgram: null, hookFlags: 0, name: 'T', symbol: 'T', uri: '', createdAt: 0n, creator: K(9), hookSignerBump: 0, reserved: Buffer.alloc(31),
    slotAuthority: K(8), slotCount: slots.length, slots: [...slots, ...Array.from({ length: 4 - slots.length }, empty)],
  });
  const warMint = K(120), rival = K(121), ordersItem = K(122);
  const launchOf = (m: PublicKey) => enc('launch', 'Launch', { mint: m, quoteMint: QUOTE, lpFeeBps: 100, pool: K(1) });
  const mintKeys = new Set([S(warMint), S(rival)]);
  const war = new Map<string, Buffer>([
    [S(warMint), tokenMint([{ ...empty(), kind: 6, item: ordersItem, program: hookwars.ITEMS_ID }])], [S(rival), tokenMint([])],
    [S(ordersItem), itemData(9, K(123))],
    [S(hookwars.launchAddr(warMint)), launchOf(warMint)], [S(hookwars.launchAddr(rival)), launchOf(rival)],
    [S(hookwars.holdingAddr(rival, owner)), hookwars.holdingCodec.encode({ ...(zeroOf('token', 'Holding') as unknown as Parameters<typeof hookwars.holdingCodec.encode>[0]), mint: rival, owner, amount: 50n })],
  ]);
  const winfo = (k: PublicKey): AccountInfo<Buffer> | null => {
    const data = war.get(S(k)) ?? accounts.get(S(k));
    if (!data) return null;
    return { data, owner: mintKeys.has(S(k)) ? hookwars.TOKEN_ID : hookwars.ARMORY_ID, lamports: 1, executable: false, rentEpoch: 0 };
  };
  const wconn = { getAccountInfo: async (k: PublicKey) => winfo(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(winfo) } as never;
  const build = async (route: string, body: Record<string, unknown>) => {
    const ixs = await PREPARES[route]!.build({ owner: S(owner), ...body }, wconn);
    return ixs[ixs.length - 1]!;
  };

  it('siege names the War orders, the rival launch and its pool, and the chest swap accounts', async () => {
    const ix = await build('war/siege/prepare', { mint: S(warMint), rival: S(rival) });
    const a = named(ix, 'war', 'siege');
    expect([a.orders_item, a.orders_template, a.rival_mint, a.rival_launch, a.rival_war_state]).toEqual([
      S(ordersItem), S(hookwars.templateAddress(9)), S(rival), S(hookwars.launchAddr(rival)), S(hookwars.warStateAddress(rival)),
    ]);
    const rest = keysOf(tail(ix, 'war', 'siege'));
    expect(rest).toContain(S(hookwars.holdingAddr(rival, hookwars.warChestAddress(warMint))));
    expect(rest).toContain(S(hookwars.SWAP_ID));
  });
  it('counter-strike buys its own token into the chest and burns it; raze sells the captured rival', async () => {
    const c = await build('war/counter-strike/prepare', { mint: S(warMint) });
    expect(named(c, 'war', 'counter_strike').launch).toBe(S(hookwars.launchAddr(warMint)));
    expect(keysOf(tail(c, 'war', 'counter_strike'))).toContain(S(hookwars.holdingAddr(warMint, hookwars.warChestAddress(warMint))));
    const r = await build('war/raze/prepare', { mint: S(warMint), rival: S(rival) });
    expect(named(r, 'war', 'raze').rival_launch).toBe(S(hookwars.launchAddr(rival)));
    await expect(build('war/siege/prepare', { mint: S(rival), rival: S(warMint) })).rejects.toThrow(/no War orders/);
  });
  it('sell routes the held tokens into the launch pool and unwraps what it delivered', async () => {
    war.set(S(launchPoolAddress(rival, QUOTE, LP_FEE_BPS)), Buffer.alloc(8));
    const ixs = await PREPARES['sell/prepare']!.build({ owner: S(owner), mint: S(rival), amount: '40', minOut: '1' }, wconn);
    expect(ixs.map((i) => S(i.programId))).toEqual([S(hookwars.TOKEN_ID), S(hookwars.SWAP_ID), PROGRAM_IDS.bridge]);
    await expect(PREPARES['sell/prepare']!.build({ owner: S(owner), mint: S(rival), amount: '60' }, wconn)).rejects.toThrow(/holds 50 base units/);
  });
});

describe('licence buy refuses a market escrow as holder (review 3 L-4)', () => {
  it('a listed item: the holder is the listing escrow, the prepare says so', async () => {
    const escrow = hookwars.marketEscrowAddress(wearing);
    const extra = new Map<string, Buffer>([
      [S(hookwars.accessPolicyAddress(wearItem)), enc('armory', 'AccessPolicy', { item: wearItem, mode: 2, licenceTerms: { priceLamports: 5n, termSecs: 3_600, per: 0, maxLive: 1 }, holderAtSet: holder })],
      [S(hookwars.marketConfigAddress()), enc('market', 'MarketConfig', { admin, treasury })],
    ]);
    const look = (k: PublicKey): AccountInfo<Buffer> | null => { const d = extra.get(S(k)); return d ? { data: d, owner: hookwars.MARKET_ID, lamports: 1, executable: false, rentEpoch: 0 } : info(k); };
    const escrowHolding = hookwars.holdingCodec.encode({ ...(zeroOf('token', 'Holding') as unknown as Parameters<typeof hookwars.holdingCodec.encode>[0]), mint: wearing, owner: escrow, amount: 1n });
    const c = {
      getAccountInfo: async (k: PublicKey) => look(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(look),
      getProgramAccounts: async () => [{ pubkey: hookwars.holdingAddr(wearing, escrow), account: { data: escrowHolding, owner: hookwars.TOKEN_ID, lamports: 1, executable: false, rentEpoch: 0 } }],
    } as never;
    await expect(PREPARES['licences/buy/prepare']!.build({ owner: S(owner), itemMint: S(wearing), tokenMint: S(mint) }, c)).rejects.toThrow(/listed or leased/);
  });
  it('an item that is not Licensed sells no licence (the price comes from its AccessPolicy)', async () => {
    const gated = new Map<string, Buffer>([[S(hookwars.accessPolicyAddress(wearItem)), enc('armory', 'AccessPolicy', { item: wearItem, mode: 1, holderAtSet: holder })], [S(hookwars.marketConfigAddress()), enc('market', 'MarketConfig', { admin, treasury })]]);
    const look = (k: PublicKey): AccountInfo<Buffer> | null => { const d = gated.get(S(k)); return d ? { data: d, owner: hookwars.MARKET_ID, lamports: 1, executable: false, rentEpoch: 0 } : info(k); };
    const c = { getAccountInfo: async (k: PublicKey) => look(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(look), getProgramAccounts: async () => [] } as never;
    await expect(PREPARES['licences/buy/prepare']!.build({ owner: S(owner), itemMint: S(wearing), tokenMint: S(mint) }, c)).rejects.toThrow(/not Licensed/);
  });
});
