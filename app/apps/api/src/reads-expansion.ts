// Changed by Hookwars: new file, reads for the agents, market and social programs (09, 10). Account
// state comes from the chain (getProgramAccounts filtered by the account's discriminator, bounded);
// history (sales, awards) comes from the indexer's event tables. Nothing is estimated: a figure the
// chain or the indexer does not hold is null.
import { PublicKey, type Connection, type GetProgramAccountsFilter } from '@solana/web3.js';
import bs58 from 'bs58';
import type { Pool } from 'pg';
import { hookwars } from '@hookwars/sdk';

/** At most this many accounts of one kind per read (app audit A-7). */
export const MAX_ROWS = 200;

const json = (v: unknown): unknown => JSON.parse(JSON.stringify(v, (_k, x) => (typeof x === 'bigint' ? x.toString() : x instanceof PublicKey ? x.toBase58() : x)));

async function accountsOf<T>(conn: Connection, program: PublicKey, codec: hookwars.AccountCodec<T>, filters: GetProgramAccountsFilter[] = []): Promise<{ address: string; data: T }[]> {
  const res = await conn.getProgramAccounts(program, { commitment: 'confirmed', filters: [{ memcmp: { offset: 0, bytes: bs58.encode(codec.disc) } }, ...filters] });
  const out: { address: string; data: T }[] = [];
  for (const r of res.slice(0, MAX_ROWS)) {
    try { out.push({ address: r.pubkey.toBase58(), data: codec.decode(r.account.data) }); } catch { /* another layout under the same name */ }
  }
  return out;
}
const at = (offset: number, key: PublicKey | string): GetProgramAccountsFilter => ({ memcmp: { offset, bytes: typeof key === 'string' ? key : key.toBase58() } });

async function safeRows<T>(db: Pool | null, sql: string, args: unknown[]): Promise<T[]> {
  if (!db) return [];
  try { return (await db.query(sql, args)).rows as T[]; } catch { return []; }
}

// ---------------------------------------------------------------- agents --

const RECORD_KEYS = ['itemsAuthored', 'royaltiesClaimedSol', 'treatiesHeld', 'cranks', 'bountiesClaimedLamports'] as const;
export type LeagueSort = typeof RECORD_KEYS[number];

/** The agent league (09 10): every passport by a track-record counter. No prize, no score. */
export async function agentsLeague(conn: Connection, sort: string | null): Promise<unknown> {
  const key: LeagueSort = (RECORD_KEYS as readonly string[]).includes(sort ?? '') ? sort as LeagueSort : 'itemsAuthored';
  const rows = await accountsOf(conn, hookwars.AGENTS_ID, hookwars.passportCodec);
  const v = (r: { data: hookwars.PassportData }): bigint => BigInt((r.data.record as unknown as Record<string, bigint | number>)[key] ?? 0);
  rows.sort((a, b) => (v(b) > v(a) ? 1 : v(b) < v(a) ? -1 : 0));
  return json({ sort: key, sorts: RECORD_KEYS, items: rows.map((r) => ({ passport: r.address, ...r.data })) });
}

/** One agent: passport, links, attestation, wallet policy and bonds. */
export async function agent(conn: Connection, passportKey: string): Promise<unknown> {
  const passport = new PublicKey(passportKey);
  const info = await conn.getAccountInfo(passport, 'confirmed');
  if (!info) return null;
  const p = hookwars.passportCodec.decode(info.data);
  const [links, bonds, att, pol, vault] = await Promise.all([
    accountsOf(conn, hookwars.AGENTS_ID, hookwars.linkCodec, [at(8, passport)]),
    accountsOf(conn, hookwars.AGENTS_ID, hookwars.bondCodec, [at(8, passport)]),
    conn.getAccountInfo(hookwars.attestationAddress(passport), 'confirmed'),
    conn.getAccountInfo(hookwars.policyAddress(passport), 'confirmed'),
    conn.getAccountInfo(hookwars.agentVaultAddress(passport), 'confirmed'),
  ]);
  return json({
    passport: passportKey, ...p,
    links: links.map((l) => ({ address: l.address, ...l.data })),
    attestation: att ? hookwars.attestationCodec.decode(att.data) : null,
    policy: pol ? hookwars.policyCodec.decode(pol.data) : null,
    vault: hookwars.agentVaultAddress(passport).toBase58(),
    vaultLamports: vault ? String(vault.lamports) : null,
    bonds: bonds.map((x) => ({ address: x.address, ...x.data })),
  });
}

