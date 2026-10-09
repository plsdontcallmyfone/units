/**
 * The prepare routes of docs/spec/06-app.md 3.3: build with the SDK, simulate, return unsigned v0
 * transactions for the wallet (the backend holds no user key, 06 section 1 rule 5). Every prepare
 * first checks that the programs it calls exist on the cluster; until they are deployed it refuses
 * with one sentence that says so, rather than returning a transaction that cannot land.
 */
import {
  ComputeBudgetProgram, Connection, PublicKey, TransactionMessage, VersionedTransaction, type TransactionInstruction,
} from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PROGRAM_IDS, type PreparedTx } from '@hookwars/shared';

export class PrepareError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) { super(message); this.status = status; this.code = code; }
}

type Body = Record<string, unknown>;
const pk = (b: Body, k: string): PublicKey => {
  const v = b[k];
  if (typeof v !== 'string') throw new PrepareError(400, 'BadRequest', `"${k}" is required.`);
  try { return new PublicKey(v); } catch { throw new PrepareError(400, 'BadRequest', `"${k}" is not an address.`); }
};
const n = (b: Body, k: string): number => {
  const v = Number(b[k]);
  if (!Number.isFinite(v)) throw new PrepareError(400, 'BadRequest', `"${k}" must be a number.`);
  return v;
};
const big = (b: Body, k: string): bigint => {
  try { return BigInt(String(b[k])); } catch { throw new PrepareError(400, 'BadRequest', `"${k}" must be an integer amount.`); }
};

interface PrepareDef {
  programs: (keyof typeof PROGRAM_IDS)[];
  label: string;
  payer: (b: Body) => PublicKey;
  build: (b: Body, conn: Connection) => Promise<TransactionInstruction[]>;
}

/** Route path (after /v1) to its builder. */
export const PREPARES: Record<string, PrepareDef> = {
  'votes/prepare': {
    programs: ['armory', 'token'], label: 'Vote', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.vote(pk(b, 'owner'), pk(b, 'mint'), n(b, 'slot'), big(b, 'nonce'), Boolean(b.support), big(b, 'amount'))],
  },
  'proposals/prepare': {
    programs: ['armory'], label: 'Propose', payer: (b) => pk(b, 'owner'),
    build: async (b, conn) => {
      const mint = pk(b, 'mint'); const slot = n(b, 'slot');
      const ss = await conn.getAccountInfo(hookwars.slotStateAddress(mint, slot), 'confirmed');
      if (!ss) throw new PrepareError(409, 'NoSlotState', 'This slot has no state yet: it was not filled at launch, so it takes no proposals.');
      const state = hookwars.slotStateCodec.decode(ss.data);
      const targets = Array.isArray(b.targets) ? (b.targets as string[]).map((t) => new PublicKey(t)) : [];
      const role = ({ none: 0, pay: 1, receive: 2 } as Record<string, number>)[String(b.role ?? 'none')] ?? 0;
      return [hookwars.propose(pk(b, 'owner'), mint, slot, state.nextNonce, b.item ? pk(b, 'item') : null, targets, role)];
    },
  },
  'settle/prepare': {
    programs: ['items', 'token'], label: 'Settle', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.settleEquip(pk(b, 'owner'), pk(b, 'mint'), n(b, 'slot'), pk(b, 'item'))],
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
      return [hookwars.claimQuest(owner, mint, n(b, 'season'), n(b, 'questId') === 2 ? 2 : 1, n(b, 'period'), raid.slot, extras)];
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
    build: async (b) => [hookwars.finalize(pk(b, 'mint'), n(b, 'slot'), big(b, 'nonce'))],
  },
  'proposals/cancel/prepare': {
    programs: ['armory'], label: 'Cancel the proposal', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.cancelProposal(pk(b, 'owner'), pk(b, 'mint'), n(b, 'slot'), big(b, 'nonce'))],
  },
  'votes/close/prepare': {
    programs: ['armory'], label: 'Close the vote', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.closeVote(pk(b, 'owner'), pk(b, 'mint'), n(b, 'slot'), big(b, 'nonce'))],
  },
  'war/funding/prepare': {
    programs: ['war'], label: 'Record funding', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.recordFunding(pk(b, 'mint'))],
  },
  'war/treaty-time/prepare': {
    programs: ['war'], label: 'Accrue treaty time', payer: (b) => pk(b, 'owner'),
    build: async (b) => {
      const pairs = Array.isArray(b.pairs) ? (b.pairs as [string, string][]).map(([i, p]) => [new PublicKey(i), new PublicKey(p)] as [PublicKey, PublicKey]) : [];
      return [hookwars.accrueTreatyTime(pk(b, 'mint'), n(b, 'season'), pairs)];
    },
  },
  'seasons/submit/prepare': {
    programs: ['war'], label: 'Submit a candidate', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.submitCandidate(pk(b, 'owner'), n(b, 'season'), pk(b, 'mint'), Boolean(b.withLedger))],
  },
  'seasons/finalize/prepare': {
    programs: ['war'], label: 'Finalize the season', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.finalizeSeason(n(b, 'season'))],
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
    build: async () => { throw new PrepareError(409, 'RouteNeedsDex', 'A raid is a two-hop swap_route; it needs the DEX with swap_route on this cluster.'); },
  },
  'launch/prepare': {
    programs: ['launch', 'armory', 'items', 'token', 'swap'], label: 'Launch', payer: (b) => pk(b, 'owner'),
    build: async () => { throw new PrepareError(409, 'LaunchNeedsPrograms', 'A launch is prepare_launch, create_launch and init_war; it needs the launchpad with slot launches on this cluster.'); },
  },
};

