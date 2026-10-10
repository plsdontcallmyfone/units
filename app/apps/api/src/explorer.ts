// Changed by Hookwars: new file, the explorer reads (explorer v2).
/**
 * Explorer reads: search, a decoded transaction, an account with what the indexer knows about it,
 * and a program's recent activity. Everything is read from the cluster; the indexer only adds
 * history (events naming the address, slot rows, item rows) and is optional: without it the pages
 * still show what the chain holds, and say the history is not available.
 */
import { Connection, PublicKey, type VersionedTransactionResponse } from '@solana/web3.js';
import type { Pool } from 'pg';
import bs58 from 'bs58';
import { hookwars } from '@hookwars/sdk';
import { HttpError, intParam } from './guard.ts';

const { explore } = hookwars;

/** The explorer's source shape from a `getTransaction` answer (json, max version 0). */
export function txSourceOf(signature: string, r: VersionedTransactionResponse): hookwars.explore.TxSource {
  const msg = r.transaction.message;
  const loaded = r.meta?.loadedAddresses;
  const keys = [
    ...msg.staticAccountKeys.map((k) => k.toBase58()),
    ...(loaded?.writable ?? []).map((k) => k.toBase58()),
    ...(loaded?.readonly ?? []).map((k) => k.toBase58()),
  ];
  const writable = keys.map((_, i) => { try { return msg.isAccountWritable(i); } catch { return false; } });
  const signer = keys.map((_, i) => { try { return msg.isAccountSigner(i); } catch { return false; } });
  return {
    signature, slot: r.slot, blockTime: r.blockTime ?? null, err: r.meta?.err ?? null, fee: r.meta?.fee ?? null,
    computeUnits: r.meta?.computeUnitsConsumed ?? null,
    accountKeys: keys, writable, signer,
    instructions: msg.compiledInstructions.map((ix) => ({ programIdIndex: ix.programIdIndex, accounts: [...ix.accountKeyIndexes], data: bs58.encode(ix.data) })),
    inner: (r.meta?.innerInstructions ?? []).map((g) => ({
      index: g.index,
      instructions: g.instructions.map((c) => ({ programIdIndex: c.programIdIndex, accounts: c.accounts, data: c.data, stackHeight: (c as { stackHeight?: number | null }).stackHeight ?? null })),
    })),
    logs: r.meta?.logMessages ?? [],
  };
}

export async function transaction(conn: Connection, signature: string) {
  if (explore.classifyQuery(signature).kind !== 'signature') throw new HttpError(400, 'BadSignature', 'That is not a transaction signature.');
  const r = await conn.getTransaction(signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 });
  if (!r) return null;
  return explore.explainTransaction(txSourceOf(signature, r));
}

type EventRow = { signature: string; ordinal: number; slot: string | number; program: string; name: string; data: Record<string, unknown> };
/** Fields of an indexed event that name an address the explorer links to. */
const ADDRESS_FIELDS = ['mint', 'item', 'owner', 'trader', 'passport', 'pool', 'seller', 'buyer', 'creator', 'rival', 'rivalMint', 'author', 'lessor', 'agent', 'from', 'to'];

/** Indexed events naming an address in a top-level field, newest first. */
export async function eventsNaming(db: Pool, key: string, limit = 50): Promise<EventRow[]> {
  const where = ADDRESS_FIELDS.map((f) => `data->>'${f}' = $1`).join(' or ');
  return (await db.query(`select signature, ordinal, slot, program, name, data from events where ${where} order by slot desc, ordinal desc limit ${limit}`, [key])).rows as EventRow[];
}

export interface Signature { signature: string; slot: number; blockTime: number | null; ok: boolean; memo: string | null }
async function recentSignatures(conn: Connection, key: PublicKey, limit: number): Promise<Signature[]> {
  const sigs = await conn.getSignaturesForAddress(key, { limit }, 'confirmed');
  return sigs.map((s) => ({ signature: s.signature, slot: s.slot, blockTime: s.blockTime ?? null, ok: s.err === null, memo: s.memo ?? null }));
}

