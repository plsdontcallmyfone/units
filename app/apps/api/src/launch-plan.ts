// Changed by Hookwars: new file (explorer v2, launch page). The launch walked step by step: the same
// staged transactions as `launch/prepare`, each returned with what it holds (its instructions
// decoded back from the very bytes the wallet will sign), its size against the packet, its lookup
// tables and its fee; a simulate route the page calls for each step right before signing it; and
// the buy quote that says when a buy is cut to the curve's remainder (fuzz audit 1, finding 1).
import { Connection, PublicKey, VersionedTransaction } from '@solana/web3.js';
import { hookwars, decodeLaunchConfig, LAUNCH_CONFIG } from '@hookwars/sdk';
import { FIXED_ADDRESSES, LP_FEE_BPS, PROGRAM_IDS, TEMPLATES, type PreparedTx } from '@hookwars/shared';
import { deployed, finish, launchStages, PACKET, pk, poolItemRegistries, PREPARES, PrepareError, protocolTable, remainderOf, type Body, type Stage } from './prepares.ts';

const QUOTE = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
/** The cluster's base fee per signature (lamports). */
const LAMPORTS_PER_SIGNATURE = 5_000;

export interface PlanStep {
  stage: number; label: string;
  /** Decoded from the unsigned transaction itself. */
  instructions: { program: string | null; name: string | null; args: Record<string, unknown> | null }[];
  bytes: number; packet: number; lookupTables: number; signatures: number; feeLamports: number;
  /** The compute limit written into the transaction, and whether it came from a simulation now. */
  computeLimit: number | null; simulatedNow: boolean;
}

/** What a prepared transaction holds, read back from its bytes. */
export function describe(tx: PreparedTx): PlanStep {
  const bytes = Buffer.from(tx.transaction, 'base64');
  const v = VersionedTransaction.deserialize(bytes);
  const m = v.message;
  const keys = m.staticAccountKeys.map((k) => k.toBase58());
  const lookups = m.addressTableLookups;
  // Keys loaded from tables are not known without the tables; an instruction naming one shows its index.
  const keyAt = (i: number) => keys[i] ?? `table key ${i - keys.length}`;
  const ixs = m.compiledInstructions.map((c) => {
    const d = hookwars.explore.decodeInstruction(keyAt(c.programIdIndex), Buffer.from(c.data), c.accountKeyIndexes.map((i) => ({ pubkey: keyAt(i), writable: false, signer: false })));
    return { program: d.program, name: d.name, args: d.args };
  });
  const limit = ixs.find((x) => x.program === 'computeBudget' && x.name === 'setComputeUnitLimit')?.args?.units;
  const signatures = m.header.numRequiredSignatures;
  return {
    stage: tx.stage, label: tx.label, instructions: ixs, bytes: bytes.length, packet: PACKET, lookupTables: lookups.length,
    signatures, feeLamports: signatures * LAMPORTS_PER_SIGNATURE,
    computeLimit: typeof limit === 'number' ? limit : null, simulatedNow: typeof limit === 'number' && limit < 1_400_000,
  };
}

/** The launch's stages (03 4.3): phase "prepare" is prepare_launch and one equip_prepared per launch
 * item; phase "launch" is the mint's lookup table, create_prepared_launch, refresh_pool_registry
 * when a pool slot forwards to an item, and the war chest (init_war, init_raid_ledger). */