/** The War orders, the Raid slot and its touch extras for `owner` (a war step that spends raid points). */
async function raidContext(conn: Connection, mint: PublicKey, owner: PublicKey) {
  const ctx = await hookwars.fetchWarContext(conn, mint).catch(() => null);
  if (!ctx) throw new PrepareError(404, 'NoSuchToken', 'This is not a token on this cluster.');
  if (!ctx.raid) throw new PrepareError(409, 'NoRaidItem', 'This token has no Raid item equipped, so its holdings keep no raid points.');
  const slot = await hookwars.fetchSlot(conn, mint, ctx.raid.slot);
  const extras = await hookwars.fetchTouchExtras(conn, mint, slot, owner);
  return { orders: ctx.orders, raid: ctx.raid, extras };
}

export async function deployed(conn: Connection, names: (keyof typeof PROGRAM_IDS)[]): Promise<string[]> {
  const keys = names.map((x) => new PublicKey(PROGRAM_IDS[x]));
  const infos = await conn.getMultipleAccountsInfo(keys, 'confirmed');
  return names.filter((_, i) => !infos[i]?.executable);
}

/** Simulates, sizes the compute limit at units used plus 15% (upstream hooks-v2 section 6), returns the unsigned v0 transaction. */
export async function finish(conn: Connection, payer: PublicKey, ixs: TransactionInstruction[], label: string): Promise<PreparedTx> {
  const { blockhash } = await conn.getLatestBlockhash('confirmed');
  const compile = (limit: number) => new VersionedTransaction(new TransactionMessage({
    payerKey: payer, recentBlockhash: blockhash, instructions: [ComputeBudgetProgram.setComputeUnitLimit({ units: limit }), ...ixs],
  }).compileToV0Message());
  const sim = await conn.simulateTransaction(compile(1_400_000), { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed' });
  if (sim.value.err) {
    const log = (sim.value.logs ?? []).reverse().find((l) => /Error Message:|failed/.test(l)) ?? JSON.stringify(sim.value.err);
    throw new PrepareError(409, 'SimulationFailed', `The programs refused this: ${log.replace(/^Program log: /, '')}`);
  }
  const used = sim.value.unitsConsumed ?? 200_000;
  const tx = compile(Math.min(1_400_000, Math.ceil(used * 1.15)));
  return { transaction: Buffer.from(tx.serialize()).toString('base64'), version: 'v0', stage: 0, label, extraSigners: [] };
}

export async function prepare(conn: Connection, route: string, body: Body): Promise<{ transactions: PreparedTx[] }> {
  const def = PREPARES[route];
  if (!def) throw new PrepareError(404, 'NotFound', 'No such prepare route.');
  const missing = await deployed(conn, def.programs);
  if (missing.length) {
    throw new PrepareError(409, 'NotDeployed', `Not on this cluster yet: the ${missing.join(', ')} program${missing.length > 1 ? 's are' : ' is'} not deployed, so this cannot be prepared.`);
  }
  const ixs = await def.build(body, conn);
  return { transactions: [await finish(conn, def.payer(body), ixs, def.label)] };
}
