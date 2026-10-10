// Changed by Hookwars: staged slot launch, raid and settle prepares; v0 with lookup tables (the protocol's and the mint's own); input caps (integration, app audit A-4, A-6, A-12); agents, market, social and arsenal payout prepares.
/**
 * The prepare routes of docs/spec/06-app.md 3.3: build with the SDK, simulate, return unsigned v0
 * transactions for the wallet (the backend holds no user key, 06 section 1 rule 5). Every prepare
 * first checks that the programs it calls exist on the cluster; until they are deployed it refuses
 * with one sentence that says so, rather than returning a transaction that cannot land.
 */
import {
  AddressLookupTableAccount, AddressLookupTableProgram, ComputeBudgetProgram, Connection, PublicKey, TransactionMessage, VersionedTransaction,
  type AccountMeta, type TransactionInstruction,
} from '@solana/web3.js';
import { hookwars, decodeLaunch, decodeKitConfig, decodePool, decodeLaunchConfig, launchHookExtras, launchPoolAddress, launchRulesFromInput, checkProtocolLookupTable, token, LAUNCH_CONFIG } from '@hookwars/sdk';
import type { Pool as Db } from 'pg';
import { EXPANSION_PREPARES } from './expansion-prepares.ts';
import { FIXED_ADDRESSES, LP_FEE_BPS, remainderBuy, MAX_VIRTUAL_QUOTE, MIN_VIRTUAL_QUOTE, NO_RULES, PROGRAM_IDS, TEMPLATES, type LaunchRulesInput, type PreparedTx } from '@hookwars/shared';

export class PrepareError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) { super(message); this.status = status; this.code = code; }
}

export type Body = Record<string, unknown>;
export const pk = (b: Body, k: string): PublicKey => {
  const v = b[k];
  if (typeof v !== 'string') throw new PrepareError(400, 'BadRequest', `"${k}" is required.`);
  try { return new PublicKey(v); } catch { throw new PrepareError(400, 'BadRequest', `"${k}" is not an address.`); }
};
const U64_MAX = (1n << 64n) - 1n;
export const big = (b: Body, k: string): bigint => {
  let v: bigint;
  try { v = BigInt(String(b[k])); } catch { throw new PrepareError(400, 'BadRequest', `"${k}" must be an integer amount.`); }
  if (v < 0n || v > U64_MAX) throw new PrepareError(400, 'BadRequest', `"${k}" must be from 0 to 2^64 - 1.`);
  return v;
};

/** An integer in [lo, hi]; anything else is a 400 (audit A-12). */
export const int = (b: Body, k: string, lo: number, hi: number): number => {
  const v = Number(b[k]);
  if (!Number.isInteger(v) || v < lo || v > hi) throw new PrepareError(400, 'BadRequest', `"${k}" must be an integer from ${lo} to ${hi}.`);
  return v;
};
/** An array of addresses of at most `max` entries (audit A-4). */
export const keys = (b: Body, k: string, max: number): PublicKey[] => {
  const v = b[k];
  if (v === undefined) return [];
  if (!Array.isArray(v) || v.length > max) throw new PrepareError(400, 'BadRequest', `"${k}" must be a list of at most ${max} addresses.`);
  return v.map((x) => { try { return new PublicKey(String(x)); } catch { throw new PrepareError(400, 'BadRequest', `"${k}" holds something that is not an address.`); } });
};
/** Item targets per equip: `Template.max_targets` is a u8, so no template takes more than 255. */
const MAX_TARGETS = 255;
/** Treaty pairs per call: each pair names two accounts and a transaction locks at most 64. */
const MAX_PAIRS = 32;

/** One staged transaction before compiling: its instructions, the keys the browser signs with, and whether it can be simulated now. */
export interface Stage { label: string; ixs: TransactionInstruction[]; extraSigners: ('mint' | 'config')[]; simulate: boolean; tables: AddressLookupTableAccount[] }

export interface PrepareDef {
  programs: (keyof typeof PROGRAM_IDS)[];
  label: string;
  payer: (b: Body) => PublicKey;
  build: (b: Body, conn: Connection) => Promise<TransactionInstruction[]>;
  /** A flow of several transactions that must land in order (a launch). */
  staged?: (b: Body, conn: Connection) => Promise<Stage[]>;
}

