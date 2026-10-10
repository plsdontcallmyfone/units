// Changed by Hookwars: new file (app pass v3). Prepares for the hook economy (docs/spec/11, 12
// section 7, 13 section 6) and the runtime gaps R-1 and R-2: item creation with its economy tail
// (author counter, craft wear, social counter), composites, fuse and presets, counters, proposal
// close with the bond guard, crafting and repair, the material order book and class bids, the
// Loyalty Pot reslot, the sell route and the three war cranks. Each reads what it needs from the
// chain and builds with the `@hookwars/sdk` economy builders (generated IDLs); `finish` simulates.
import { PublicKey, type AccountMeta, type Connection, type TransactionInstruction } from '@solana/web3.js';
import { hookwars, token, bridge, launch as upLaunch, launchKeysOf, decodeLaunch, launchPoolAddress, launchHookExtras } from '@hookwars/sdk';
import { FIXED_ADDRESSES, LP_FEE_BPS, PROGRAM_IDS } from '@hookwars/shared';
import { big, int, pk, PrepareError, type Body, type PrepareDef } from './prepares.ts';

const QUOTE = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
const PARAMS = 11;
const U32_MAX = 4_294_967_295;
const one = (label: string, programs: PrepareDef['programs'], build: PrepareDef['build']): PrepareDef => ({ programs, label, payer: (b) => pk(b, 'owner'), build });

async function need<T>(conn: Connection, address: PublicKey, decode: (d: Buffer) => T, missing: string): Promise<T> {
  const info = await conn.getAccountInfo(address, 'confirmed');
  if (!info) throw new PrepareError(409, 'NotFound', missing);
  return decode(info.data);
}
const exists = async (conn: Connection, address: PublicKey): Promise<boolean> => (await conn.getAccountInfo(address, 'confirmed')) !== null;

const armoryConfig = (conn: Connection) => need(conn, hookwars.armoryConfigAddress(), (d) => hookwars.armoryConfigCodec.decode(d), 'The armory has no config on this cluster yet.');
const template = (conn: Connection, id: number) => need(conn, hookwars.templateAddress(id), (d) => hookwars.templateCodec().decode(d), `Template ${id} is not registered on this cluster.`);
const item = (conn: Connection, itemMint: PublicKey) => need(conn, hookwars.itemAddress(itemMint), (d) => hookwars.itemCodec().decode(d), 'No armory item has this mint.');

/** Up to `PARAMS` integers, zero-filled. */
function params(b: Body, k: string): number[] {
  const v = b[k];
  if (!Array.isArray(v) || v.length > PARAMS || v.some((x) => !Number.isInteger(x) || x < 0 || x > U32_MAX)) throw new PrepareError(400, 'BadRequest', `"${k}" must be a list of at most ${PARAMS} integers.`);
  return [...(v as number[]), ...Array<number>(PARAMS - v.length).fill(0)];
}

function modules(b: Body): hookwars.ModuleInput[] {
  const v = b.modules;
  if (!Array.isArray(v) || v.length < 2 || v.length > 6) throw new PrepareError(400, 'BadRequest', '"modules" must be a list of 2 to 6 modules.');
  return (v as Body[]).map((m) => ({
    templateId: int(m, 'templateId', 1, 65_535), params: params(m, 'params'), targetStart: int(m, 'targetStart', 0, 255), targetCount: int(m, 'targetCount', 0, 255),
    dataBytes: m.dataBytes === undefined ? 0 : int(m, 'dataBytes', 0, 255), readsModule: m.readsModule === undefined ? 255 : int(m, 'readsModule', 0, 255),
  }));
}

/** The economy tail every item creation may carry: the author's counter when it exists, the craft
 * wear when the template wears, the social counter when the author has a profile. */
async function createExtras(conn: Connection, author: PublicKey, templateId: number): Promise<hookwars.CreateExtras> {
  const [t, counter, profile] = await Promise.all([template(conn, templateId), exists(conn, hookwars.authorCounterAddress(author)), exists(conn, hookwars.profileAddress(author))]);
  return { counter, wear: t.chargesOnCreate > 0, social: profile };
}

