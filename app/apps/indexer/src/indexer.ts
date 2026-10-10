// Changed by Hookwars: cursors for agents, market and social; a slot launch's lookup table per mint; social state (postage, profile authors).
/**
 * The indexing loop (06 2.1): one signature cursor per program, oldest first; each transaction is
 * fetched once, its events decoded (self-CPI, and the program logs of items and the launchpad), written keyed by
 * `(signature, ordinal)`, and the state tables updated. Failed transactions are skipped.
 */
import { Connection, PublicKey, type ConfirmedSignatureInfo, type VersionedTransactionResponse } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { PROGRAM_IDS } from '@hookwars/shared';
import type { Pool, PoolClient } from 'pg';
import { colName, eventTable } from './schema.ts';
import { applySocialState } from './social.ts';

/** One cursor per program (06 2.1, plus agents, market and social). */
export const CURSOR_PROGRAMS: { program: string; address: string }[] = [
  { program: 'token', address: PROGRAM_IDS.token },
  { program: 'swap', address: PROGRAM_IDS.swap },
  { program: 'launch', address: PROGRAM_IDS.launch },
  { program: 'kit', address: PROGRAM_IDS.kit },
  { program: 'bridge', address: PROGRAM_IDS.bridge },
  { program: 'companion', address: PROGRAM_IDS.companion },
  { program: 'armory', address: PROGRAM_IDS.armory },
  { program: 'items', address: PROGRAM_IDS.items },
  { program: 'war', address: PROGRAM_IDS.war },
  { program: 'agents', address: PROGRAM_IDS.agents },
  { program: 'market', address: PROGRAM_IDS.market },
  { program: 'social', address: PROGRAM_IDS.social },
];

export interface IndexStats { program: string; signatures: number; transactions: number; events: number; skippedFailed: number }

const toJson = (v: unknown): unknown => JSON.parse(JSON.stringify(v, (_k, x) => (typeof x === 'bigint' ? x.toString() : x)));

function cell(v: unknown): unknown {
  if (v === null || v === undefined) return null;
  if (typeof v === 'object') return JSON.stringify(v);
  return v;
}

export async function ensureCursors(db: Pool): Promise<void> {
  for (const c of CURSOR_PROGRAMS) {
    await db.query('insert into cursors (program, address) values ($1, $2) on conflict (program) do update set address = excluded.address', [c.program, c.address]);
  }
}

/** Signatures newer than the cursor, oldest first. */
async function newSignatures(conn: Connection, address: string, until: string | null): Promise<ConfirmedSignatureInfo[]> {
  const out: ConfirmedSignatureInfo[] = [];
  let before: string | undefined;
  for (;;) {
    const page = await conn.getSignaturesForAddress(new PublicKey(address), { before, until: until ?? undefined, limit: 1000 }, 'confirmed');
    out.push(...page);
    if (page.length < 1000) break;
    before = page[page.length - 1]!.signature;
  }
  return out.reverse();
}

/** The address lookup tables a transaction loaded, other than the protocol's. */
export function tablesOf(tx: VersionedTransactionResponse, protocolTable: string | null = process.env.PROTOCOL_LOOKUP_TABLE ?? null): string[] {
  const msg = tx.transaction.message as { addressTableLookups?: { accountKey: PublicKey }[] };
  return (msg.addressTableLookups ?? []).map((l) => l.accountKey.toBase58()).filter((k) => k !== protocolTable);
}

function keysOf(tx: VersionedTransactionResponse): string[] {
  const msg = tx.transaction.message;
  const keys = msg.staticAccountKeys.map((k) => k.toBase58());
  const loaded = tx.meta?.loadedAddresses;
  if (loaded) keys.push(...loaded.writable.map((k) => k.toBase58()), ...loaded.readonly.map((k) => k.toBase58()));
  return keys;
}

export async function writeTransaction(client: PoolClient, signature: string, slot: number, blockTime: number | null, events: hookwars.TxEvent[], truncated: boolean, programs: string[], tables: string[] = []): Promise<void> {
  const bt = blockTime === null ? null : new Date(blockTime * 1000);
  await client.query('insert into transactions (signature, slot, block_time, logs_truncated, programs) values ($1,$2,$3,$4,$5) on conflict (signature) do nothing', [signature, slot, bt, truncated, programs]);
  for (const ev of events) {
    const ts = typeof ev.data.ts === 'string' ? ev.data.ts : blockTime;
    await client.query(
      'insert into events (signature, ordinal, slot, ts, block_time, program, program_id, name, via, data) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) on conflict do nothing',
      [signature, ev.ordinal, slot, ts, bt, ev.program, ev.programId, ev.name, ev.via, toJson(ev.data)],
    );
    const spec = hookwars.EVENT_SPECS[ev.program]?.find(([n]) => n === ev.name);
    if (spec) {
      const fields = spec[1].flatMap(([f, ty]) => { const c = colName(f, ty); return c ? [[f, c] as const] : []; });
      const cols = fields.map(([, c]) => `"${c}"`);
      const vals = fields.map(([f]) => cell(ev.data[f]));
      const ph = vals.map((_, i) => `$${i + 6}`).join(',');
      await client.query(
        `insert into ${eventTable(ev.program, ev.name)} (signature, ordinal, slot, ts, block_time, ${cols.join(',')}) values ($1,$2,$3,$4,$5,${ph}) on conflict do nothing`,
        [signature, ev.ordinal, slot, ts, bt, ...vals],
      );
    }
    await applyState(client, ev, slot);
    await applySocialState(client, signature, ev, slot);
    // A slot launch's own lookup table (the launch stage loads it, 03 M3b): prepares that touch the
    // mint compile with it (app audit A-6).
    if (ev.program === 'launch' && ev.name === 'LaunchCreated' && tables.length > 0) {
      await client.query(`insert into mint_tables (mint, lookup_table, updated_slot) values ($1,$2,$3) on conflict (mint) do update set lookup_table = excluded.lookup_table, updated_slot = excluded.updated_slot`, [String(ev.data.mint), tables[0], slot]);
    }
  }
}