/** Route path (after /v1) to its builder. */
export const PREPARES: Record<string, PrepareDef> = {
  'votes/prepare': {
    programs: ['armory', 'token'], label: 'Vote', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.vote(pk(b, 'owner'), pk(b, 'mint'), int(b, 'slot', 0, 255), big(b, 'nonce'), Boolean(b.support), big(b, 'amount'))],
  },
  'proposals/prepare': {
    programs: ['armory'], label: 'Propose', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const mint = pk(b, 'mint'); const slot = int(b, 'slot', 0, 255);
      const ss = await conn.getAccountInfo(hookwars.slotStateAddress(mint, slot), 'confirmed');
      if (!ss) throw new PrepareError(409, 'NoSlotState', 'This slot has no state yet: it was not filled at launch, so it takes no proposals.');
      const state = hookwars.slotStateCodec.decode(ss.data);
      const targets = keys(b, 'targets', MAX_TARGETS);
      const role = ({ none: 0, pay: 1, receive: 2 } as Record<string, number>)[String(b.role ?? 'none')] ?? 0;
      return [hookwars.propose(pk(b, 'owner'), mint, slot, state.nextNonce, b.item ? pk(b, 'item') : null, targets, role)];
    },
  },
  'settle/prepare': {
    programs: ['items', 'token'], label: 'Settle', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const mint = pk(b, 'mint'); const slot = int(b, 'slot', 0, 7);
      const st = await conn.getAccountInfo(hookwars.equipStateAddress(mint, slot), 'confirmed');
      if (!st) throw new PrepareError(409, 'NotEquipped', 'Nothing is equipped in this slot.');
      const state = hookwars.equipStateCodec.decode(st.data) as { item: PublicKey; config: { targets: PublicKey[] } };
      const itemInfo = await conn.getAccountInfo(state.item, 'confirmed');
      if (!itemInfo) throw new PrepareError(409, 'NoItem', 'The equipped item no longer exists.');
      const item = hookwars.itemCodec().decode(itemInfo.data) as { templateId: number; manifest: { tokenFlags: number } };
      const composite = item.templateId === 41;
      const modules: { templateId: number; targets: PublicKey[] }[] = composite
        ? (await compositeModules(conn, state.item)).map((m) => ({ templateId: m.templateId, targets: state.config.targets.slice(m.targetStart, m.targetStart + m.targetCount) }))
        : [{ templateId: item.templateId, targets: state.config.targets }];
      const quote = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
      const dests = modules.map((m) => hookwars.settleDestination(m.templateId, mint, quote, m.targets));
      const tokenCuts = (item.manifest.tokenFlags & 64) !== 0;
      return [hookwars.settleEquip(owner, mint, slot, { key: state.item, tokenCuts, composite }, quote, dests)];
    },
  },
  'royalties/prepare': {
    programs: ['armory', 'token'], label: 'Claim royalty', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.claimRoyalty(pk(b, 'owner'), pk(b, 'item'), pk(b, 'itemMint'), pk(b, 'cutMint'), big(b, 'amount'))],
  },
  'bounties/prepare': {
    programs: ['war', 'token', 'items'], label: 'Claim bounty', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const mint = pk(b, 'mint');
      const { orders, raid, extras } = await raidContext(conn, mint, owner);
      if (!orders) throw new PrepareError(409, 'NoWarOrders', 'This token has no War orders equipped, so its chest pays no bounties.');
      return [hookwars.claimBounty(owner, mint, orders, raid.slot, extras)];
    },
  },
  'quests/prepare': {
    programs: ['war', 'token', 'items'], label: 'Claim quest', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const mint = pk(b, 'mint');
      const { raid, extras } = await raidContext(conn, mint, owner);
      return [hookwars.claimQuest(owner, mint, int(b, 'season', 0, 4_294_967_295), int(b, 'questId', 1, 2) === 2 ? 2 : 1, int(b, 'period', 0, 4_294_967_295), raid.slot, extras)];
    },
  },
  'rolls/prepare': {
    programs: ['war', 'token', 'items'], label: 'Roll', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const mint = pk(b, 'mint');
      const cfgInfo = await conn.getAccountInfo(hookwars.WAR_CONFIG, 'confirmed');
      if (!cfgInfo) throw new PrepareError(409, 'NoWarConfig', 'The war program has no config on this cluster yet.');
      const cfg = hookwars.warConfigCodec.decode(cfgInfo.data);
      const { raid, extras } = await raidContext(conn, mint, owner);
      return [hookwars.roll(owner, mint, big(b, 'nonce'), raid.slot, { program: cfg.randomnessProgram, account: pk(b, 'oracleAccount') }, extras)];
    },
  },
  'war/init/prepare': {
    programs: ['war'], label: 'Open the war chest', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.initWar(pk(b, 'owner'), pk(b, 'mint'))],
  },
  'forge/prepare': {
    programs: ['armory', 'items', 'token'], label: 'Forge', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner');
      const [ai, bi, cfg] = await conn.getMultipleAccountsInfo([pk(b, 'itemA'), pk(b, 'itemB'), hookwars.armoryConfigAddress()], 'confirmed');
      if (!ai || !bi) throw new PrepareError(404, 'NoSuchItem', 'One of the two items does not exist.');
      if (!cfg) throw new PrepareError(409, 'NoArmoryConfig', 'The armory has no config on this cluster yet.');
      const a = hookwars.itemCodec().decode(ai.data); const c = hookwars.itemCodec().decode(bi.data);
      if (a.templateId !== c.templateId) throw new PrepareError(409, 'NotForgeable', 'Only two items of the same template can be forged.');
      if (a.equippedCount > 0 || c.equippedCount > 0) throw new PrepareError(409, 'ItemEquipped', 'Unequip both items before forging them.');
      const itemsMinted = hookwars.armoryConfigCodec.decode(cfg.data).itemsMinted;
      return [hookwars.forge(owner, { item: pk(b, 'itemA'), itemMint: a.itemMint }, { item: pk(b, 'itemB'), itemMint: c.itemMint }, a.templateId, itemsMinted)];
    },
  },
  'proposals/finalize/prepare': {
    programs: ['armory'], label: 'Count the vote', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.finalize(pk(b, 'mint'), int(b, 'slot', 0, 255), big(b, 'nonce'))],
  },
  'proposals/cancel/prepare': {
    programs: ['armory'], label: 'Cancel the proposal', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.cancelProposal(pk(b, 'owner'), pk(b, 'mint'), int(b, 'slot', 0, 255), big(b, 'nonce'))],
  },
  'votes/close/prepare': {
    programs: ['armory'], label: 'Close the vote', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.closeVote(pk(b, 'owner'), pk(b, 'mint'), int(b, 'slot', 0, 255), big(b, 'nonce'))],
  },
  'war/funding/prepare': {
    programs: ['war'], label: 'Record funding', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.recordFunding(pk(b, 'mint'))],
  },
  'war/treaty-time/prepare': {
    programs: ['war'], label: 'Accrue treaty time', payer: (b) => pk(b, 'owner'),
    build: async (b) => {
      if (b.pairs !== undefined && (!Array.isArray(b.pairs) || b.pairs.length > MAX_PAIRS)) throw new PrepareError(400, 'BadRequest', `"pairs" must be a list of at most ${MAX_PAIRS} pairs.`);
      const pairs = ((b.pairs ?? []) as unknown[]).map((x) => {
        if (!Array.isArray(x) || x.length !== 2) throw new PrepareError(400, 'BadRequest', 'each pair is [item, partner].');
        try { return [new PublicKey(String(x[0])), new PublicKey(String(x[1]))] as [PublicKey, PublicKey]; } catch { throw new PrepareError(400, 'BadRequest', 'a pair holds something that is not an address.'); }
      });
      return [hookwars.accrueTreatyTime(pk(b, 'mint'), int(b, 'season', 0, 4_294_967_295), pairs)];
    },
  },
  'seasons/submit/prepare': {
    programs: ['war'], label: 'Submit a candidate', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.submitCandidate(pk(b, 'owner'), int(b, 'season', 0, 4_294_967_295), pk(b, 'mint'), Boolean(b.withLedger))],
  },
  'seasons/finalize/prepare': {
    programs: ['war'], label: 'Finalize the season', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.finalizeSeason(int(b, 'season', 0, 4_294_967_295))],
  },
  'seasons/open/prepare': {
    programs: ['war'], label: 'Open the next season', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const cfgInfo = await conn.getAccountInfo(hookwars.WAR_CONFIG, 'confirmed');
      if (!cfgInfo) throw new PrepareError(409, 'NoWarConfig', 'The war program has no config on this cluster yet.');
      return [hookwars.openSeason(hookwars.warConfigCodec.decode(cfgInfo.data).currentSeason)];
    },
  },
  'prize/split/prepare': {
    programs: ['war'], label: 'Split the prize vault', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const cfgInfo = await conn.getAccountInfo(hookwars.WAR_CONFIG, 'confirmed');
      if (!cfgInfo) throw new PrepareError(409, 'NoWarConfig', 'The war program has no config on this cluster yet.');
      const cfg = hookwars.warConfigCodec.decode(cfgInfo.data);
      const winner = cfg.lastWinner ? { mint: cfg.lastWinner, season: cfg.lastWinnerSeason } : null;
      return [hookwars.splitProtocolFees(pk(b, 'owner'), cfg.protocolTreasury, winner)];
    },
  },
  'raid/prepare': {
    programs: ['swap', 'launch', 'items', 'token'], label: 'Join the raid', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const target = pk(b, 'target'); const rival = pk(b, 'rival');
      const amount = big(b, 'amount'); const minOut = b.minOut === undefined ? 0n : big(b, 'minOut');
      if (amount <= 0n) throw new PrepareError(400, 'BadRequest', '"amount" must be above zero.');
      return raidRoute(conn, owner, rival, target, amount, minOut);
    },
  },
  // Changed by Hookwars (fuzz audit 1, finding 1): a buy on a launch pool, cut to the exact
  // remainder the curve can still fill (graduation waits for that buy).
  'buy/prepare': {
    programs: ['swap', 'launch', 'items', 'token'], label: 'Buy', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const owner = pk(b, 'owner'); const mint = pk(b, 'mint');
      const amount = big(b, 'amount'); const minOut = b.minOut === undefined ? 0n : big(b, 'minOut');
      if (amount <= 0n) throw new PrepareError(400, 'BadRequest', '"amount" must be above zero.');
      const { amountIn } = await remainderOf(conn, owner, mint, amount);
      return [token.createHolding(owner, mint, owner), hookwars.swapRoute(owner, amountIn, minOut, [await launchHop(conn, owner, mint, 1)])];
    },
  },
  'launch/prepare': {
    programs: ['launch', 'armory', 'items', 'token', 'swap', 'war'], label: 'Launch', payer: (b) => pk(b, 'owner'),
    build: async () => { throw new PrepareError(400, 'UseStagedRoute', 'A launch is several transactions; it is prepared by the staged launch route.'); },
    staged: async (b, conn) => launchStages(b, conn),
  },
};

