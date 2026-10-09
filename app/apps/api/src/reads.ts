/**
 * The read routes of docs/spec/06-app.md 3.3, built from the indexer's tables. Every figure comes
 * from a row or an account; what the backend cannot read is `null` (06 section 1 rule 3).
 */
import { intParam } from './guard.ts';
import type { Pool } from 'pg';
import {
  TEMPLATES, decodeRange, itemSentence, PARAMS, SLOT_KINDS, EQUIP_RULES, rollingRaidVolume,
  type BattleEvent, type BattleKind, type General, type HookwarsStatus, type ItemSummary, type ProposalInfo, type QuestInfo, type SlotInfo,
  type TemplateInfo, type WarMap, type Page, type SlotRangeSpec,
} from '@hookwars/shared';

const num = (v: unknown): number => Number(v ?? 0);
const str = (v: unknown): string => (v === null || v === undefined ? '' : String(v));

export async function templates(db: Pool): Promise<TemplateInfo[]> {
  const rows = (await db.query('select t.*, (select count(*)::int from items i where i.template_id = t.template_id and not i.closed) as n from templates t order by template_id')).rows;
  return rows.map((r) => {
    const def = TEMPLATES.find((t) => t.id === r.template_id);
    const min = (r.field_min ?? []) as number[];
    const max = (r.field_max ?? []) as number[];
    return {
      templateId: r.template_id,
      name: r.name ?? def?.name ?? `Template ${r.template_id}`,
      sentence: def ? itemSentence(def.id, def.fields.map((f) => min[f.index] ?? 0)) : '',
      program: r.program,
      kind: SLOT_KINDS[r.kind] ?? 'fee',
      fields: (def?.fields ?? []).map((f) => ({ index: f.index, name: f.name, min: min[f.index] ?? 0, max: max[f.index] ?? 0, forge: f.forge })),
      codeHash: str(r.code_hash),
      verified: null,
      upgradeAuthority: null,
      upgradeable: r.deploy_slot === null ? false : null,
      status: r.status === 'retired' ? 'retired' : 'active',
      openAuthoring: false, lootEnabled: false, forgeEnabled: def?.forgeable ?? false, maxLevel: 0,
      items: r.n,
    } satisfies TemplateInfo;
  });
}

function itemRow(r: any): ItemSummary {
  const params = (r.params ?? []) as number[];
  const def = TEMPLATES.find((t) => t.id === r.template_id);
  const m = r.manifest ?? {};
  return {
    item: r.item, itemMint: r.item_mint, templateId: r.template_id, templateName: def?.name ?? `Template ${r.template_id}`,
    kind: SLOT_KINDS[num(m.kind)] ?? def?.kind ?? 'fee', params, paramsText: itemSentence(r.template_id, params),
    manifest: {
      kind: SLOT_KINDS[num(m.kind)] ?? 'fee', tokenFlags: num(m.tokenFlags), poolFlags: num(m.poolFlags), maxCutBuyBps: num(m.maxCutBuyBps),
      maxCutSellBps: num(m.maxCutSellBps), maxCutTransferBps: num(m.maxCutTransferBps), maxDiscountBps: num(m.maxDiscountBps),
      mayRefuse: Boolean(m.mayRefuse), mayBurn: Boolean(m.mayBurn), dataBytes: num(m.dataBytes), readsOtherPools: num(m.readsOtherPools), marks: (num(m.poolFlags) & 4) !== 0,
    },
    level: num(r.level), source: (r.source ?? 'authored') as ItemSummary['source'], author: r.author, owner: r.owner ?? null,
    royaltyBps: num(r.royalty_bps), runs: null, royalties: [], equippedOn: [], closed: Boolean(r.closed),
  };
}

export async function items(db: Pool, q: URLSearchParams): Promise<Page<ItemSummary>> {
  const where: string[] = []; const args: unknown[] = [];
  if (q.get('template')) { args.push(Number(q.get('template'))); where.push(`i.template_id = $${args.length}`); }
  if (q.get('owner')) { args.push(q.get('owner')); where.push(`o.owner = $${args.length}`); }
  if (q.get('source')) { args.push(q.get('source')); where.push(`i.source = $${args.length}`); }
  if (q.get('closed') !== '1') where.push('not i.closed');
  const sql = `select i.*, o.owner from items i left join item_owners o using (item_mint) ${where.length ? 'where ' + where.join(' and ') : ''} order by i.created_slot desc nulls last limit 100`;
  return { items: (await db.query(sql, args)).rows.map(itemRow), next: null };
}

export async function item(db: Pool, id: string): Promise<(ItemSummary & { history: BattleEvent[] }) | null> {
  const r = (await db.query('select i.*, o.owner from items i left join item_owners o using (item_mint) where i.item = $1', [id])).rows[0];
  if (!r) return null;
  const hist = (await db.query(`select * from events where data->>'item' = $1 order by slot desc, ordinal desc limit 50`, [id])).rows.map(battleRow).filter((x): x is BattleEvent => x !== null);
  return { ...itemRow(r), history: hist };
}