/** State handlers (06 2.3 "Changed" and state tables). */
export async function applyState(client: PoolClient, ev: hookwars.TxEvent, slot: number): Promise<void> {
  const d = ev.data as Record<string, any>;
  const key = `${ev.program}:${ev.name}`;
  switch (key) {
    case 'token:SlotsInitialized':
      for (const [i, s] of (d.slots as any[]).entries()) {
        await client.query(
          `insert into slots (mint, slot, kind, equip_rule, max_cut_bps, may_refuse, may_write_data, may_answer_touch, data_offset, data_len, equip_vault, locked_program, data_epoch, updated_slot)
           values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,0,$13) on conflict (mint, slot) do update set kind = excluded.kind, equip_rule = excluded.equip_rule, max_cut_bps = excluded.max_cut_bps, may_refuse = excluded.may_refuse, may_write_data = excluded.may_write_data, may_answer_touch = excluded.may_answer_touch, data_offset = excluded.data_offset, data_len = excluded.data_len, equip_vault = excluded.equip_vault, locked_program = excluded.locked_program, updated_slot = excluded.updated_slot`,
          [d.mint, i, s.kind, s.equipRule, s.maxCutBps, s.mayRefuse, s.mayWriteData, s.mayAnswerTouch, s.dataOffset, s.dataLen, s.equipVault, s.lockedProgram, slot],
        );
      }
      return;
    case 'token:SlotEquipped':
      await client.query(
        `insert into slots (mint, slot, item, program, flags, pool_flags, data_epoch, updated_slot) values ($1,$2,$3,$4,$5,$6,$7,$8)
         on conflict (mint, slot) do update set item = excluded.item, program = excluded.program, flags = excluded.flags, pool_flags = excluded.pool_flags, data_epoch = excluded.data_epoch, updated_slot = excluded.updated_slot where slots.updated_slot is null or slots.updated_slot <= excluded.updated_slot`,
        [d.mint, d.slot, d.newItem, d.program, d.flags, d.poolFlags, d.dataEpoch, slot],
      );
      return;
    case 'token:VoteLockSet':
      await client.query(
        `insert into vote_locks (holding, mint, owner, amount, until, updated_slot) values ($1,$2,$3,$4,$5,$6) on conflict (holding) do update set amount = excluded.amount, until = excluded.until, updated_slot = excluded.updated_slot`,
        [d.holding, d.mint, d.owner, d.amount, d.until, slot],
      );
      return;
    case 'token:HookDataWritten':
      await client.query(
        `insert into holding_hook_data (holding, mint, owner, data, updated_slot) values ($1,$2,$3,$4,$5) on conflict (holding) do update set data = excluded.data, updated_slot = excluded.updated_slot where holding_hook_data.updated_slot <= excluded.updated_slot`,
        [d.holding, d.mint, d.owner, d.data, slot],
      );
      return;
    case 'token:Transferred':
      // Item ownership: a transfer of an item mint (supply 1) moves the item (02 2.5).
      await client.query(`update item_owners set owner = $2, updated_slot = $3 where item_mint = $1 and (updated_slot is null or updated_slot <= $3)`, [d.mint, d.destinationOwner, slot]);
      return;
    case 'armory:TemplateRegistered':
      await client.query(
        `insert into templates (template_id, program, code_hash, deploy_slot, kind, field_count, field_min, field_max, name, registered_slot) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) on conflict (template_id) do update set program = excluded.program, code_hash = excluded.code_hash, deploy_slot = excluded.deploy_slot, status = 'active'`,
        [d.templateId, d.program, d.codeHash, d.deploySlot, d.kind, d.fieldCount, JSON.stringify(d.fieldMin), JSON.stringify(d.fieldMax), d.name, slot],
      );
      return;
    case 'armory:TemplateRetired':
      await client.query(`update templates set status = 'retired' where template_id = $1`, [d.templateId]);
      return;
    case 'armory:ItemCreated':
      await client.query(
        `insert into items (item, item_mint, template_id, params, manifest, author, royalty_bps, level, source, created_slot) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) on conflict (item) do nothing`,
        [d.item, d.itemMint, d.templateId, JSON.stringify(d.params), JSON.stringify(d.manifest), d.author, d.royaltyBps, d.level, ['authored', 'loot', 'forged'][Number(d.source)] ?? String(d.source), slot],
      );
      await client.query(`insert into item_owners (item_mint, owner, updated_slot) values ($1,$2,$3) on conflict (item_mint) do nothing`, [d.itemMint, d.author, slot]);
      return;
    case 'armory:LootMinted':
      await client.query(`update item_owners set owner = $2 where item_mint = (select item_mint from items where item = $1)`, [d.item, d.owner]);
      return;
    case 'armory:Forged':
      await client.query(`update items set closed = true where item = any($1)`, [d.burned]);
      return;
    case 'armory:ProposalCreated': {
      const proposal = hookwars.proposalAddress(new PublicKey(d.mint), Number(d.slot), BigInt(d.nonce)).toBase58();
      await client.query(
        `insert into proposals (proposal, mint, slot, nonce, proposer, item, vote_end, executable_at, status, votes_for, votes_against, updated_slot)
         values ($1,$2,$3,$4,$5,$6,$7,$8,'open',0,0,$9) on conflict (proposal) do nothing`,
        [proposal, d.mint, d.slot, d.nonce, d.proposer, d.item, d.voteEnd, d.executableAt, slot],
      );
      return;
    }
    case 'armory:ProposalResolved':
      await client.query(`update proposals set status = $2, votes_for = $3, votes_against = $4, eligible = $5, updated_slot = $6 where proposal = $1`,
        [d.proposal, ['open', 'passed', 'failed', 'executed', 'cancelled'][Number(d.status)] ?? String(d.status), d.votesFor, d.votesAgainst, d.eligible, slot]);
      return;
    default:
      return;
  }
}