/** The War orders, the Raid slot and its touch extras for `owner` (a war step that spends raid points). */
// Agents, market, social and the arsenal payouts (09, 10, 08): one transaction each.
Object.assign(PREPARES, EXPANSION_PREPARES);

async function raidContext(conn: Connection, mint: PublicKey, owner: PublicKey) {
  const ctx = await hookwars.fetchWarContext(conn, mint).catch(() => null);
  if (!ctx) throw new PrepareError(404, 'NoSuchToken', 'This is not a token on this cluster.');
  if (!ctx.raid) throw new PrepareError(409, 'NoRaidItem', 'This token has no Raid item equipped, so its holdings keep no raid points.');
  const slot = await hookwars.fetchSlot(conn, mint, ctx.raid.slot);
  const extras = await hookwars.fetchTouchExtras(conn, mint, slot, owner);
  return { orders: ctx.orders, raid: ctx.raid, extras };
}

/** A composite's modules (`["composite", item]` under the armory). */
async function compositeModules(conn: Connection, item: PublicKey): Promise<{ templateId: number; targetStart: number; targetCount: number }[]> {
  const key = PublicKey.findProgramAddressSync([Buffer.from('composite'), item.toBuffer()], new PublicKey(PROGRAM_IDS.armory))[0];
  const info = await conn.getAccountInfo(key, 'confirmed');
  if (!info) throw new PrepareError(409, 'NoComposite', 'The composite item has no module list.');
  const c = hookwars.coderOf('armory').decodeAccount('CompositeItem', info.data) as { modules: { templateId: number; targetStart: number; targetCount: number }[] };
  return c.modules;
}