/** What an account is, in the explorer's words, from its owner and decoded type. */
export function kindOf(program: string | null, type: string | null, executable: boolean): string {
  if (executable) return 'program';
  if (!program) return 'wallet or other account';
  const k: Record<string, string> = {
    'token.Mint': 'mint', 'token.Holding': 'holding', 'swap.Pool': 'pool', 'armory.Item': 'item', 'armory.Template': 'template',
    'armory.SlotState': 'slot', 'armory.Proposal': 'proposal', 'armory.CompositeItem': 'composite item', 'agents.Passport': 'passport',
    'war.WarState': 'war state', 'war.Season': 'season', 'items.EquipState': 'equipped item',
  };
  return k[`${program}.${type}`] ?? (type ? `${type}` : `${program} account`);
}

export async function address(conn: Connection, db: Pool | null, key: string) {
  let pk: PublicKey;
  try { pk = new PublicKey(key); } catch { throw new HttpError(400, 'BadAddress', 'That is not an address.'); }
  const info = await conn.getAccountInfo(pk, 'confirmed');
  const owner = info?.owner.toBase58() ?? null;
  const decoded = info && owner ? explore.decodeAccount(owner, Buffer.from(info.data)) : { program: null, type: null, data: null };
  const program = info?.executable ? explore.programName(key) : null;
  const kind = info ? kindOf(decoded.program, decoded.type, info.executable) : 'empty';
  const extra: Record<string, unknown> = {};
  if (info && decoded.program === 'swap' && decoded.type === 'Pool') {
    try { extra.ring = explore.ringSummary(hookwars.decodePoolObservations(Buffer.from(info.data))); } catch { extra.ring = null; }
  }
  if (info && decoded.program === 'token' && decoded.type === 'Mint') {
    // The war chest and pool items ledger of a mint, when they exist (05, 04 2.9).
    const [chest, war] = await conn.getMultipleAccountsInfo([hookwars.warChestAddress(pk), hookwars.warStateAddress(pk)], 'confirmed');
    extra.warChest = chest ? { address: hookwars.warChestAddress(pk).toBase58(), lamports: String(chest.lamports) } : null;
    extra.warState = war ? hookwars.warStateAddress(pk).toBase58() : null;
  }
  let indexed: Record<string, unknown> | null = null;
  if (db) {
    indexed = { events: (await eventsNaming(db, key)).map((r) => ({ ...r, slot: Number(r.slot), story: explore.storyOf({ program: r.program, name: r.name, data: r.data }) })) };
    if (kind === 'mint') indexed.slots = (await db.query('select s.*, i.template_id, i.params, i.manifest, i.author, i.royalty_bps, i.level from slots s left join items i on i.item = s.item where s.mint = $1 order by s.slot', [key])).rows;
    if (kind === 'item') {
      indexed.item = (await db.query('select * from items where item = $1', [key])).rows[0] ?? null;
      indexed.owner = (await db.query('select o.* from item_owners o join items i on i.item_mint = o.item_mint where i.item = $1', [key])).rows[0] ?? null;
      indexed.equippedOn = (await db.query('select mint, slot from slots where item = $1', [key])).rows;
    }
    if (kind === 'template' && decoded.data) indexed.template = (await db.query('select * from templates where template_id = $1', [Number((decoded.data as Record<string, unknown>).templateId ?? -1)])).rows[0] ?? null;
  }
  return {
    address: key, exists: info !== null, lamports: info ? String(info.lamports) : null, owner, ownerName: owner ? explore.programName(owner) : null,
    executable: info?.executable ?? false, space: info ? info.data.length : null, kind, program,
    decoded, extra, indexed,
    recent: await recentSignatures(conn, pk, 25),
  };
}