/** The holder of an item (a one-unit mint): the owner of its only funded holding. */
export async function itemHolder(conn: Connection, itemMint: PublicKey): Promise<PublicKey | null> {
  const res = await conn.getProgramAccounts(hookwars.TOKEN_ID, { commitment: 'confirmed', filters: [
    { memcmp: { offset: 0, bytes: Buffer.from(hookwars.holdingCodec.disc).toString('base64'), encoding: 'base64' } }, { memcmp: { offset: 10, bytes: itemMint.toBase58() } },
  ] });
  for (const r of res) {
    try { const h = hookwars.holdingCodec.decode(r.account.data); if (h.amount > 0n) return h.owner; } catch { /* not a holding */ }
  }
  return null;
}

/**
 * The economy suffixes of `settle_equip` for one slot (12 I-7, 13 E-2 to E-4), each only when it
 * applies: the lease suffix when an Active lease names this token and slot, the craft suffix when
 * the item wears and has run since the last settle, the fee suffix when the protocol or the
 * template's author takes a share.
 */
export async function settleTail(conn: Connection, mint: PublicKey, slot: number, st: { item: PublicKey; runs: bigint; runsAtSettle: bigint }, it: { itemMint: PublicKey; templateId: number; hasWear: boolean }): Promise<AccountMeta[]> {
  const leaseKey = hookwars.leaseAddress(st.item);
  const [leaseInfo, cfg, t] = await Promise.all([conn.getAccountInfo(leaseKey, 'confirmed'), armoryConfig(conn), template(conn, it.templateId)]);
  const s: hookwars.Suffixes = {};
  if (leaseInfo) {
    const l = hookwars.leaseCodec.decode(leaseInfo.data);
    if (l.state === 1 && l.tokenMint.equals(mint) && l.slot === slot) s.rent = hookwars.rentSuffix(st.item, l.lessor, mint, QUOTE);
  }
  if (it.hasWear && st.runs > st.runsAtSettle) {
    const holder = await itemHolder(conn, it.itemMint);
    if (!holder) throw new PrepareError(409, 'NoHolder', 'The equipped item has no holder to receive its drop.');
    const rule = await conn.getAccountInfo(hookwars.dropRuleAddress(hookwars.DROP_SOURCE.settleCrank), 'confirmed');
    const materialId = rule ? hookwars.dropRuleCodec.decode(rule.data).materialId : 0;
    s.craft = hookwars.settleCraftSuffix({ item: st.item, itemMint: it.itemMint, recipient: holder, materialId });
  }
  if (cfg.itemProtocolBps > 0 || t.authorBps > 0) s.fee = hookwars.feeSuffix(it.templateId, cfg.admin, t.registeredBy, mint, QUOTE);
  return hookwars.suffixes(s);
}

/**
 * The accounts of the slot revert that market `end_lease` CPIs (12 section 3): the leased item out,
 * the slot's launch item back in. Null when the slot no longer holds the leased item.
 */
export async function leaseRevert(conn: Connection, payer: PublicKey, lease: { item: PublicKey; tokenMint: PublicKey; slot: number }): Promise<AccountMeta[] | null> {
  const mint = lease.tokenMint; const slot = lease.slot;
  const [ssInfo, esInfo] = await conn.getMultipleAccountsInfo([hookwars.slotStateAddress(mint, slot), hookwars.equipStateAddress(mint, slot)], 'confirmed');
  if (!ssInfo || !esInfo) return null;
  const es = hookwars.equipStateCodec.decode(esInfo.data);
  if (!es.item.equals(lease.item)) return null;
  const ss = hookwars.slotStateCodec.decode(ssInfo.data);
  const oldItem = await need(conn, lease.item, (d) => hookwars.itemCodec().decode(d), 'The leased item no longer exists.');
  const change: hookwars.EquipChange = { oldItem: lease.item, oldEquipVault: (oldItem.manifest.tokenFlags & 64) !== 0 ? hookwars.equipVault(mint, slot) : null };
  const next = ss.launchItem;
  if (next) {
    const ni = await need(conn, next, (d) => hookwars.itemCodec().decode(d), 'The slot\'s launch item no longer exists.');
    const t = await template(conn, ni.templateId);
    const owner = hookwars.royaltyOwner(next);
    Object.assign(change, {
      newItem: next, newTemplate: hookwars.templateAddress(ni.templateId), templateProgram: t.program, templateProgramdata: programData(t.program),
      registry: hookwars.itemRegistryAddress(mint, next), newEquipVault: (ni.manifest.tokenFlags & 64) !== 0 ? hookwars.equipVault(mint, slot) : null,
      royaltyOwner: owner, royaltyHoldingToken: hookwars.holdingAddr(mint, owner), quoteMint: QUOTE, royaltyHoldingQuote: hookwars.holdingAddr(QUOTE, owner),
      newComposite: ni.templateId === hookwars.COMPOSITE_TEMPLATE_ID ? hookwars.compositeAddress(next) : null,
    } satisfies hookwars.EquipChange);
  }
  return hookwars.revertForLeaseEndMetas(payer, mint, slot, lease.item, change);
}