const QUOTE = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
const holding = (mint: PublicKey, owner: PublicKey) => PublicKey.findProgramAddressSync([Buffer.from('holding'), mint.toBuffer(), owner.toBuffer()], new PublicKey(PROGRAM_IDS.token))[0];

/** The token-hook slices of one side of a launch-pool swap, for `owner`. */
async function sideSlices(conn: Connection, mint: PublicKey, pool: PublicKey, owner: PublicKey, sell: boolean): Promise<AccountMeta[]> {
  const vault = holding(mint, pool);
  const mine = holding(mint, owner);
  const s = await hookwars.fetchTokenHookSlices(conn, sell
    ? { mint, source: mine, destination: vault, authority: owner, sourceOwner: owner, destinationOwner: pool }
    : { mint, source: vault, destination: mine, authority: pool, sourceOwner: pool, destinationOwner: owner });
  if (!s) throw new PrepareError(404, 'NoSuchToken', 'This is not a token on this cluster.');
  return hookwars.sliceAccounts(s);
}

/** One launch-pool hop of a raid route. */
async function launchHop(conn: Connection, owner: PublicKey, mint: PublicKey, direction: 0 | 1): Promise<hookwars.RouteHop> {
  const pool = launchPoolAddress(mint, QUOTE, LP_FEE_BPS);
  const slices = await sideSlices(conn, mint, pool, owner, direction === 0);
  const items = await hookwars.fetchPoolItems(conn, mint);
  const four = launchHookExtras(mint, QUOTE);
  return {
    pool, baseMint: mint, quoteMint: QUOTE, traderBase: holding(mint, owner), traderQuote: holding(QUOTE, owner),
    hookProgram: new PublicKey(PROGRAM_IDS.launch), baseMintWritable: true, quoteMintWritable: false, direction,
    inSlice: direction === 0 ? slices : [], outSlice: direction === 1 ? slices : [],
    poolExtras: hookwars.slotPoolExtras(mint, QUOTE, four, items.accounts),
  };
}