export async function slots(db: Pool, mint: string): Promise<SlotInfo[]> {
  const rows = (await db.query('select * from slots where mint = $1 order by slot', [mint])).rows;
  const props = await proposals(db, mint, 'open');
  const itemRows = new Map((await db.query('select i.*, o.owner from items i left join item_owners o using (item_mint) where i.item = any($1)', [rows.map((r) => r.item).filter(Boolean)])).rows.map((r) => [r.item, r]));
  return rows.map((r) => {
    const kind = SLOT_KINDS[num(r.kind)] ?? 'fee';
    const it = r.item && r.item !== '11111111111111111111111111111111' ? itemRows.get(r.item) : undefined;
    return {
      slot: r.slot, kind, equipRule: EQUIP_RULES[num(r.equip_rule)] ?? 'locked',
      bounds: { maxCutBps: num(r.max_cut_bps), mayRefuse: Boolean(r.may_refuse), mayWriteData: Boolean(r.may_write_data), mayAnswerTouch: Boolean(r.may_answer_touch) },
      noticeSecs: null, dataRange: { offset: num(r.data_offset), len: num(r.data_len) }, dataEpoch: num(r.data_epoch),
      item: it ? itemRow(it) : null, targets: [], role: 'none',
      locked: kind === 'locked' && r.locked_program ? { program: r.locked_program, label: 'kit' } : null,
      launchItem: null, performance: null,
      openProposal: props.find((p) => p.slot === r.slot) ?? null,
    } satisfies SlotInfo;
  });
}

export async function proposals(db: Pool, mint: string, status?: string | null): Promise<ProposalInfo[]> {
  const args: unknown[] = [mint];
  let sql = 'select * from proposals where mint = $1';
  if (status) { args.push(status); sql += ' and status = $2'; }
  const rows = (await db.query(sql + ' order by vote_end desc', args)).rows;
  return rows.map((r) => ({
    proposal: r.proposal, mint: r.mint, slot: r.slot, nonce: num(r.nonce), proposer: r.proposer, item: null,
    voteEnd: num(r.vote_end), executableAt: num(r.executable_at), status: r.status,
    votesFor: str(r.votes_for), votesAgainst: str(r.votes_against), eligibleNow: null,
    quorumBps: PARAMS.find((p) => p.name === 'VOTE_QUORUM_BPS')?.value ?? null, needsSettleFirst: false, myVote: null,
  }));
}

const KIND_OF: Record<string, BattleKind> = {
  RaidMarked: 'raid', SiegeExecuted: 'siege', SiegeWaited: 'siege_waited', CounterStrikeExecuted: 'counter_strike', Razed: 'raze',
  CapturedReturned: 'return', TreatyInflowShared: 'treaty_shared', EquipApplied: 'equip', ProposalCreated: 'proposal', EquipSettled: 'settle',
  BountyClaimed: 'bounty', RollRequested: 'roll', RollRevealed: 'loot', Forged: 'forge', QuestClaimed: 'quest', SeasonFinalized: 'season', PrizePaid: 'prize',
};

function battleRow(r: any): BattleEvent | null {
  const kind = KIND_OF[r.name];
  if (!kind) return null;
  const d = r.data ?? {};
  const amount = d.spent ?? d.volume ?? d.paid ?? d.amount ?? d.toWinner ?? null;
  return {
    kind, ts: num(r.ts ?? (r.block_time ? Math.floor(new Date(r.block_time).getTime() / 1000) : 0)), signature: r.signature, ordinal: r.ordinal,
    mint: str(d.mint ?? d.winner), otherMint: d.rivalMint ?? d.rival ?? null, actor: d.cranker ?? d.trader ?? d.owner ?? d.forger ?? null,
    amount: amount === null ? null : String(amount), detail: {},
  };
}

export async function feed(db: Pool, q: URLSearchParams): Promise<Page<BattleEvent>> {
  const args: unknown[] = [Object.keys(KIND_OF)];
  let sql = 'select * from events where name = any($1)';
  if (q.get('mint')) { args.push(q.get('mint')); sql += ` and (data->>'mint' = $${args.length} or data->>'rivalMint' = $${args.length})`; }
  if (q.get('before') !== null) { args.push(intParam(q.get('before'), 'before', 0, Number.MAX_SAFE_INTEGER)); sql += ` and slot < $${args.length}`; }
  const rows = (await db.query(sql + ' order by slot desc, ordinal desc limit 100', args)).rows;
  const events = rows.map(battleRow).filter((x): x is BattleEvent => x !== null).filter((e) => !q.get('kind') || e.kind === q.get('kind'));
  return { items: events, next: rows.length === 100 ? String(rows[rows.length - 1].slot) : null };
}

/** How far back the war map looks, and how many edges of each kind it shows (A-7; operator settings). */
const MAP_WINDOW_SECS = Number(process.env.MAP_WINDOW_SECS ?? 30 * 86_400);
const MAP_EDGE_LIMIT = 500;
/** Holdings scanned for generals per request (A-7). */
const GENERALS_SCAN_LIMIT = 20_000;