export async function programPage(conn: Connection, db: Pool | null, id: string, limit: number) {
  let pk: PublicKey;
  try { pk = new PublicKey(id); } catch { throw new HttpError(400, 'BadAddress', 'That is not an address.'); }
  const name = explore.programName(id);
  const info = await conn.getAccountInfo(pk, 'confirmed');
  const counts = db && name ? (await db.query('select name, count(*)::int as n, max(slot) as last_slot from events where program_id = $1 or program = $2 group by name order by n desc', [id, name])).rows : null;
  return {
    address: id, name, title: name ? explore.PROGRAM_TITLES[name] ?? name : null, deployed: info?.executable ?? false,
    instructions: name ? (explore.EXPLORER_IDLS[name]?.instructions.map((x) => x.name) ?? []) : [],
    events: name ? (explore.EXPLORER_IDLS[name]?.events?.map((x) => x.name) ?? []) : [],
    indexedCounts: counts,
    recent: await recentSignatures(conn, pk, limit),
  };
}

export async function search(conn: Connection, db: Pool | null, q: string) {
  const t = q.trim();
  // "template 12" or "#12": a template by id (02 section 3).
  const tm = /^(?:template\s*|#)(\d{1,5})$/i.exec(t);
  if (tm) {
    const id = intParam(tm[1], 'template', 0, 65_535);
    const a = hookwars.templateAddress(id).toBase58();
    return { kind: 'template', value: a, href: `/address/${a}` };
  }
  const c = explore.classifyQuery(t);
  if (c.kind === 'signature') return { kind: 'transaction', value: c.value, href: `/tx/${c.value}` };
  if (c.kind === 'program') return { kind: 'program', value: c.value, href: `/program/${c.value}` };
  if (c.kind === 'address') {
    // An item mint resolves to its item account (armory `Item` at ["item", item mint]).
    const info = await conn.getAccountInfo(new PublicKey(c.value), 'confirmed');
    if (info?.executable) return { kind: 'program', value: c.value, href: `/program/${c.value}` };
    if (db) {
      const r = (await db.query('select item from items where item_mint = $1', [c.value])).rows[0];
      if (r) return { kind: 'item', value: r.item as string, href: `/address/${r.item as string}` };
    }
    const owner = info?.owner.toBase58();
    const d = info && owner ? explore.decodeAccount(owner, Buffer.from(info.data)) : null;
    return { kind: info ? kindOf(d?.program ?? null, d?.type ?? null, false) : 'empty', value: c.value, href: `/address/${c.value}` };
  }
  return { kind: 'invalid', value: t, href: null };
}

/** Routes for /v1/explorer/*, or null when the path is not one. */
export async function route(conn: Connection, db: Pool | null, p: string, q: URLSearchParams): Promise<{ status: number; body: unknown } | null> {
  let m: RegExpExecArray | null;
  if (p === '/v1/explorer/search') return { status: 200, body: await search(conn, db, (q.get('q') ?? '').slice(0, 120)) };
  if ((m = /^\/v1\/explorer\/tx\/(\w{43,90})$/.exec(p))) {
    const t = await transaction(conn, m[1]!);
    return t ? { status: 200, body: t } : { status: 404, body: { error: 'The cluster has no such transaction.' } };
  }
  if ((m = /^\/v1\/explorer\/address\/(\w{32,44})$/.exec(p))) return { status: 200, body: await address(conn, db, m[1]!) };
  if ((m = /^\/v1\/explorer\/program\/(\w{32,44})$/.exec(p))) return { status: 200, body: await programPage(conn, db, m[1]!, intParam(q.get('limit') ?? '25', 'limit', 1, 100)) };
  if (p === '/v1/explorer/programs') {
    return { status: 200, body: Object.entries(explore.EXPLORER_IDLS).map(([name, idl]) => ({ name, title: explore.PROGRAM_TITLES[name] ?? name, address: idl.address })) };
  }
  return null;
}