/** The buy to send for `wanted` lamports of `mint` on its launch pool: `wanted`, or the exact remainder
 * the curve can still fill (and within max wallet). Pool items' cuts are not in the quote; they only
 * lower what reaches the curve, so the remainder still fits. */
export async function remainderOf(conn: Connection, owner: PublicKey, mint: PublicKey, wanted: bigint): Promise<{ amountIn: bigint; remainder: boolean }> {
  const pool = launchPoolAddress(mint, QUOTE, LP_FEE_BPS);
  const [pi, li] = await conn.getMultipleAccountsInfo([pool, hookwars.launchAddr(mint)], 'confirmed');
  if (!pi || !li) throw new PrepareError(404, 'NoSuchToken', 'This is not a launch on this cluster.');
  const p = decodePool(pi.data); const l = decodeLaunch(li.data);
  let eligible = 0n; let minEligible = 0n; let allowance = U64_MAX;
  if (l.modules !== 0) {
    const ki = await conn.getAccountInfo(l.kitConfig, 'confirmed');
    if (ki) {
      const k = decodeKitConfig(ki.data);
      eligible = k.eligible; minEligible = k.minEligible;
      if (k.maxWalletBps > 0 && !k.graduated) {
        const hi = await conn.getAccountInfo(holding(mint, owner), 'confirmed');
        const held = hi ? (hookwars.holdingCodec.decode(hi.data) as { amount: bigint }).amount : 0n;
        allowance = k.maxWalletAmount > held ? k.maxWalletAmount - held : 0n;
      }
    }
  }
  const r = { baseReserve: p.baseReserve, quoteReserve: p.quoteReserve, virtualBase: p.virtualBase, virtualQuote: p.virtualQuote };
  const fees = { creatorFeeBps: l.creatorFeeBps, holderFeeBuyBps: l.rules.holderFeeBuyBps, holderFeeSellBps: l.rules.holderFeeSellBps, burnBuyBps: l.rules.burnBuyBps, burnSellBps: l.rules.burnSellBps, eligible, minEligible };
  const out = remainderBuy(r, wanted, p.lpFeeBps, p.protocolShareBps, fees, allowance);
  if (out.amountIn === 0n) throw new PrepareError(409, 'CurveFull', 'The curve has nothing left to buy (or your max wallet is reached): it is ready to graduate.');
  return out;
}

/** A raid: sell the rival on its own launch pool and buy the target, in one `swap_route`. */
async function raidRoute(conn: Connection, owner: PublicKey, rival: PublicKey, target: PublicKey, amount: bigint, minOut: bigint): Promise<TransactionInstruction[]> {
  const hops = [await launchHop(conn, owner, rival, 0), await launchHop(conn, owner, target, 1)];
  return [token.createHolding(owner, target, owner), hookwars.swapRoute(owner, amount, minOut, hops)];
}

/** The slot table of a launch request, as the web form sends it. */
interface SlotRequest { kind: string; rule: string; maxCutBps: number; noticeSecs: number; templateId: number | null; launchItem: string; targets?: string[] }
const KIND: Record<string, number> = { fee: 0, reward: 1, defense: 2, relation: 3, pool: 4, war: 6 };
const RULE: Record<string, number> = { locked: 0, vote: 1, performance: 2 };
/** Templates whose module answers holder touches, and those that burn on a swap. */
const TOUCH = new Set([1]);
const BURN = new Set([32, 33]);

/** A launch's staged transactions (03 section 4.3, M3b): prepare, one equip per launch item, the
 * mint's lookup table, the launch, then the registry, the war state and the raid ledger. */