const UPGRADEABLE_LOADER = new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
const programData = (program: PublicKey): PublicKey => PublicKey.findProgramAddressSync([program.toBuffer()], UPGRADEABLE_LOADER)[0];

/** A launch's keys (pool, kit modules) for the chest swaps the war steps make. */
async function launchKeys(conn: Connection, mint: PublicKey) {
  const info = await conn.getAccountInfo(hookwars.launchAddr(mint), 'confirmed');
  if (!info) throw new PrepareError(404, 'NoSuchToken', 'This token has no launch on this cluster.');
  const l = decodeLaunch(info.data);
  return { l, keys: launchKeysOf(l), pool: launchPoolAddress(mint, QUOTE, LP_FEE_BPS) };
}

/** The token-hook slices of one side of a launch-pool swap whose counterparty is `owner`. */
async function sideSlices(conn: Connection, mint: PublicKey, pool: PublicKey, owner: PublicKey, sell: boolean): Promise<AccountMeta[]> {
  const vault = hookwars.holdingAddr(mint, pool); const mine = hookwars.holdingAddr(mint, owner);
  const s = await hookwars.fetchTokenHookSlices(conn, sell
    ? { mint, source: mine, destination: vault, authority: owner, sourceOwner: owner, destinationOwner: pool }
    : { mint, source: vault, destination: mine, authority: pool, sourceOwner: pool, destinationOwner: owner });
  if (!s) throw new PrepareError(404, 'NoSuchToken', 'This is not a token on this cluster.');
  return hookwars.sliceAccounts(s);
}

/** The War orders of `mint`, required by every war step. */
async function orders(conn: Connection, mint: PublicKey): Promise<hookwars.Orders> {
  const ctx = await hookwars.fetchWarContext(conn, mint).catch(() => null);
  if (!ctx) throw new PrepareError(404, 'NoSuchToken', 'This is not a token on this cluster.');
  if (!ctx.orders) throw new PrepareError(409, 'NoWarOrders', 'This token has no War orders equipped, so it takes no war steps.');
  return ctx.orders;
}

/** The instructions a chest swap on `poolMint`'s launch pool makes (their accounts; war client `chest_swap_inner`). */
function chestSwapInner(keys: ReturnType<typeof launchKeysOf>, chest: PublicKey, poolMint: PublicKey, cranker: PublicKey, sell: boolean): TransactionInstruction[] {
  return [upLaunch.swap(keys, chest, chest, sell ? 0 : 1, 1n, 0n), token.createHolding(cranker, poolMint, chest), bridge.unwrapSol(chest, 0n)];
}

function side(b: Body): number {
  const v = b.side;
  if (v === 'bid' || v === 0) return hookwars.BOOK_SIDE.bid;
  if (v === 'ask' || v === 1) return hookwars.BOOK_SIDE.ask;
  throw new PrepareError(400, 'BadRequest', '"side" is "bid" or "ask".');
}

async function bookMarket(conn: Connection, b: Body) {
  const baseMint = b.baseMint !== undefined ? pk(b, 'baseMint') : hookwars.materialMintAddress(int(b, 'materialId', 0, 65_535));
  const m = await need(conn, hookwars.bookMarketAddress(baseMint), (d) => hookwars.bookMarketCodec.decode(d), 'There is no order book for this material yet.');
  return { baseMint, m };
}
const bookConfig = (conn: Connection) => need(conn, hookwars.bookConfigAddress(), (d) => hookwars.bookConfigCodec.decode(d), 'The order book has no config on this cluster yet.');
const craftConfig = (conn: Connection) => need(conn, hookwars.craftConfigAddress(), (d) => hookwars.craftConfigCodec.decode(d), 'Craft has no config on this cluster yet.');
const nowSecs = (): bigint => BigInt(Math.floor(Date.now() / 1000));
const ref = (b: Body): Buffer => {
  const v = b.reference;
  if (v === undefined) return Buffer.alloc(32);
  if (typeof v !== 'string' || !/^[0-9a-f]{64}$/.test(v)) throw new PrepareError(400, 'BadRequest', '"reference" must be 64 hex characters.');
  return Buffer.from(v, 'hex');
};