/** One pass over every cursor. */
export async function indexOnce(conn: Connection, db: Pool, opts: { maxPerProgram?: number } = {}): Promise<IndexStats[]> {
  await ensureCursors(db);
  const stats: IndexStats[] = [];
  const seen = new Set<string>();
  for (const c of CURSOR_PROGRAMS) {
    const st: IndexStats = { program: c.program, signatures: 0, transactions: 0, events: 0, skippedFailed: 0 };
    const cur = await db.query<{ last_signature: string | null }>('select last_signature from cursors where program = $1', [c.program]);
    const until = cur.rows[0]?.last_signature ?? null;
    let sigs = await newSignatures(conn, c.address, until);
    if (opts.maxPerProgram) sigs = sigs.slice(0, opts.maxPerProgram);
    st.signatures = sigs.length;
    for (const s of sigs) {
      if (s.err) { st.skippedFailed++; continue; }
      if (!seen.has(s.signature)) {
        seen.add(s.signature);
        const tx = await conn.getTransaction(s.signature, { maxSupportedTransactionVersion: 0, commitment: 'confirmed' });
        if (tx && !tx.meta?.err) {
          const inner = (tx.meta?.innerInstructions ?? []).flatMap((g) => g.instructions.map((ix) => ({ programIdIndex: ix.programIdIndex, accounts: ix.accounts, data: ix.data })));
          const { events, truncated } = hookwars.decodeTransactionEvents({ accountKeys: keysOf(tx), inner, logs: tx.meta?.logMessages ?? [] });
          const client = await db.connect();
          try {
            await client.query('begin');
            await writeTransaction(client, s.signature, tx.slot, tx.blockTime ?? null, events, truncated, [c.program], tablesOf(tx));
            await client.query('commit');
          } catch (e) {
            await client.query('rollback');
            throw e;
          } finally {
            client.release();
          }
          st.transactions++;
          st.events += events.length;
        }
      }
      await db.query('update cursors set last_signature = $2, last_slot = $3, updated_at = now() where program = $1', [c.program, s.signature, s.slot]);
    }
    stats.push(st);
  }
  return stats;
}

/** Program info (06 2.1): deployed or not, upgrade authority, from ProgramData. */
export async function refreshProgramInfo(conn: Connection, db: Pool): Promise<void> {
  const keys = CURSOR_PROGRAMS.map((c) => new PublicKey(c.address));
  const infos = await conn.getMultipleAccountsInfo(keys, 'confirmed');
  for (const [i, c] of CURSOR_PROGRAMS.entries()) {
    const info = infos[i];
    let authority: string | null = null;
    if (info?.executable && info.data.length >= 36) {
      const pd = new PublicKey(info.data.subarray(4, 36));
      const pdInfo = await conn.getAccountInfo(pd, 'confirmed');
      if (pdInfo && pdInfo.data.length >= 45 && pdInfo.data[12] === 1) authority = new PublicKey(pdInfo.data.subarray(13, 45)).toBase58();
    }
    await db.query(
      `insert into program_info (program, address, deployed, upgrade_authority, checked_at) values ($1,$2,$3,$4, now()) on conflict (program) do update set deployed = excluded.deployed, upgrade_authority = excluded.upgrade_authority, checked_at = now()`,
      [c.program, c.address, Boolean(info?.executable), authority],
    );
  }
}