export async function launchStages(b: Body, conn: Connection): Promise<Stage[]> {
  const owner = pk(b, 'owner'); const mint = pk(b, 'mint');
  const name = String(b.name ?? ''); const symbol = String(b.symbol ?? '');
  if (!name || name.length > 32 || !/^[^\s]{1,10}$/.test(symbol)) throw new PrepareError(400, 'BadRequest', 'A name of 1 to 32 characters and a ticker of 1 to 10 without spaces are required.');
  const virtualQuote = big(b, 'virtualQuote');
  if (virtualQuote < MIN_VIRTUAL_QUOTE || virtualQuote > MAX_VIRTUAL_QUOTE) throw new PrepareError(400, 'BadRequest', `"virtualQuote" must be from ${MIN_VIRTUAL_QUOTE} to ${MAX_VIRTUAL_QUOTE} lamports.`);
  const creatorFeeBps = b.creatorFeeBps === undefined ? 0 : int(b, 'creatorFeeBps', 0, 200);
  const rulesIn: LaunchRulesInput = b.kit ? (b.rules as LaunchRulesInput | undefined) ?? (() => { throw new PrepareError(400, 'BadRequest', 'Kit rules are on: send "rules".'); })() : NO_RULES;
  const rules = launchRulesFromInput(rulesIn);
  if (!Array.isArray(b.slots) || b.slots.length > 4) throw new PrepareError(400, 'BadRequest', '"slots" must be a list of at most 4 slots.');
  const req = b.slots as SlotRequest[];
  const slots: hookwars.SlotInitArgs[] = req.map((r) => {
    const kind = KIND[r.kind]; const equipRule = RULE[r.rule];
    if (kind === undefined || equipRule === undefined) throw new PrepareError(400, 'BadRequest', 'A slot has an unknown kind or equip rule.');
    const t = TEMPLATES.find((x) => x.id === r.templateId);
    const dataLen = t && t.dataBytes > 0 ? t.dataBytes + 1 : 0;
    const maxCutBps = Number.isInteger(r.maxCutBps) && r.maxCutBps >= 0 && r.maxCutBps <= 10_000 ? r.maxCutBps : 0;
    return {
      kind, equipRule,
      bounds: { maxCutBps: kind === 4 || kind === 6 ? 0 : maxCutBps, mayRefuse: true, mayWriteData: dataLen > 0, mayAnswerTouch: t ? TOUCH.has(t.id) : false, mayBurn: t ? BURN.has(t.id) : false },
      dataLen, lockedProgram: null, lockedFlags: 0, lockedExtraCount: 0,
    };
  });
  const uri = String(b.uri ?? '');
  // Changed by Hookwars: two phases. The launch's deposit slices and the pool items' registries
  // exist only once the equips have landed, so `phase: "launch"` is a second call made after the
  // first phase's transactions confirm (the site does this; security review 2 L-D).
  if (b.phase === 'launch') return launchPhase(conn, owner, mint, { name, symbol, uri, creatorFeeBps, virtualQuote, rules }, req);
  if (b.phase !== undefined && b.phase !== 'prepare') throw new PrepareError(400, 'BadRequest', '"phase" is "prepare" or "launch".');
  const stages: Stage[] = [];
  stages.push({ label: 'Prepare the launch', ixs: [hookwars.prepareLaunch(owner, mint, { name, symbol, uri, creatorFeeBps, rules, slots })], extraSigners: ['mint'], simulate: true, tables: [] });
  const armory = new PublicKey(PROGRAM_IDS.armory);
  for (let i = 0; i < req.length; i++) {
    const r = req[i]!;
    if (!r.launchItem) continue;
    let item: PublicKey;
    try { item = new PublicKey(r.launchItem); } catch { throw new PrepareError(400, 'BadRequest', `slot ${i}: the launch item is not an address.`); }
    const info = await conn.getAccountInfo(item, 'confirmed');
    if (!info || !info.owner.equals(armory)) throw new PrepareError(409, 'NoItem', `slot ${i}: no such item on this cluster.`);
    const it = hookwars.itemCodec().decode(info.data) as { templateId: number; manifest: { tokenFlags: number; poolFlags: number } };
    const targets = (r.targets ?? []).slice(0, MAX_TARGETS).map((x) => new PublicKey(x));
    const entry = { slot: i, item, config: { targets, role: 0 }, noticeSecs: Number.isInteger(r.noticeSecs) && r.noticeSecs >= 0 ? r.noticeSecs : 0, rule: null };
    const tokenCuts = (it.manifest.tokenFlags & 64) !== 0; const poolCuts = it.manifest.poolFlags !== 0;
    const eq = hookwars.equipLaunch(owner, mint, QUOTE, entry, { item, templateId: it.templateId, tokenCuts, poolCuts, composite: it.templateId === 41 });
    stages.push({ label: `Equip slot ${i}`, ixs: [hookwars.equipPrepared(owner, mint, eq)], extraSigners: [], simulate: false, tables: [] });
  }
  return stages;
}