// ---------------------------------------------------------------- market --

export async function listings(conn: Connection, db: Pool | null): Promise<unknown> {
  const rows = await accountsOf(conn, hookwars.MARKET_ID, hookwars.listingCodec);
  const items = await safeRows<{ item: string; template_id: number; params: unknown; level: number }>(db, 'select item, template_id, params, level from items where item = any($1)', [rows.map((r) => r.data.item.toBase58())]);
  const byItem = new Map(items.map((i) => [i.item, i]));
  return json({ items: rows.map((r) => ({ listing: r.address, ...r.data, template: byItem.get(r.data.item.toBase58()) ?? null })) });
}

/** One item for sale or not: its listing, its sales (the only price history), its lineage from forges. */
export async function marketItem(conn: Connection, db: Pool | null, itemMintKey: string): Promise<unknown> {
  const itemMint = new PublicKey(itemMintKey);
  const item = hookwars.itemAddress(itemMint);
  const [itemInfo, listing, lease] = await Promise.all([
    conn.getAccountInfo(item, 'confirmed'),
    conn.getAccountInfo(hookwars.listingAddress(itemMint), 'confirmed'),
    conn.getAccountInfo(hookwars.leaseAddress(item), 'confirmed'),
  ]);
  if (!itemInfo) return null;
  const sales = await safeRows<Record<string, unknown>>(db, 'select signature, slot, ts, seller, buyer, price, fee, resale from ev_market_sold where item_mint = $1 order by slot asc limit 500', [itemMintKey]);
  const lineage = await lineageOf(db, item.toBase58());
  return json({
    item: item.toBase58(), itemMint: itemMintKey, data: hookwars.itemCodec().decode(itemInfo.data),
    listing: listing ? hookwars.listingCodec.decode(listing.data) : null,
    lease: lease ? hookwars.leaseCodec.decode(lease.data) : null,
    sales, lineage,
  });
}

/** Parents (the items burned to forge this one, recursively) and children (forges that burned it). */
export async function lineageOf(db: Pool | null, item: string, depth = 4): Promise<{ item: string; level: number | null; parents: unknown[] } & { children: string[] }> {
  const node = async (key: string, d: number): Promise<{ item: string; level: number | null; parents: unknown[] }> => {
    const f = (await safeRows<{ burned: unknown; level: number }>(db, 'select burned, level from ev_armory_forged where item = $1 limit 1', [key]))[0];
    if (!f || d <= 0) return { item: key, level: f ? Number(f.level) : null, parents: [] };
    const burned = (typeof f.burned === 'string' ? JSON.parse(f.burned) : f.burned) as string[];
    return { item: key, level: Number(f.level), parents: await Promise.all((burned ?? []).map((b) => node(b, d - 1))) };
  };
  const children = (await safeRows<{ item: string }>(db, `select item from ev_armory_forged where burned @> to_jsonb(array[$1::text]) limit 20`, [item])).map((r) => r.item);
  return { ...(await node(item, depth)), children };
}

export async function leases(conn: Connection): Promise<unknown> {
  return json({ items: (await accountsOf(conn, hookwars.MARKET_ID, hookwars.leaseCodec)).map((r) => ({ lease: r.address, ...r.data })) });
}
export async function collections(conn: Connection): Promise<unknown> {
  return json({ items: (await accountsOf(conn, hookwars.MARKET_ID, hookwars.collectionCodec)).map((r) => ({ collection: r.address, ...r.data })) });
}
export async function commissions(conn: Connection): Promise<unknown> {
  return json({ items: (await accountsOf(conn, hookwars.MARKET_ID, hookwars.commissionCodec)).map((r) => ({ commission: r.address, ...r.data })) });
}
export async function commission(conn: Connection, key: string): Promise<unknown> {
  const c = new PublicKey(key);
  const info = await conn.getAccountInfo(c, 'confirmed');
  if (!info) return null;
  const subs = await accountsOf(conn, hookwars.MARKET_ID, hookwars.submissionCodec, [at(10, c)]);
  const vault = await conn.getAccountInfo(hookwars.commissionVaultAddress(c), 'confirmed');
  return json({ commission: key, ...hookwars.commissionCodec.decode(info.data), vaultLamports: vault ? String(vault.lamports) : null, submissions: subs.map((s) => ({ submission: s.address, ...s.data })) });
}