export const ECONOMY_PREPARES: Record<string, PrepareDef> = {
  // ---------------------------------------------------------------- armory items (12, 13) --
  'items/create/prepare': one('Create an item', ['armory', 'items', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const templateId = int(b, 'templateId', 1, 65_535);
    const [cfg, extras] = await Promise.all([armoryConfig(conn), createExtras(conn, owner, templateId)]);
    if (extras.wear && !(await exists(conn, hookwars.craftConfigAddress()))) throw new PrepareError(409, 'NoCraft', 'This template wears, and craft has no config on this cluster yet.');
    return [hookwars.createItemEco(owner, templateId, params(b, 'params'), int(b, 'royaltyBps', 0, 10_000), cfg.itemsMinted, extras)];
  }),
  'items/composite/prepare': one('Create a composite', ['armory', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner');
    const [cfg, extras] = await Promise.all([armoryConfig(conn), createExtras(conn, owner, hookwars.COMPOSITE_TEMPLATE_ID)]);
    return [hookwars.createComposite(owner, modules(b), int(b, 'royaltyBps', 0, 10_000), cfg.itemsMinted, extras)];
  }),
  'items/fuse/prepare': one('Fuse items', ['armory', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner');
    const list = b.components;
    if (!Array.isArray(list) || list.length < 2 || list.length > 6) throw new PrepareError(400, 'BadRequest', '"components" must be a list of 2 to 6 held items.');
    const components: hookwars.FuseComponent[] = [];
    for (const c of list as Body[]) {
      const itemMint = pk(c, 'itemMint');
      const it = await item(conn, itemMint);
      if (it.templateId === hookwars.COMPOSITE_TEMPLATE_ID) throw new PrepareError(409, 'IsComposite', 'A composite cannot be fused again.');
      if (it.equippedCount > 0) throw new PrepareError(409, 'ItemEquipped', 'An equipped item cannot be fused; it must be unequipped first.');
      components.push({ item: hookwars.itemAddress(itemMint), itemMint, templateId: it.templateId, start: int(c, 'start', 0, 255), count: int(c, 'count', 0, 255) });
    }
    const [cfg, extras] = await Promise.all([armoryConfig(conn), createExtras(conn, owner, hookwars.COMPOSITE_TEMPLATE_ID)]);
    return [hookwars.fuse(owner, components, int(b, 'royaltyBps', 0, 10_000), cfg.itemsMinted, extras)];
  }),
  'presets/mint/prepare': one('Mint a preset composite', ['armory', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const presetId = int(b, 'presetId', 0, 65_535);
    const preset = await need(conn, hookwars.presetAddress(presetId), (d) => hookwars.presetCodec.decode(d), 'No such preset.');
    const mods = modules(b);
    if (mods.length !== preset.templateIds.length || mods.some((m, i) => m.templateId !== preset.templateIds[i])) throw new PrepareError(400, 'BadRequest', `The modules must be the preset's templates in its order: ${preset.templateIds.join(', ')}.`);
    const [cfg, extras] = await Promise.all([armoryConfig(conn), createExtras(conn, owner, hookwars.COMPOSITE_TEMPLATE_ID)]);
    return [hookwars.mintComposite(owner, presetId, mods, int(b, 'royaltyBps', 0, 10_000), cfg.itemsMinted, extras)];
  }),
  'counters/init/prepare': one('Open author and claim counters', ['armory'], async (b) => [hookwars.initCounters(pk(b, 'owner'), b.wallet === undefined ? pk(b, 'owner') : pk(b, 'wallet'))]),
  'proposals/close/prepare': one('Close the proposal', ['armory'], async (b, conn) => {
    const mint = pk(b, 'mint'); const slot = int(b, 'slot', 0, 255); const nonce = big(b, 'nonce');
    const proposal = hookwars.proposalAddress(mint, slot, nonce);
    const p = await need(conn, proposal, (d) => hookwars.proposalCodec.decode(d), 'No such proposal.');
    const mark = await conn.getAccountInfo(hookwars.bondMarkAddress(proposal), 'confirmed');
    const bond = mark ? hookwars.bondMarkCodec.decode(mark.data).bond : null;
    return [hookwars.closeProposal(p.proposer, mint, slot, nonce, bond)];
  }),
  'loyalty/reslot/prepare': one('Move the loyalty pot', ['items'], async (b, conn) => {
    const mint = pk(b, 'mint'); const slot = int(b, 'slot', 0, 7);
    const pot = await need(conn, hookwars.loyaltyPotAddress(mint), (d) => hookwars.loyaltyPotCodec.decode(d), 'This token has no loyalty pot.');
    const [oldSt, newSt] = await conn.getMultipleAccountsInfo([hookwars.equipStateAddress(mint, pot.slot), hookwars.equipStateAddress(mint, slot)], 'confirmed');
    if (!newSt) throw new PrepareError(409, 'NotEquipped', 'Nothing is equipped in that slot.');
    const oldItem = oldSt ? hookwars.equipStateCodec.decode(oldSt.data).item : hookwars.ITEMS_ID;
    return [hookwars.reslotLoyalty(mint, pot.slot, oldItem, slot, hookwars.equipStateCodec.decode(newSt.data).item)];
  }),

  // ---------------------------------------------------------------- craft (11 section 5) --
  'craft/prepare': one('Craft', ['craft', 'armory', 'items', 'social', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const recipeId = int(b, 'recipeId', 0, 65_535);
    const recipe = await need(conn, hookwars.recipeAddress(recipeId), (d) => hookwars.recipeCodec.decode(d), 'No such recipe.');
    if (recipe.terms.kind !== 0) throw new PrepareError(409, 'NotACraft', 'This recipe repairs; it makes nothing.');
    if (!recipe.terms.active) throw new PrepareError(409, 'Inactive', 'This recipe is not active.');
    const [cfg, armory, t] = await Promise.all([craftConfig(conn), armoryConfig(conn), template(conn, recipe.terms.templateId)]);
    return [hookwars.craftItem(owner, {
      recipeId, templateId: recipe.terms.templateId, materialIds: recipe.terms.inputs.map((x) => x.materialId), treasury: cfg.treasury, seasonPool: cfg.seasonPool,
      itemsMinted: armory.itemsMinted, wear: t.chargesOnCreate > 0, reference: ref(b),
    })];
  }),
  'craft/repair/prepare': one('Repair', ['craft', 'social', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const recipeId = int(b, 'recipeId', 0, 65_535); const itemMint = pk(b, 'itemMint');
    const recipe = await need(conn, hookwars.recipeAddress(recipeId), (d) => hookwars.recipeCodec.decode(d), 'No such recipe.');
    if (recipe.terms.kind !== 1) throw new PrepareError(409, 'NotARepair', 'This recipe crafts; it repairs nothing.');
    const it = await item(conn, itemMint);
    if (it.templateId !== recipe.terms.templateId) throw new PrepareError(409, 'WrongTemplate', `This recipe repairs template ${recipe.terms.templateId} only.`);
    if (!it.hasWear) throw new PrepareError(409, 'NoWear', 'This item does not wear, so it needs no repair.');
    const cfg = await craftConfig(conn);
    return [hookwars.repairItem(owner, { recipeId, item: hookwars.itemAddress(itemMint), itemMint, materialIds: recipe.terms.inputs.map((x) => x.materialId), treasury: cfg.treasury, seasonPool: cfg.seasonPool, reference: ref(b) })];
  }),

  // ---------------------------------------------------------------- book (11 section 6) --
  'book/place/prepare': one('Place an order', ['book', 'social', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const s = side(b); const price = big(b, 'price'); const size = big(b, 'size');
    const [{ baseMint, m }, cfg] = await Promise.all([bookMarket(conn, b), bookConfig(conn)]);
    const expiresAt = b.expiresAt === undefined ? 0n : big(b, 'expiresAt');
    const makers = hookwars.bookMakers(m, cfg.params, s, price, size, nowSecs());
    return [token.createHolding(owner, baseMint, owner), hookwars.bookPlace(owner, { baseMint, treasury: cfg.treasury, side: s, price, size, postOnly: b.postOnly === true, expiresAt, makers, reference: ref(b) })];
  }),
  'book/cancel/prepare': one('Cancel an order', ['book', 'token'], async (b, conn) => {
    const { baseMint } = await bookMarket(conn, b);
    return [hookwars.bookCancel(pk(b, 'owner'), baseMint, big(b, 'id'))];
  }),
  'book/crank/prepare': one('Clear expired orders', ['book', 'token'], async (b, conn) => {
    const { baseMint, m } = await bookMarket(conn, b);
    const max = b.max === undefined ? 8 : int(b, 'max', 1, 32);
    const owners = hookwars.bookCrankOwners(m, max, nowSecs());
    if (owners.length === 0) throw new PrepareError(409, 'NothingExpired', 'No order on this book has expired.');
    return [hookwars.bookCrank(pk(b, 'owner'), baseMint, owners.length, owners)];
  }),
  'book/market/prepare': one('Open an order book', ['book', 'craft', 'token'], async (b) => [hookwars.bookCreateMarket(pk(b, 'owner'), int(b, 'materialId', 0, 65_535), big(b, 'tickLamports'), big(b, 'minSize'))]),
  'book/class-bid/prepare': one('Bid for a class of items', ['book'], async (b) => {
    const cls = b.class as Body | undefined;
    if (!cls || typeof cls !== 'object') throw new PrepareError(400, 'BadRequest', '"class" is required: { templateId, minLevel, paramMin, paramMax }.');
    return [hookwars.bookPlaceClassBid(pk(b, 'owner'), big(b, 'nonce'), { templateId: int(cls, 'templateId', 1, 65_535), minLevel: int(cls, 'minLevel', 0, 255), paramMin: params(cls, 'paramMin'), paramMax: params(cls, 'paramMax') }, big(b, 'price'), b.expiresAt === undefined ? 0n : big(b, 'expiresAt'))];
  }),
  'book/class-bid/cancel/prepare': one('Cancel a class bid', ['book'], async (b) => [hookwars.bookCancelClassBid(pk(b, 'owner'), big(b, 'nonce'))]),
  'book/class-bid/match/prepare': one('Sell into a class bid', ['book', 'token'], async (b, conn) => {
    const bid = pk(b, 'bid'); const itemMint = pk(b, 'itemMint');
    const [cb, cfg] = await Promise.all([need(conn, bid, (d) => hookwars.classBidCodec.decode(d), 'No such class bid.'), bookConfig(conn)]);
    return [hookwars.bookMatchClass(pk(b, 'owner'), { bid, bidder: cb.bidder, item: hookwars.itemAddress(itemMint), itemMint, treasury: cfg.treasury, reference: ref(b) })];
  }),

  // ---------------------------------------------------------------- R-1: sells --
  'sell/prepare': one('Sell', ['swap', 'launch', 'items', 'token', 'bridge'], async (b, conn) => {
    const owner = pk(b, 'owner'); const mint = pk(b, 'mint');
    const amount = big(b, 'amount'); const minOut = b.minOut === undefined ? 0n : big(b, 'minOut');
    if (amount <= 0n) throw new PrepareError(400, 'BadRequest', '"amount" must be above zero.');
    const pool = launchPoolAddress(mint, QUOTE, LP_FEE_BPS);
    const [pi, hi] = await conn.getMultipleAccountsInfo([pool, hookwars.holdingAddr(mint, owner)], 'confirmed');
    if (!pi) throw new PrepareError(404, 'NoSuchToken', 'This is not a launch on this cluster.');
    const held = hi ? hookwars.holdingCodec.decode(hi.data).amount : 0n;
    if (held < amount) throw new PrepareError(409, 'NotEnough', `This wallet holds ${held} base units of this token, less than ${amount}.`);
    const slices = await sideSlices(conn, mint, pool, owner, true);
    const items = await hookwars.fetchPoolItems(conn, mint);
    const hop: hookwars.RouteHop = {
      pool, baseMint: mint, quoteMint: QUOTE, traderBase: hookwars.holdingAddr(mint, owner), traderQuote: hookwars.holdingAddr(QUOTE, owner),
      hookProgram: new PublicKey(PROGRAM_IDS.launch), baseMintWritable: true, quoteMintWritable: false, direction: 0,
      inSlice: slices, outSlice: [], poolExtras: hookwars.slotPoolExtras(mint, QUOTE, launchHookExtras(mint, QUOTE), items.accounts),
    };
    // The quote arrives as bridged SOL; what the sell delivered is unwrapped back to SOL.
    const before = (await conn.getAccountInfo(hookwars.holdingAddr(QUOTE, owner), 'confirmed'));
    const keep = before ? hookwars.holdingCodec.decode(before.data).amount : 0n;
    return [token.createHolding(owner, QUOTE, owner), hookwars.swapRoute(owner, amount, minOut, [hop]), bridge.unwrapSolAbove(owner, keep)];
  }),

  // ---------------------------------------------------------------- R-2: war cranks (05 sections 7 to 9) --
  'war/siege/prepare': one('Siege', ['war', 'swap', 'launch', 'items', 'token', 'bridge'], async (b, conn) => {
    const cranker = pk(b, 'owner'); const mint = pk(b, 'mint'); const rival = pk(b, 'rival');
    const o = await orders(conn, mint);
    const chest = hookwars.warChestAddress(mint);
    const r = await launchKeys(conn, rival);
    const slice = await sideSlices(conn, rival, r.pool, chest, false);
    return [hookwars.siege(cranker, mint, o, rival, r.pool, true, r.l.modules !== 0 ? r.l.kitConfig : null, slice, chestSwapInner(r.keys, chest, rival, cranker, false))];
  }),
  'war/counter-strike/prepare': one('Counter-strike', ['war', 'swap', 'launch', 'items', 'token', 'bridge'], async (b, conn) => {
    const cranker = pk(b, 'owner'); const mint = pk(b, 'mint');
    const o = await orders(conn, mint);
    const chest = hookwars.warChestAddress(mint);
    const own = await launchKeys(conn, mint);
    const buySlice = await sideSlices(conn, mint, own.pool, chest, false);
    const chestHolding = hookwars.holdingAddr(mint, chest);
    const burn = await hookwars.fetchTokenHookSlices(conn, { mint, source: chestHolding, destination: chestHolding, authority: chest, sourceOwner: chest, destinationOwner: chest, op: 'burn' });
    const burnSlice = burn ? hookwars.sliceAccounts(burn) : [];
    const inner = [...chestSwapInner(own.keys, chest, mint, cranker, false), token.burn(chest, chestHolding, mint, 1n)];
    return [hookwars.counterStrike(cranker, mint, o, own.pool, own.l.modules !== 0 ? own.l.kitConfig : null, buySlice, burnSlice, inner)];
  }),
  'war/raze/prepare': one('Raze', ['war', 'swap', 'launch', 'items', 'token', 'bridge'], async (b, conn) => {
    const cranker = pk(b, 'owner'); const mint = pk(b, 'mint'); const rival = pk(b, 'rival');
    const o = await orders(conn, mint);
    const chest = hookwars.warChestAddress(mint);
    const r = await launchKeys(conn, rival);
    const slice = await sideSlices(conn, rival, r.pool, chest, true);
    return [hookwars.raze(cranker, mint, o, rival, r.pool, slice, chestSwapInner(r.keys, chest, rival, cranker, true))];
  }),

  // ---------------------------------------------------------------- admin setters (13 E-2, E-7; wave F) --
  'armory/template-economy/prepare': one('Set template economy', ['armory'], async (b) => [hookwars.setTemplateEconomy(
    pk(b, 'owner'), int(b, 'templateId', 1, 65_535), int(b, 'authorBps', 0, 10_000), int(b, 'defaultAccess', 0, 255), int(b, 'allowedAccess', 0, 255), int(b, 'chargesOnCreate', 0, U32_MAX),
  )]),
  'armory/protocol-bps/prepare': one('Set the item protocol fee', ['armory'], async (b) => [hookwars.setItemProtocolBps(pk(b, 'owner'), int(b, 'itemProtocolBps', 0, 10_000))]),
  'presets/register/prepare': one('Register a preset', ['armory'], async (b) => {
    const ids = b.templateIds;
    if (!Array.isArray(ids) || ids.length < 2 || ids.length > 6 || ids.some((x) => !Number.isInteger(x) || x < 1 || x > 65_535)) throw new PrepareError(400, 'BadRequest', '"templateIds" must be 2 to 6 template ids.');
    const name = b.name;
    if (typeof name !== 'string' || name.length === 0 || name.length > 32) throw new PrepareError(400, 'BadRequest', '"name" must be 1 to 32 characters.');
    return [hookwars.registerPreset(pk(b, 'owner'), int(b, 'id', 0, 65_535), ids as number[], name)];
  }),
};