/** The forwarded pool slots' item registries of a prepared mint, in slot order. */
export function poolItemRegistries(m: hookwars.SlotMintData, mint: PublicKey): PublicKey[] {
  return hookwars.activeSlots(m)
    .filter((s) => (s.kind === 4 || s.kind === 3) && (s.poolFlags & 3) !== 0 && !s.item.equals(PublicKey.default))
    .map((s) => hookwars.slotRegistryAddress(s, mint));
}

/** The second phase: the mint's lookup table, the launch (deposit slices and pool registries read
 * from the chain), then war state and raid ledger. */
export async function launchPhase(conn: Connection, owner: PublicKey, mint: PublicKey, args: { name: string; symbol: string; uri: string; creatorFeeBps: number; virtualQuote: bigint; rules: ReturnType<typeof launchRulesFromInput> }, req: SlotRequest[]): Promise<Stage[]> {
  const mintInfo = await conn.getAccountInfo(mint, 'confirmed');
  if (!mintInfo) throw new PrepareError(409, 'NotPrepared', 'Prepare the launch first: this mint does not exist yet.');
  const m = hookwars.decodeSlotMint(mintInfo.data);
  const cfg = await conn.getAccountInfo(LAUNCH_CONFIG, 'confirmed');
  if (!cfg) throw new PrepareError(409, 'NoLaunchConfig', 'The launchpad has no config on this cluster yet.');
  const treasury = decodeLaunchConfig(cfg.data).treasury;
  const pool = launchPoolAddress(mint, QUOTE, LP_FEE_BPS);
  const launch = hookwars.launchAddr(mint);
  const s = await hookwars.fetchTokenHookSlices(conn, { mint, source: holding(mint, launch), destination: holding(mint, pool), authority: launch, sourceOwner: launch, destinationOwner: pool });
  if (!s) throw new PrepareError(409, 'NotPrepared', 'This mint is not a prepared slot launch.');
  const create = hookwars.createPreparedLaunch(owner, mint, treasury, QUOTE, LP_FEE_BPS, args, poolItemRegistries(m, mint), hookwars.sliceAccounts(s));
  const stages: Stage[] = [];
  // The launch: its accounts are too many for one legacy transaction, so it goes through a lookup
  // table of its own, made and extended in the stages before it (audit A-6).
  const recentSlot = await conn.getSlot('finalized');
  const [createTable, table] = AddressLookupTableProgram.createLookupTable({ authority: owner, payer: owner, recentSlot });
  const addrs: PublicKey[] = [];
  for (const k of create.keys) if (!k.isSigner && !addrs.some((a) => a.equals(k.pubkey))) addrs.push(k.pubkey);
  const chunks: PublicKey[][] = [];
  for (let i = 0; i < addrs.length; i += 20) chunks.push(addrs.slice(i, i + 20));
  chunks.forEach((c, i) => stages.push({
    label: i === 0 ? "The token's lookup table" : `The token's lookup table, part ${i + 1}`,
    ixs: [...(i === 0 ? [createTable] : []), AddressLookupTableProgram.extendLookupTable({ lookupTable: table, authority: owner, payer: owner, addresses: c })],
    extraSigners: [], simulate: false, tables: [],
  }));
  const local = new AddressLookupTableAccount({ key: table, state: { deactivationSlot: BigInt('18446744073709551615'), lastExtendedSlot: 0, lastExtendedSlotStartIndex: 0, authority: owner, addresses: addrs } });
  stages.push({ label: 'Launch', ixs: [create], extraSigners: create.keys.some((k) => k.isSigner && k.pubkey.equals(mint)) ? ['mint'] : [], simulate: false, tables: [local] });
  const raids = req.some((r) => r.templateId === 1 || r.templateId === 2);
  const after: TransactionInstruction[] = [hookwars.initWar(owner, mint)];
  if (raids) after.push(hookwars.initRaidLedger(owner, mint));
  stages.push({ label: 'War chest', ixs: after, extraSigners: [], simulate: false, tables: [] });
  return stages;
}

export async function deployed(conn: Connection, names: (keyof typeof PROGRAM_IDS)[]): Promise<string[]> {
  const keys = names.map((x) => new PublicKey(PROGRAM_IDS[x]));
  const infos = await conn.getMultipleAccountsInfo(keys, 'confirmed');
  return names.filter((_, i) => !infos[i]?.executable);
}

/** The protocol lookup table, when `PROTOCOL_LOOKUP_TABLE` names one and it holds the protocol's
 * addresses in order (audit A-6); none otherwise. */