// ---------------------------------------------------------------- social --

export async function guilds(conn: Connection): Promise<unknown> {
  const rows = await accountsOf(conn, hookwars.SOCIAL_ID, hookwars.guildCodec);
  const treasuries = await conn.getMultipleAccountsInfo(rows.map((r) => hookwars.guildTreasuryAddress(r.data.id)), 'confirmed');
  return json({ items: rows.map((r, i) => ({ guild: r.address, ...r.data, treasuryLamports: treasuries[i] ? String(treasuries[i]!.lamports) : null })) });
}
export async function guild(conn: Connection, id: number): Promise<unknown> {
  const info = await conn.getAccountInfo(hookwars.guildAddress(id), 'confirmed');
  if (!info) return null;
  const idBytes = Buffer.alloc(4); idBytes.writeUInt32LE(id);
  const actions = await accountsOf(conn, hookwars.SOCIAL_ID, hookwars.guildActionCodec, [{ memcmp: { offset: 10, bytes: bs58.encode(idBytes) } }]);
  const t = await conn.getAccountInfo(hookwars.guildTreasuryAddress(id), 'confirmed');
  return json({ guild: hookwars.guildAddress(id).toBase58(), ...hookwars.guildCodec.decode(info.data), treasury: hookwars.guildTreasuryAddress(id).toBase58(), treasuryLamports: t ? String(t.lamports) : null, actions: actions.map((a) => ({ action: a.address, ...a.data })) });
}
export async function badges(conn: Connection, db: Pool | null): Promise<unknown> {
  const types = await accountsOf(conn, hookwars.SOCIAL_ID, hookwars.badgeTypeCodec);
  const recent = await safeRows<Record<string, unknown>>(db, 'select signature, slot, ts, id, recipient, claimant from ev_social_badge_awarded order by slot desc limit 100', []);
  return json({ items: types.map((t) => ({ badge: t.address, ...t.data })), recent });
}

// ---------------------------------------------------------------- economy (11 sections 5 and 6, 13) --

/** Craft: its config, every material and recipe, and the presets (chain state, bounded). */
export async function craftOverview(conn: Connection): Promise<unknown> {
  const cfg = await conn.getAccountInfo(hookwars.craftConfigAddress(), 'confirmed');
  const [materials, recipes, presets] = await Promise.all([
    accountsOf(conn, hookwars.CRAFT_ID, hookwars.materialCodec), accountsOf(conn, hookwars.CRAFT_ID, hookwars.recipeCodec), accountsOf(conn, hookwars.ARMORY_ID, hookwars.presetCodec),
  ]);
  return json({
    config: cfg ? hookwars.craftConfigCodec.decode(cfg.data) : null,
    materials: materials.map((r) => ({ address: r.address, ...r.data })), recipes: recipes.map((r) => ({ address: r.address, ...r.data })),
    presets: presets.map((r) => ({ address: r.address, ...r.data })),
  });
}

/** The order books: every market, or one material's book with its resting orders and class bids. */
export async function books(conn: Connection, materialId: number | null): Promise<unknown> {
  const cfg = await conn.getAccountInfo(hookwars.bookConfigAddress(), 'confirmed');
  const config = cfg ? hookwars.bookConfigCodec.decode(cfg.data) : null;
  if (materialId === null) {
    const [markets, bids] = await Promise.all([accountsOf(conn, hookwars.BOOK_ID, hookwars.bookMarketCodec), accountsOf(conn, hookwars.BOOK_ID, hookwars.classBidCodec)]);
    return json({ config, markets: markets.map((r) => ({ address: r.address, ...r.data })), classBids: bids.map((r) => ({ address: r.address, ...r.data })) });
  }
  const key = hookwars.bookMarketAddress(hookwars.materialMintAddress(materialId));
  const info = await conn.getAccountInfo(key, 'confirmed');
  return json({ config, market: info ? { address: key.toBase58(), ...hookwars.bookMarketCodec.decode(info.data) } : null });
}

/** An item's craft `Wear` (charges, dormancy, repairs), or null for an item that does not wear. */
export async function itemWear(conn: Connection, itemKey: string): Promise<unknown> {
  const info = await conn.getAccountInfo(hookwars.wearAddress(new PublicKey(itemKey)), 'confirmed');
  return info ? json(hookwars.wearCodec.decode(info.data)) : null;
}
