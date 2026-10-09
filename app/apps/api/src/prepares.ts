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
    build: async (b) => [hookwars.claimBounty(pk(b, 'owner'), pk(b, 'mint'), pk(b, 'warOrdersItem'), pk(b, 'warOrdersTemplate'), [])],
  },
  'quests/prepare': {
    programs: ['war', 'token', 'items'], label: 'Claim quest', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.claimQuest(pk(b, 'owner'), pk(b, 'mint'), n(b, 'season'), n(b, 'questId') === 2 ? 2 : 1, n(b, 'period'), [])],
  },
  'rolls/prepare': {
    programs: ['war', 'token', 'items'], label: 'Roll', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.roll(pk(b, 'owner'), pk(b, 'mint'), big(b, 'nonce'), [], [])],
  },
  'war/init/prepare': {
    programs: ['war'], label: 'Open the war chest', payer: (b) => pk(b, 'owner'),
    build: async (b) => [hookwars.initWar(pk(b, 'owner'), pk(b, 'mint'), hookwars.launchAddr(pk(b, 'mint')))],
  },
  'forge/prepare': {
    programs: ['armory', 'items', 'token'], label: 'Forge', payer: (b) => pk(b, 'owner'),
    build: async () => { throw new PrepareError(409, 'ForgeNeedsArmory', 'Forging needs the armory on this cluster to assign the new item mint.'); },
  },
  'raid/prepare': {
    programs: ['swap', 'launch', 'items', 'token'], label: 'Join the raid', payer: (b) => pk(b, 'owner'),
    build: async () => { throw new PrepareError(409, 'RouteNeedsDex', 'A raid is a two-hop swap_route; it needs the DEX with swap_route on this cluster.'); },
  },
  'launch/prepare': {
    programs: ['launch', 'armory', 'items', 'token', 'swap'], label: 'Launch', payer: (b) => pk(b, 'owner'),
    build: async () => { throw new PrepareError(409, 'LaunchNeedsPrograms', 'A launch is prepare_launch, create_launch and init_war; it needs the launchpad, armory and items on this cluster.'); },
  },
};

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