/** 06 2.4 war map. Edges come only from indexed rows; with none, the map is empty. */
export async function warMap(db: Pool, now: number): Promise<WarMap> {
  const edges: WarMap['edges'] = [];
  // A-7: a bounded window and row count; the route also caches the answer.
  const since = now - MAP_WINDOW_SECS;
  const raids = (await db.query(`select mint, rival, sum(volume)::text as v, min(ts) as since from raids where ts >= $1 group by mint, rival order by sum(volume) desc limit ${MAP_EDGE_LIMIT}`, [since])).rows;
  for (const r of raids) edges.push({ kind: 'raid', from: r.mint, to: r.rival, weight: r.v, since: r.since === null ? null : num(r.since) });
  const sieges = (await db.query(`select mint, rival_mint, max(ts) as at from ev_war_siege_executed where ts >= $1 group by mint, rival_mint order by max(ts) desc limit ${MAP_EDGE_LIMIT}`, [since])).rows;
  for (const s of sieges) edges.push({ kind: 'siege', from: s.mint, to: s.rival_mint, weight: null, since: s.at === null ? null : num(s.at) });
  const mints = new Set<string>();
  for (const e of edges) { mints.add(e.from); mints.add(e.to); }
  return { nodes: [...mints].map((m) => ({ mint: m, symbol: m.slice(0, 4), image: null, chest: null, underSiege: false })), edges };
}

export async function generals(db: Pool, mint: string, currentSeason: number | null): Promise<General[]> {
  const s = (await db.query('select slot, kind, data_offset, data_len, data_epoch from slots where mint = $1', [mint])).rows;
  const specs: SlotRangeSpec[] = s.map((r) => ({ slot: r.slot, kind: num(r.kind), offset: num(r.data_offset), len: num(r.data_len), dataEpoch: num(r.data_epoch), templateId: null }));
  const holdings = (await db.query('select owner, data from holding_hook_data where mint = $1 limit $2', [mint, GENERALS_SCAN_LIMIT])).rows;
  const vol = new Map((await db.query('select trader, sum(volume)::text as v from raids where mint = $1 group by trader', [mint])).rows.map((r) => [r.trader, r.v]));
  const out: General[] = [];
  for (const h of holdings) {
    const bytes = Buffer.from(String(h.data).replace(/^0x/, ''), 'hex');
    const raid = specs.map((sp) => decodeRange(bytes, sp, currentSeason)).find((r) => r.type === 'raid');
    if (raid && raid.type === 'raid' && raid.raidPoints > 0) out.push({ owner: h.owner, raidPoints: raid.raidPoints, raidVolume: vol.get(h.owner) ?? '0', rank: 0 });
  }
  out.sort((a, b) => b.raidPoints - a.raidPoints || Number(BigInt(b.raidVolume) - BigInt(a.raidVolume)));
  return out.map((g, i) => ({ ...g, rank: i + 1 }));
}

export const QUESTS: QuestInfo[] = [
  { questId: 1, name: 'Raid', sentence: 'Hold QUEST_RAID_POINTS raid points this season in one token; claiming spends them and adds one loot ticket. Once per QUEST_PERIOD_SECS per token.' },
  { questId: 2, name: 'Forge', sentence: 'Forge an item since your last Forge claim; claiming adds one loot ticket to the token you name. Once per QUEST_PERIOD_SECS.' },
];

export async function status(db: Pool | null, rpc: { cluster: string; slot: () => Promise<number> }): Promise<HookwarsStatus> {
  let slot: number | null = null;
  let rpcReachable = false;
  try { slot = await rpc.slot(); rpcReachable = true; } catch { /* unreachable */ }
  let programs: HookwarsStatus['programs'] = [];
  let indexer: HookwarsStatus['indexer'] = [];
  let database = false;
  if (db) {
    try {
      programs = (await db.query('select program, address, deployed, upgrade_authority, executable_hash from program_info order by program')).rows
        .map((r) => ({ name: r.program, address: r.address, deployed: r.deployed, upgradeAuthority: r.upgrade_authority, executableHash: r.executable_hash }));
      indexer = (await db.query('select program, last_signature, last_slot, updated_at from cursors order by program')).rows
        .map((r) => ({ program: r.program, cursor: r.last_signature, lastSlot: r.last_slot === null ? null : num(r.last_slot), updatedAt: r.updated_at ? Math.floor(new Date(r.updated_at).getTime() / 1000) : null }));
      database = true;
    } catch { database = false; }
  }
  return { cluster: rpc.cluster, rpcReachable, slot, programs, indexer, database };
}

/** 05 2.5 rolling inbound volume per rival from the latest raid-ledger snapshot, when one exists. */
export function rolling(volume: bigint, prev: bigint, windowStart: bigint, t: bigint, windowSecs: bigint | null): bigint | null {
  if (windowSecs === null) return null;
  return rollingRaidVolume({ windowStart, volume, prevVolume: prev }, t, windowSecs);
}