export async function protocolTable(conn: Connection): Promise<AddressLookupTableAccount[]> {
  const key = process.env.PROTOCOL_LOOKUP_TABLE;
  if (!key) return [];
  const r = await conn.getAddressLookupTable(new PublicKey(key), { commitment: 'confirmed' });
  if (!r.value) return [];
  return checkProtocolLookupTable(r.value) === null ? [r.value] : [];
}

const SWAP_PROGRAM_ID = new PublicKey(PROGRAM_IDS.swap);
export const PACKET = 1_232;

/** Compiles `ixs` as v0 with `tables`, simulating first when `simulate` (units used plus 15%, upstream
 * hooks-v2 section 6), else with the full compute limit; refuses what would not fit a packet. */
export async function finish(conn: Connection, payer: PublicKey, ixs: TransactionInstruction[], label: string, opts: { simulate?: boolean; tables?: AddressLookupTableAccount[]; extraSigners?: ('mint' | 'config')[]; stage?: number } = {}): Promise<PreparedTx> {
  const tables = opts.tables ?? [];
  const { blockhash } = await conn.getLatestBlockhash('confirmed');
  const compile = (limit: number) => new VersionedTransaction(new TransactionMessage({
    // The DEX's allocator uses a requested heap frame: a route through slot launches needs more than 32 KiB.
    payerKey: payer, recentBlockhash: blockhash, instructions: [ComputeBudgetProgram.setComputeUnitLimit({ units: limit }), ...(ixs.some((i) => i.programId.equals(SWAP_PROGRAM_ID)) ? [ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 })] : []), ...ixs],
  }).compileToV0Message(tables));
  let limit = 1_400_000;
  if (opts.simulate !== false) {
    const sim = await conn.simulateTransaction(compile(1_400_000), { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed' });
    if (sim.value.err) {
      const log = (sim.value.logs ?? []).reverse().find((l) => /Error Message:|failed/.test(l)) ?? JSON.stringify(sim.value.err);
      throw new PrepareError(409, 'SimulationFailed', `The programs refused this: ${log.replace(/^Program log: /, '')}`);
    }
    limit = Math.min(1_400_000, Math.ceil((sim.value.unitsConsumed ?? 200_000) * 1.15));
  }
  const tx = compile(limit);
  let bytes: Uint8Array;
  try { bytes = tx.serialize(); } catch { throw new PrepareError(409, 'TooLarge', `"${label}" is too large for one transaction even with its lookup tables.`); }
  if (bytes.length > PACKET) throw new PrepareError(409, 'TooLarge', `"${label}" is ${bytes.length} bytes, over ${PACKET}.`);
  return { transaction: Buffer.from(bytes).toString('base64'), version: 'v0', stage: opts.stage ?? 0, label, extraSigners: opts.extraSigners ?? [] };
}

/** A slot launch's own lookup table, when the indexer recorded one for `mint` (audit A-6). */
export async function mintTables(conn: Connection, db: Db | null | undefined, mint: unknown): Promise<AddressLookupTableAccount[]> {
  if (!db || typeof mint !== 'string') return [];
  let key: string | undefined;
  try { key = (await db.query<{ lookup_table: string }>('select lookup_table from mint_tables where mint = $1', [mint])).rows[0]?.lookup_table; } catch { return []; }
  if (!key) return [];
  const r = await conn.getAddressLookupTable(new PublicKey(key), { commitment: 'confirmed' });
  return r.value ? [r.value] : [];
}

export async function prepare(conn: Connection, route: string, body: Body, db?: Db | null): Promise<{ transactions: PreparedTx[] }> {
  const def = PREPARES[route];
  if (!def) throw new PrepareError(404, 'NotFound', 'No such prepare route.');
  const missing = await deployed(conn, def.programs);
  if (missing.length) {
    throw new PrepareError(409, 'NotDeployed', `Not on this cluster yet: the ${missing.join(', ')} program${missing.length > 1 ? 's are' : ' is'} not deployed, so this cannot be prepared.`);
  }
  const protocol = [...await protocolTable(conn), ...await mintTables(conn, db, body.mint ?? body.tokenMint)];
  if (def.staged) {
    const stages = await def.staged(body, conn);
    const out: PreparedTx[] = [];
    for (let i = 0; i < stages.length; i++) {
      const st = stages[i]!;
      out.push(await finish(conn, def.payer(body), st.ixs, st.label, { simulate: st.simulate, tables: [...protocol, ...st.tables], extraSigners: st.extraSigners, stage: i }));
    }
    return { transactions: out };
  }
  const ixs = await def.build(body, conn);
  return { transactions: [await finish(conn, def.payer(body), ixs, def.label, { tables: protocol })] };
}