export async function launchPlan(conn: Connection, b: Body) {
  const missing = await deployed(conn, ['launch', 'armory', 'items', 'token', 'swap', 'war']);
  if (missing.length) throw new PrepareError(409, 'NotDeployed', `Not on this cluster yet: the ${missing.join(', ')} program${missing.length > 1 ? 's are' : ' is'} not deployed, so this cannot be prepared.`);
  const owner = new PublicKey(String(b.owner));
  const mint = new PublicKey(String(b.mint));
  const stages: Stage[] = await launchStages(b, conn);
  if (b.phase === 'launch') {
    const info = await conn.getAccountInfo(mint, 'confirmed');
    const regs = info ? poolItemRegistries(hookwars.decodeSlotMint(info.data), mint) : [];
    const at = stages.findIndex((s) => s.label === 'Launch');
    if (regs.length && at >= 0) stages.splice(at + 1, 0, { label: 'Refresh the pool registry', ixs: [hookwars.refreshPoolRegistry(owner, mint, QUOTE, LP_FEE_BPS, regs)], extraSigners: [], simulate: false, tables: [] });
  }
  const protocol = await protocolTable(conn);
  const transactions: PreparedTx[] = [];
  for (let i = 0; i < stages.length; i++) {
    const st = stages[i]!;
    transactions.push(await finish(conn, owner, st.ixs, st.label, { simulate: st.simulate, tables: [...protocol, ...st.tables], extraSigners: st.extraSigners, stage: i }));
  }
  // The launchpad's own fee, read from its config (charged once by create_prepared_launch).
  const cfg = await conn.getAccountInfo(LAUNCH_CONFIG, 'confirmed');
  const launchFeeLamports = cfg ? decodeLaunchConfig(cfg.data).launchFeeLamports.toString() : null;
  return { transactions, steps: transactions.map(describe), launchFeeLamports };
}

/** Simulates one unsigned prepared transaction now (the page does this for each step right before
 * the wallet signs it): units used, and the program's own error when it would fail. */
export async function simulate(conn: Connection, b: Body) {
  if (typeof b.transaction !== 'string' || b.transaction.length > 4000) throw new PrepareError(400, 'BadRequest', '"transaction" is the base64 of one prepared transaction.');
  const tx = VersionedTransaction.deserialize(Buffer.from(b.transaction, 'base64'));
  const keys = tx.message.staticAccountKeys;
  const ours = new Set<string>([...Object.values(PROGRAM_IDS), hookwars.explore.programIdOf('craft') ?? '', hookwars.explore.programIdOf('book') ?? '']);
  if (!tx.message.compiledInstructions.some((c) => ours.has(keys[c.programIdIndex]?.toBase58() ?? '')) && !tx.message.compiledInstructions.some((c) => keys[c.programIdIndex]?.toBase58() === 'AddressLookupTab1e1111111111111111111111111')) {
    throw new PrepareError(400, 'BadRequest', 'Only units transactions are simulated here.');
  }
  const sim = await conn.simulateTransaction(tx, { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed' });
  const logs = sim.value.logs ?? [];
  return { ok: sim.value.err === null, units: sim.value.unitsConsumed ?? null, failure: sim.value.err === null ? null : hookwars.explore.failureOf(logs) ?? { programId: '', program: null, code: null, name: null, message: JSON.stringify(sim.value.err) }, logs: logs.slice(-30) };
}

/** What a buy of `amount` lamports sends: the amount, or the curve's exact remainder near graduation. */
export async function buyQuote(conn: Connection, q: URLSearchParams) {
  let owner: PublicKey; let mint: PublicKey;
  try { owner = new PublicKey(q.get('owner') ?? ''); mint = new PublicKey(q.get('mint') ?? ''); } catch { throw new PrepareError(400, 'BadRequest', '"owner" and "mint" are addresses.'); }
  const a = q.get('amount') ?? '';
  if (!/^\d{1,20}$/.test(a) || BigInt(a) === 0n) throw new PrepareError(400, 'BadRequest', '"amount" is lamports above zero.');
  const r = await remainderOf(conn, owner, mint, BigInt(a));
  return { wanted: a, amountIn: r.amountIn.toString(), remainder: r.remainder };
}

// ------------------------------------------------------------------- composites (08 section 2) --

/** `MAX_MODULES` (08 2.2) and the module rules of `hookwars_common::composite::validate_modules`
 * the page checks before anything is signed; the armory checks them again on chain. */
export const MAX_MODULES = 4;
const NOT_COMPOSABLE = new Set([9, 41, 42, 43, 44, 45]);
const NO_READ = 0xff;
export interface ModuleIn { templateId: number; params: number[]; targetStart: number; targetCount: number; readsModule?: number }

/** The module list as the armory takes it, or the reason the armory would refuse it (by its error name). */
export function compositeModules(modules: ModuleIn[]): { modules: Record<string, unknown>[] } | { refused: string; reason: string } {
  if (!Array.isArray(modules) || modules.length === 0 || modules.length > MAX_MODULES) return { refused: 'TooManyModules', reason: `One to ${MAX_MODULES} modules.` };
  const out: Record<string, unknown>[] = [];
  for (const [i, m] of modules.entries()) {
    const t = TEMPLATES.find((x) => x.id === m.templateId);
    if (!t || NOT_COMPOSABLE.has(m.templateId)) return { refused: 'NotComposable', reason: `Module ${i + 1}: ${t?.name ?? `template ${m.templateId}`} cannot be a module.` };
    if (!Array.isArray(m.params) || m.params.length > 11 || m.params.some((x) => !Number.isInteger(x) || x < 0 || x > 4_294_967_295)) return { refused: 'BadParams', reason: `Module ${i + 1}: params are up to 11 whole numbers.` };
    const params = [...m.params, ...Array(11 - m.params.length).fill(0)];
    const reads = m.readsModule ?? NO_READ;
    out.push({ templateId: m.templateId, params, targetStart: m.targetStart, targetCount: m.targetCount, dataBytes: t.dataBytes, readsModule: reads });
  }
  return { modules: out };
}

Object.assign(PREPARES, {
  'composite/prepare': {
    programs: ['armory', 'items', 'token'], label: 'Create a composite item', payer: (b: Body) => pk(b, 'owner'),
    build: async (b: Body, conn: Connection) => {
      const owner = pk(b, 'owner');
      const r = compositeModules(b.modules as ModuleIn[]);
      if ('refused' in r) throw new PrepareError(400, r.refused, r.reason);
      const royaltyBps = Number(b.royaltyBps ?? 0);
      if (!Number.isInteger(royaltyBps) || royaltyBps < 0 || royaltyBps > 10_000) throw new PrepareError(400, 'BadRequest', '"royaltyBps" is 0 to 10,000.');
      const cfg = await conn.getAccountInfo(hookwars.armoryConfigAddress(), 'confirmed');
      if (!cfg) throw new PrepareError(409, 'NoArmoryConfig', 'The armory has no config on this cluster yet.');
      const itemMint = hookwars.itemMintAddress(hookwars.armoryConfigCodec.decode(cfg.data).itemsMinted);
      const item = hookwars.itemAddress(itemMint);
      const composite = PublicKey.findProgramAddressSync([Buffer.from('composite'), item.toBuffer()], new PublicKey(PROGRAM_IDS.armory))[0];
      const templates = (b.modules as ModuleIn[]).map((m) => ({ pubkey: hookwars.templateAddress(m.templateId), isSigner: false, isWritable: false }));
      return [hookwars.idlIx('armory', 'create_composite', {
        author: owner, template: hookwars.templateAddress(41), itemMint, item, composite, recipientHolding: hookwars.holdingAddr(itemMint, owner),
      }, { modules: r.modules, royaltyBps }, templates)];
    },
  },
});

/** The launchpad's config as the launch page shows it before anything is signed: the launch fee,
 * the creator fee ceiling, the virtual reserve range and the token rule bounds, all from the chain. */
export async function launchConfig(conn: Connection) {
  const cfg = await conn.getAccountInfo(LAUNCH_CONFIG, 'confirmed');
  if (!cfg) return null;
  const c = decodeLaunchConfig(cfg.data);
  return {
    launchFeeLamports: c.launchFeeLamports.toString(), maxCreatorFeeBps: c.maxCreatorFeeBps, lpFeeBps: c.lpFeeBps,
    minVirtualQuote: c.minVirtualQuote.toString(), maxVirtualQuote: c.maxVirtualQuote.toString(), paused: c.paused,
    sniperWindowSecs: c.sniperWindowSecs, sniperStartBps: c.sniperStartBps, ruleBounds: c.ruleBounds,
  };
}
