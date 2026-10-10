// Changed by Hookwars: new file (economy panel). The /economy page's reads.
/**
 * The economy panel's reads (docs/spec/11 section 7, "Protocol"): sums over the indexer's typed
 * event tables, plus the chain's own state where a figure lives only there (book depth, material
 * supply). Nothing is estimated or converted: a sum in bridged SOL only adds amounts the programs
 * paid in bridged SOL or in lamports, and amounts in a launched token's own units are counted per
 * token, never added to SOL. Every sum is a decimal string; a figure nothing recorded is null.
 */
import type { Connection } from '@solana/web3.js';
import type { Pool } from 'pg';
import { FIXED_ADDRESSES } from '@hookwars/shared';
import { books, craftOverview } from './reads-expansion.ts';

export const PERIODS = ['24h', '7d', '30d', 'season', 'all'] as const;
export type Period = (typeof PERIODS)[number];

/** `hookwars_common::economy::fee_source`, in order. */
export const FEE_SOURCES = ['dex', 'launchLp', 'itemRun', 'sale', 'licence', 'lease', 'bookFill', 'recipe'] as const;
const SOURCE_OF: Record<number, (typeof FEE_SOURCES)[number]> = Object.fromEntries(FEE_SOURCES.map((s, i) => [i, s]));

const BSOL = FIXED_ADDRESSES.bridgedSolMint;
/** `Pubkey::default()`: the mint the book, licences and recipes name for fees paid in native lamports. */
const NATIVE = '11111111111111111111111111111111';

export interface Window { period: Period; from: number; to: number; bucketSecs: number; seasonNumber: number | null }

type Row = Record<string, unknown>;
const rows = async (db: Pool, sql: string, params: unknown[] = []): Promise<Row[]> => (await db.query(sql, params)).rows;
/** A sum as a decimal string, null when no row was summed. */
const sum = (v: unknown): string | null => (v === null || v === undefined ? null : String(v));
const n = (v: unknown): number => Number(v ?? 0);

export function parsePeriod(v: string | null): Period {
  return (PERIODS as readonly string[]).includes(v ?? '') ? (v as Period) : '7d';
}

/** The window a period covers: from its start to now, in buckets the chart can draw (hours for a
 * day, days up to a quarter, weeks past it). `season` starts at the last opened season. */
export async function windowOf(db: Pool, period: Period, now = Math.floor(Date.now() / 1000)): Promise<Window> {
  const day = 86_400;
  let from = 0;
  let seasonNumber: number | null = null;
  if (period === '24h') from = now - day;
  else if (period === '7d') from = now - 7 * day;
  else if (period === '30d') from = now - 30 * day;
  else if (period === 'season') {
    const s = (await rows(db, 'select season, starts_at from ev_war_season_opened order by season desc limit 1'))[0];
    seasonNumber = s ? n(s.season) : null;
    from = s ? n(s.starts_at) : now;
  }
  // `all` sums from the start; its buckets follow the span since the first indexed event.
  let spanFrom = from;
  if (period === 'all') {
    const first = (await rows(db, 'select min(ts) as t from events'))[0];
    spanFrom = first?.t ? n(first.t) : now;
  }
  const span = Math.max(1, now - spanFrom);
  const bucketSecs = period === '24h' ? 3600 : span <= 120 * day ? day : 7 * day;
  return { period, from, to: now, bucketSecs, seasonNumber };
}

// ------------------------------------------------------------------------------------------ revenue --

export interface RevenueSource { source: (typeof FEE_SOURCES)[number]; lamports: string | null; events: number }
export interface Revenue {
  window: Window;
  /** Per source, in lamports of bridged SOL (or SOL, for market sales and licences). */
  sources: RevenueSource[];
  totalLamports: string | null;
  /** Buckets (unix seconds at their start), per source, in lamports. */
  series: { t: number; bySource: Partial<Record<(typeof FEE_SOURCES)[number], string>> }[];
  /** Protocol fees taken in a launched token's own units: counted per token, never summed to SOL. */
  otherMints: { mint: string; source: (typeof FEE_SOURCES)[number]; amount: string; events: number }[];
  /** Lifetime totals the program configs keep (book and craft), read from the chain. */
  configTotals: { book: string | null; craft: string | null };
}

/**
 * Protocol revenue by source. The DEX share is `Swapped.protocol_fee` (always in the pool's quote
 * token) on pools quoted in bridged SOL; licences, book fills and recipes are their programs'
 * `ProtocolFee` events in native lamports (the default mint); sales are `Sold.fee`, in lamports.
 * Item-run fees are taken on the token side in the token's own units, so they are listed per token
 * in `otherMints`, never summed into SOL. Lease fees and the launch LP source record no protocol
 * fee in the current programs and read as null.
 */
export async function revenue(db: Pool, conn: Connection | null, w: Window): Promise<Revenue> {
  const b = w.bucketSecs;
  const series = await rows(db, `
    with fees as (
      select s.ts, 0 as source, s.protocol_fee as amount from ev_swap_swapped s join ev_swap_pool_created p on p.pool = s.pool where p.quote_mint = $3 and s.protocol_fee > 0
      union all select ts, source::int, amount from ev_items_protocol_fee where mint in ($3, $5)
      union all select ts, source::int, amount from ev_craft_protocol_fee where mint in ($3, $5)
      union all select ts, source::int, amount from ev_book_protocol_fee where mint in ($3, $5)
      union all select ts, source::int, amount from ev_market_protocol_fee where mint in ($3, $5)
      union all select ts, 3, fee from ev_market_sold where fee > 0
    )
    select (ts / $4)::bigint * $4 as t, source, sum(amount) as lamports, count(*) as events
    from fees where ts >= $1 and ts <= $2 group by 1, 2 order by 1`, [w.from, w.to, BSOL, b, NATIVE]);
  const totals = new Map<string, { lamports: bigint; events: number }>();
  const buckets = new Map<number, Partial<Record<(typeof FEE_SOURCES)[number], string>>>();
  for (const r of series) {
    const src = SOURCE_OF[n(r.source)];
    if (!src) continue;
    const t = totals.get(src) ?? { lamports: 0n, events: 0 };
    t.lamports += BigInt(String(r.lamports)); t.events += n(r.events);
    totals.set(src, t);
    const at = n(r.t);
    const bk = buckets.get(at) ?? {};
    bk[src] = (BigInt(bk[src] ?? '0') + BigInt(String(r.lamports))).toString();
    buckets.set(at, bk);
  }
  const sources = FEE_SOURCES.map((source) => { const t = totals.get(source); return { source, lamports: t ? t.lamports.toString() : null, events: t?.events ?? 0 }; });
  const total = [...totals.values()].reduce((a, t) => a + t.lamports, 0n);
  const other = await rows(db, `
    select mint, source::int as source, sum(amount) as amount, count(*) as events from (
      select ts, mint, source, amount from ev_items_protocol_fee union all select ts, mint, source, amount from ev_craft_protocol_fee
      union all select ts, mint, source, amount from ev_book_protocol_fee union all select ts, mint, source, amount from ev_market_protocol_fee
    ) f where mint not in ($3, $4) and ts >= $1 and ts <= $2 group by 1, 2 order by 4 desc limit 20`, [w.from, w.to, BSOL, NATIVE]);
  let configTotals: Revenue['configTotals'] = { book: null, craft: null };
  if (conn) {
    try {
      const [bk, cr] = await Promise.all([books(conn, null) as Promise<{ config: { protocolFeesTotal?: string } | null }>, craftOverview(conn) as Promise<{ config: { protocolFeesTotal?: string } | null }>]);
      configTotals = { book: bk.config?.protocolFeesTotal ?? null, craft: cr.config?.protocolFeesTotal ?? null };
    } catch { /* the chain read failed: the totals stay null */ }
  }
  return {
    window: w, sources, totalLamports: totals.size ? total.toString() : null,
    series: [...buckets.entries()].sort((a, c) => a[0] - c[0]).map(([t, bySource]) => ({ t, bySource })),
    otherMints: other.map((r) => ({ mint: String(r.mint), source: SOURCE_OF[n(r.source)] ?? 'itemRun', amount: String(r.amount), events: n(r.events) })),
    configTotals,
  };
}

// ---------------------------------------------------------------------------------------- settlement --

export interface Settlement {
  window: Window;
  /** The pool side of every settle, in lamports of bridged SOL: where the quote cuts went. */
  quote: { holderRoyalty: string | null; author: string | null; rent: string | null; bounty: string | null; destinations: string | null; settles: number };
  /** The DEX's protocol share on the same pools, taken before a cut reaches an item's vault. */
  dexShare: string | null;
  /** The token side, in each token's own units: per token, never added across tokens. */
  tokenSide: { mint: string; settles: number; holderRoyalty: string; author: string; rent: string; bounty: string; destinations: string; burned: string; protocol: string }[];
}

export async function settlement(db: Pool, w: Window): Promise<Settlement> {
  const p = [w.from, w.to];
  const q = (await rows(db, `
    select count(*) as settles, sum(royalty_quote) as royalty, sum(bounty_quote) as bounty, sum(amount_quote) as dest
    from ev_items_equip_settled where ts >= $1 and ts <= $2`, p))[0] ?? {};
  const a = (await rows(db, 'select sum(author_quote) as author from ev_items_author_share_paid where ts >= $1 and ts <= $2', p))[0] ?? {};
  const r = (await rows(db, 'select sum(rent_quote) as rent from ev_items_lease_rent_paid where ts >= $1 and ts <= $2', p))[0] ?? {};
  const dex = (await rows(db, `select sum(s.protocol_fee) as v from ev_swap_swapped s join ev_swap_pool_created p on p.pool = s.pool where p.quote_mint = $3 and s.ts >= $1 and s.ts <= $2`, [...p, BSOL]))[0] ?? {};
  const tok = await rows(db, `
    with s as (select mint, count(*) as settles, sum(royalty_token) as royalty, sum(bounty_token) as bounty, sum(amount_token) as dest, sum(burned) as burned from ev_items_equip_settled where ts >= $1 and ts <= $2 group by mint),
    a as (select mint, sum(author_token) as author from ev_items_author_share_paid where ts >= $1 and ts <= $2 group by mint),
    r as (select mint, sum(rent_token) as rent from ev_items_lease_rent_paid where ts >= $1 and ts <= $2 group by mint),
    f as (select mint, sum(amount) as protocol from ev_items_protocol_fee where source = 2 and ts >= $1 and ts <= $2 group by mint)
    select s.*, coalesce(a.author, 0) as author, coalesce(r.rent, 0) as rent, coalesce(f.protocol, 0) as protocol
    from s left join a using (mint) left join r using (mint) left join f using (mint) order by s.settles desc limit 12`, p);
  const settles = n(q.settles);
  return {
    window: w,
    quote: { holderRoyalty: settles ? sum(q.royalty) : null, author: sum(a.author), rent: sum(r.rent), bounty: settles ? sum(q.bounty) : null, destinations: settles ? sum(q.dest) : null, settles },
    dexShare: sum(dex.v),
    tokenSide: tok.map((x) => ({ mint: String(x.mint), settles: n(x.settles), holderRoyalty: String(x.royalty), author: String(x.author), rent: String(x.rent), bounty: String(x.bounty), destinations: String(x.dest), burned: String(x.burned), protocol: String(x.protocol) })),
  };
}

// ------------------------------------------------------------------------------------------ builders --

export interface Builders {
  window: Window;
  /** Templates by the author share their items paid (quote side, lamports). */
  templates: { templateId: number; name: string | null; authorLamports: string; payments: number }[];
  /** Royalty claims in bridged SOL and licence income to holders, by wallet. */
  holders: { wallet: string; royaltyLamports: string; licenceLamports: string }[];
  /** Items by what their settles moved on the quote side (royalty, author, rent, bounty, destinations). */
  items: { item: string; itemMint: string | null; templateId: number | null; name: string | null; owner: string | null; settledLamports: string; settles: number }[];
}

export async function builders(db: Pool, w: Window): Promise<Builders> {
  const p = [w.from, w.to];
  const t = await rows(db, `
    select a.template_id, t.name, sum(a.author_quote) as v, count(*) as c from ev_items_author_share_paid a
    left join templates t on t.template_id = a.template_id where a.ts >= $1 and a.ts <= $2 and a.author_quote > 0 group by 1, 2 order by 3 desc limit 10`, p);
  const h = await rows(db, `
    with r as (select claimant as wallet, sum(amount) as v from ev_armory_royalty_claimed where cut_mint = $3 and ts >= $1 and ts <= $2 group by 1),
    l as (select holder as wallet, sum(to_holder) as v from ev_market_licence_bought where ts >= $1 and ts <= $2 group by 1)
    select coalesce(r.wallet, l.wallet) as wallet, coalesce(r.v, 0) as royalty, coalesce(l.v, 0) as licence
    from r full join l on l.wallet = r.wallet order by coalesce(r.v, 0) + coalesce(l.v, 0) desc limit 10`, [...p, BSOL]);
  const i = await rows(db, `
    with s as (select item, count(*) as c, sum(royalty_quote + bounty_quote + amount_quote) as v from ev_items_equip_settled where ts >= $1 and ts <= $2 group by 1),
    a as (select item, sum(author_quote) as v from ev_items_author_share_paid where ts >= $1 and ts <= $2 group by 1),
    r as (select item, sum(rent_quote) as v from ev_items_lease_rent_paid where ts >= $1 and ts <= $2 group by 1)
    select s.item, it.item_mint, it.template_id, t.name, o.owner, s.c, s.v + coalesce(a.v, 0) + coalesce(r.v, 0) as total
    from s left join a using (item) left join r using (item) left join items it on it.item = s.item
    left join templates t on t.template_id = it.template_id left join item_owners o on o.item_mint = it.item_mint
    where s.v + coalesce(a.v, 0) + coalesce(r.v, 0) > 0 order by total desc limit 10`, p);
  return {
    window: w,
    templates: t.map((x) => ({ templateId: n(x.template_id), name: (x.name as string) ?? null, authorLamports: String(x.v), payments: n(x.c) })),
    holders: h.map((x) => ({ wallet: String(x.wallet), royaltyLamports: String(x.royalty), licenceLamports: String(x.licence) })),
    items: i.map((x) => ({ item: String(x.item), itemMint: (x.item_mint as string) ?? null, templateId: x.template_id === null ? null : n(x.template_id), name: (x.name as string) ?? null, owner: (x.owner as string) ?? null, settledLamports: String(x.total), settles: n(x.c) })),
  };
}

// -------------------------------------------------------------------------------------------- market --

export interface Market {
  window: Window;
  sales: { count: number; volumeLamports: string | null; feeLamports: string | null; resaleLamports: string | null };
  /** Per template: active listings, the cheapest of them, and the last sale. */
  classes: { templateId: number; name: string | null; listings: number; floorLamports: string | null; lastSaleLamports: string | null; lastSaleTs: number | null; sales: number }[];
  activeListings: number;
  licences: { live: number; bought: number; incomeLamports: string | null; protocolLamports: string | null; authorLamports: string | null; holderLamports: string | null };
  leases: { active: number; started: number; feesLamports: string | null; rentLamports: string | null };
  commissions: { open: number; openBountyLamports: string | null; paid: number; paidLamports: string | null; refunded: number };
}

export async function market(db: Pool, w: Window): Promise<Market> {
  const p = [w.from, w.to];
  const s = (await rows(db, 'select count(*) as c, sum(price) as v, sum(fee) as f, sum(resale) as r from ev_market_sold where ts >= $1 and ts <= $2', p))[0] ?? {};
  // A listing is active while no sale, delisting or expiry of its item mint came after it.
  const active = `
    with last_listed as (select distinct on (item_mint) item_mint, item, price_lamports, expires_at, slot, ordinal from ev_market_listed order by item_mint, slot desc, ordinal desc),
    closed as (select item_mint, max(slot) as slot from (select item_mint, slot from ev_market_sold union all select item_mint, slot from ev_market_delisted union all select item_mint, slot from ev_market_listing_expired) x group by 1)
    select l.* from last_listed l left join closed c on c.item_mint = l.item_mint
    where (c.slot is null or c.slot < l.slot) and (l.expires_at = 0 or l.expires_at > $1)`;
  const cls = await rows(db, `
    with a as (${active}),
    al as (select it.template_id, count(*) as listings, min(a.price_lamports) as floor from a join items it on it.item = a.item group by 1),
    sl as (select distinct on (it.template_id) it.template_id, s.price, s.ts from ev_market_sold s join items it on it.item = s.item order by it.template_id, s.slot desc, s.ordinal desc),
    sc as (select it.template_id, count(*) as c from ev_market_sold s join items it on it.item = s.item where s.ts >= $2 and s.ts <= $3 group by 1)
    select coalesce(al.template_id, sl.template_id) as template_id, t.name, coalesce(al.listings, 0) as listings, al.floor, sl.price as last_price, sl.ts as last_ts, coalesce(sc.c, 0) as sales
    from al full join sl on sl.template_id = al.template_id left join sc on sc.template_id = coalesce(al.template_id, sl.template_id)
    left join templates t on t.template_id = coalesce(al.template_id, sl.template_id) order by listings desc, sales desc limit 20`, [w.to, ...p]);
  const activeCount = (await rows(db, `select count(*) as c from (${active}) a`, [w.to]))[0] ?? {};
  const lic = (await rows(db, 'select count(*) as c, sum(price) as v, sum(protocol) as p, sum(author) as a, sum(to_holder) as h from ev_market_licence_bought where ts >= $1 and ts <= $2', p))[0] ?? {};
  // Live: the latest purchase of a (item, token) pair runs past now and nothing revoked or expired it since.
  const live = (await rows(db, `
    with l as (select distinct on (item, token_mint) item, token_mint, ends_at, slot from ev_market_licence_bought order by item, token_mint, slot desc, ordinal desc),
    e as (select item, token_mint, max(slot) as slot from (select item, token_mint, slot from ev_market_licence_revoked union all select item, token_mint, slot from ev_market_licence_expired) x group by 1, 2)
    select count(*) as c from l left join e using (item, token_mint) where l.ends_at > $1 and (e.slot is null or e.slot < l.slot)`, [w.to]))[0] ?? {};
  const leases = (await rows(db, `
    with st as (select distinct on (item) item, ends_at, slot from ev_market_lease_started order by item, slot desc, ordinal desc),
    en as (select item, max(slot) as slot from ev_market_lease_ended group by 1)
    select count(*) as c from st left join en using (item) where st.ends_at > $1 and (en.slot is null or en.slot < st.slot)`, [w.to]))[0] ?? {};
  // A lease's fee is the one its last offer named before it started.
  const started = (await rows(db, `
    select count(*) as c, sum(o.fee_lamports) as fees from ev_market_lease_started s
    left join lateral (select fee_lamports from ev_market_lease_offered o where o.item = s.item and o.slot <= s.slot order by o.slot desc, o.ordinal desc limit 1) o on true
    where s.ts >= $1 and s.ts <= $2`, p))[0] ?? {};
  const rent = (await rows(db, 'select sum(rent_quote) as v from ev_items_lease_rent_paid where ts >= $1 and ts <= $2', p))[0] ?? {};
  const com = (await rows(db, `
    with o as (select commission, bounty_lamports from ev_market_commission_opened),
    d as (select commission from ev_market_commission_paid union select commission from ev_market_commission_refunded)
    select count(*) as c, sum(bounty_lamports) as v from o where commission not in (select commission from d)`))[0] ?? {};
  const paid = (await rows(db, 'select count(*) as c, sum(bounty_lamports) as v from ev_market_commission_paid where ts >= $1 and ts <= $2', p))[0] ?? {};
  const refunded = (await rows(db, 'select count(*) as c from ev_market_commission_refunded where ts >= $1 and ts <= $2', p))[0] ?? {};
  const sales = n(s.c);
  return {
    window: w,
    sales: { count: sales, volumeLamports: sales ? sum(s.v) : null, feeLamports: sales ? sum(s.f) : null, resaleLamports: sales ? sum(s.r) : null },
    classes: cls.map((x) => ({ templateId: n(x.template_id), name: (x.name as string) ?? null, listings: n(x.listings), floorLamports: sum(x.floor), lastSaleLamports: sum(x.last_price), lastSaleTs: x.last_ts === null ? null : n(x.last_ts), sales: n(x.sales) })),
    activeListings: n(activeCount.c),
    licences: { live: n(live.c), bought: n(lic.c), incomeLamports: sum(lic.v), protocolLamports: sum(lic.p), authorLamports: sum(lic.a), holderLamports: sum(lic.h) },
    leases: { active: n(leases.c), started: n(started.c), feesLamports: sum(started.fees), rentLamports: sum(rent.v) },
    commissions: { open: n(com.c), openBountyLamports: sum(com.v), paid: n(paid.c), paidLamports: sum(paid.v), refunded: n(refunded.c) },
  };
}

// --------------------------------------------------------------------------------------------- craft --

type Mat = { id: number; name: string; mint: string; emissionCapPerSeason: string; emittedThisSeason: string; emittedTotal: string; burnedTotal: string };
type Rec = { id: number; uses: string; terms: { kind: number; inputs: { materialId: number; amount: string }[]; feeLamports: string } };
type Ord = { price: string; size: string };
type Mkt = { address: string; materialId: number; baseMint: string; bids: Ord[]; asks: Ord[]; fills: string };

export interface Craft {
  window: Window;
  materials: { id: number; name: string; mint: string; cap: string; emittedThisSeason: string; emittedTotal: string; burnedTotal: string; dropped: string | null; burnedByRecipes: string | null; net: string | null }[];
  dropsBySource: { source: number; materialId: number; amount: string; drops: number }[];
  recipes: { crafts: number; repairs: number; feesLamports: string | null };
  books: { materialId: number; name: string | null; market: string; bestBid: string | null; bestAsk: string | null; spread: string | null; bidDepth: string; askDepth: string; bidOrders: number; askOrders: number; fills: number; volumeLamports: string | null; lastPrice: string | null; lastTs: number | null }[];
  classFills: { count: number; volumeLamports: string | null };
  wear: { tracked: number; dormant: number; repairs: number };
  /** Whether the craft program (materials, recipes) and the book program could be read. */
  chain: boolean;
  bookChain: boolean;
}

/** Drops come from `Dropped`, burns from the recipes used in the window times each recipe's inputs
 * (the chain keeps a recipe's terms; a recipe changed inside the window counts at its current
 * terms, so the per-window burn is labelled as such). Supply and caps are the chain's. */
export async function craft(db: Pool, conn: Connection | null, w: Window): Promise<Craft> {
  const p = [w.from, w.to];
  let mats: Mat[] = []; let recs: Rec[] = []; let mkts: Mkt[] = []; let chain = false; let bookChain = false;
  if (conn) {
    // Each program read on its own: a book that cannot be read leaves the materials standing.
    const [c, b] = await Promise.allSettled([craftOverview(conn) as Promise<{ materials: Mat[]; recipes: Rec[] }>, books(conn, null) as Promise<{ markets: Mkt[] }>]);
    if (c.status === 'fulfilled') { mats = c.value.materials; recs = c.value.recipes; chain = true; }
    if (b.status === 'fulfilled') { mkts = b.value.markets; bookChain = true; }
  }
  const drops = await rows(db, 'select source::int as source, material_id::int as material_id, sum(amount) as amount, count(*) as c from ev_craft_dropped where ts >= $1 and ts <= $2 group by 1, 2 order by 3 desc', p);
  const used = await rows(db, `
    select recipe, count(*) as c, sum(fee) as fee, 'craft' as kind from ev_craft_crafted where ts >= $1 and ts <= $2 group by recipe
    union all select recipe, count(*), sum(fee), 'repair' from ev_craft_repaired where ts >= $1 and ts <= $2 group by recipe`, p);
  const burnBy = new Map<number, bigint>();
  let crafts = 0, repairs = 0, fees = 0n, anyFee = false;
  for (const u of used) {
    const count = n(u.c);
    if (u.kind === 'craft') crafts += count; else repairs += count;
    if (u.fee !== null) { fees += BigInt(String(u.fee)); anyFee = true; }
    const rec = recs.find((r) => String(r.id) === String(u.recipe));
    for (const i of rec?.terms.inputs ?? []) burnBy.set(i.materialId, (burnBy.get(i.materialId) ?? 0n) + BigInt(i.amount) * BigInt(count));
  }
  const dropBy = new Map<number, bigint>();
  for (const d of drops) dropBy.set(n(d.material_id), (dropBy.get(n(d.material_id)) ?? 0n) + BigInt(String(d.amount)));
  const fills = await rows(db, `
    select f.market, count(*) as c, sum(f.price * f.size) as v from ev_book_filled f where f.ts >= $1 and f.ts <= $2 group by 1`, p);
  const last = await rows(db, 'select distinct on (market) market, price, ts from ev_book_filled order by market, slot desc, ordinal desc');
  const cf = (await rows(db, 'select count(*) as c, sum(price) as v from ev_book_class_filled where ts >= $1 and ts <= $2', p))[0] ?? {};
  const wr = (await rows(db, 'select count(*) as c, count(*) filter (where dormant) as d, coalesce(sum(repairs), 0) as r from wear'))[0] ?? {};
  const best = (orders: Ord[], hi: boolean): bigint | null => orders.length ? orders.map((o) => BigInt(o.price)).reduce((a, b) => (hi ? (b > a ? b : a) : (b < a ? b : a))) : null;
  const depth = (orders: Ord[]): string => orders.reduce((a, o) => a + BigInt(o.price) * BigInt(o.size), 0n).toString();
  return {
    window: w,
    materials: mats.map((m) => {
      const dropped = dropBy.get(m.id) ?? null; const burned = burnBy.get(m.id) ?? null;
      return { id: m.id, name: m.name, mint: m.mint, cap: m.emissionCapPerSeason, emittedThisSeason: m.emittedThisSeason, emittedTotal: m.emittedTotal, burnedTotal: m.burnedTotal,
        dropped: dropped === null ? null : dropped.toString(), burnedByRecipes: burned === null ? null : burned.toString(),
        net: dropped === null && burned === null ? null : ((dropped ?? 0n) - (burned ?? 0n)).toString() };
    }),
    dropsBySource: drops.map((d) => ({ source: n(d.source), materialId: n(d.material_id), amount: String(d.amount), drops: n(d.c) })),
    recipes: { crafts, repairs, feesLamports: anyFee ? fees.toString() : null },
    books: mkts.map((m) => {
      const bid = best(m.bids, true), ask = best(m.asks, false);
      const f = fills.find((x) => x.market === m.address); const l = last.find((x) => x.market === m.address);
      return { materialId: m.materialId, name: mats.find((x) => x.id === m.materialId)?.name ?? null, market: m.address,
        bestBid: bid === null ? null : bid.toString(), bestAsk: ask === null ? null : ask.toString(), spread: bid !== null && ask !== null ? (ask - bid).toString() : null,
        bidDepth: depth(m.bids), askDepth: depth(m.asks), bidOrders: m.bids.length, askOrders: m.asks.length,
        fills: n(f?.c), volumeLamports: f ? String(f.v) : null, lastPrice: l ? String(l.price) : null, lastTs: l ? n(l.ts) : null };
    }),
    classFills: { count: n(cf.c), volumeLamports: n(cf.c) ? sum(cf.v) : null },
    wear: { tracked: n(wr.c), dormant: n(wr.d), repairs: n(wr.r) },
    chain, bookChain,
  };
}

// -------------------------------------------------------------------------------------------- cranks --

/** Crank types whose events carry the bounty paid, in lamports. */
const CRANKS: { kind: string; sql: string }[] = [
  { kind: 'settle', sql: 'select ts, null::text as cranker, bounty_quote as bounty from ev_items_equip_settled' },
  { kind: 'siege', sql: 'select ts, cranker, bounty from ev_war_siege_executed' },
  { kind: 'counterStrike', sql: 'select ts, cranker, bounty from ev_war_counter_strike_executed' },
  { kind: 'raze', sql: 'select ts, cranker, bounty from ev_war_razed' },
  { kind: 'coalitionSiege', sql: 'select ts, cranker, bounty from ev_war_coalition_siege_executed' },
  { kind: 'coalitionRaze', sql: 'select ts, null::text as cranker, bounty from ev_war_coalition_razed' },
  { kind: 'treatyShare', sql: 'select ts, cranker, bounty from ev_war_treaty_inflow_shared' },
  { kind: 'prize', sql: 'select ts, cranker, bounty from ev_war_prize_paid' },
  { kind: 'bookExpiry', sql: 'select ts, cranker, bounty from ev_book_expired' },
  { kind: 'buyback', sql: 'select ts, cranker, bounty from ev_companion_bought_back' },
  { kind: 'feeClaim', sql: 'select ts, cranker, bounty from ev_companion_fees_claimed' },
  { kind: 'holderShare', sql: 'select ts, cranker, bounty from ev_companion_shared_with_holders' },
];

export interface Cranks {
  window: Window;
  byKind: { kind: string; runs: number; bountyLamports: string | null }[];
  /** Crankers named by their events (settle and coalition razes name none). */
  top: { cranker: string; runs: number; bountyLamports: string }[];
  /** `AgentCredited` by passport and record kind, as each program records the value. */
  agents: { passport: string; name: string | null; byKind: Record<string, string>; records: number }[];
}

export const AGENT_KINDS = ['itemsAuthored', 'itemsEquipped', 'itemsForged', 'royaltyClaim', 'crank', 'bounty', 'lootReveal'];

export async function cranks(db: Pool, w: Window): Promise<Cranks> {
  const p = [w.from, w.to];
  const all = CRANKS.map((c, i) => `select '${c.kind}' as kind, ${i} as k, ts, cranker, bounty from (${c.sql}) x${i}`).join(' union all ');
  const by = await rows(db, `select kind, min(k) as k, count(*) as c, sum(bounty) as v from (${all}) a where ts >= $1 and ts <= $2 group by kind order by 2`, p);
  const top = await rows(db, `select cranker, count(*) as c, sum(bounty) as v from (${all}) a where cranker is not null and ts >= $1 and ts <= $2 group by 1 order by 3 desc limit 10`, p);
  const ag = await rows(db, `
    select c.passport, r.name, c.kind::int as kind, sum(c.value) as v, count(*) as records from ev_agents_agent_credited c
    left join lateral (select name from ev_agents_passport_registered r where r.passport = c.passport order by slot desc limit 1) r on true
    where c.ts >= $1 and c.ts <= $2 group by 1, 2, 3`, p);
  const agents = new Map<string, Cranks['agents'][number]>();
  for (const r of ag) {
    const key = String(r.passport);
    const a = agents.get(key) ?? { passport: key, name: (r.name as string) ?? null, byKind: {}, records: 0 };
    a.byKind[AGENT_KINDS[n(r.kind)] ?? `kind${n(r.kind)}`] = String(r.v);
    a.records += n(r.records);
    agents.set(key, a);
  }
  const earned = (a: Cranks['agents'][number]) => BigInt(a.byKind.royaltyClaim ?? '0') + BigInt(a.byKind.bounty ?? '0');
  return {
    window: w,
    byKind: CRANKS.map((c) => { const r = by.find((x) => x.kind === c.kind); return { kind: c.kind, runs: n(r?.c), bountyLamports: r ? sum(r.v) : null }; }),
    top: top.map((r) => ({ cranker: String(r.cranker), runs: n(r.c), bountyLamports: String(r.v) })),
    agents: [...agents.values()].sort((a, b) => (earned(b) > earned(a) ? 1 : earned(b) < earned(a) ? -1 : b.records - a.records)).slice(0, 12),
  };
}

// ----------------------------------------------------------------------------------------------- war --

export interface War {
  window: Window;
  inflows: { funded: string | null; fundings: number; fromCompanions: string | null; treatyShared: string | null; coalitionContributed: string | null };
  outflows: { siegeSpent: string | null; sieges: number; counterStrikeSpent: string | null; razeProceeds: string | null; razes: number; coalitionSiegeSpent: string | null; bountiesPaid: string | null };
  boss: { funded: string | null; claimed: string | null; sealed: number };
  prize: { toWinner: string | null; toTreasury: string | null; paid: number };
  /** War chests by what they took in, with what they spent on sieges. */
  chests: { mint: string; fundedLamports: string; siegeSpentLamports: string; razeLamports: string }[];
  series: { t: number; funded: string; spent: string }[];
}

export async function war(db: Pool, w: Window): Promise<War> {
  const p = [w.from, w.to];
  const one = async (sql: string) => (await rows(db, sql, p))[0] ?? {};
  const f = await one('select count(*) as c, sum(amount) as v from ev_war_war_funded where ts >= $1 and ts <= $2');
  const cmp = await one('select sum(amount) as v from ev_companion_companion_war_funded where ts >= $1 and ts <= $2');
  const tr = await one('select sum(amount) as v from ev_war_treaty_inflow_shared where ts >= $1 and ts <= $2');
  const co = await one('select sum(amount) as v from ev_war_coalition_contributed where ts >= $1 and ts <= $2');
  const sg = await one('select count(*) as c, sum(spent) as v from ev_war_siege_executed where ts >= $1 and ts <= $2');
  const cs = await one('select sum(spent) as v from ev_war_counter_strike_executed where ts >= $1 and ts <= $2');
  const rz = await one('select count(*) as c, sum(got) as v from ev_war_razed where ts >= $1 and ts <= $2');
  const csg = await one('select sum(spent) as v from ev_war_coalition_siege_executed where ts >= $1 and ts <= $2');
  const bt = await one('select sum(paid) as v from ev_war_bounty_claimed where ts >= $1 and ts <= $2');
  const bf = await one('select sum(amount) as v from ev_war_boss_pool_funded where ts >= $1 and ts <= $2');
  const bc = await one('select sum(amount) as v from ev_war_boss_share_claimed where ts >= $1 and ts <= $2');
  const bs = await one('select count(*) as c from ev_war_boss_pool_sealed where ts >= $1 and ts <= $2');
  const pz = await one('select count(*) as c, sum(to_winner) as w, sum(to_treasury) as t from ev_war_prize_paid where ts >= $1 and ts <= $2');
  const chests = await rows(db, `
    with f as (select mint, sum(amount) as v from ev_war_war_funded where ts >= $1 and ts <= $2 group by 1),
    s as (select mint, sum(spent) as v from ev_war_siege_executed where ts >= $1 and ts <= $2 group by 1),
    r as (select mint, sum(got) as v from ev_war_razed where ts >= $1 and ts <= $2 group by 1)
    select coalesce(f.mint, s.mint, r.mint) as mint, coalesce(f.v, 0) as funded, coalesce(s.v, 0) as spent, coalesce(r.v, 0) as raze
    from f full join s on s.mint = f.mint full join r on r.mint = coalesce(f.mint, s.mint) order by 2 desc, 3 desc limit 10`, p);
  const series = await rows(db, `
    select t, sum(funded) as funded, sum(spent) as spent from (
      select (ts / $3)::bigint * $3 as t, amount as funded, 0 as spent from ev_war_war_funded where ts >= $1 and ts <= $2
      union all select (ts / $3)::bigint * $3, 0, spent from ev_war_siege_executed where ts >= $1 and ts <= $2
    ) x group by 1 order by 1`, [...p, w.bucketSecs]);
  return {
    window: w,
    inflows: { funded: sum(f.v), fundings: n(f.c), fromCompanions: sum(cmp.v), treatyShared: sum(tr.v), coalitionContributed: sum(co.v) },
    outflows: { siegeSpent: sum(sg.v), sieges: n(sg.c), counterStrikeSpent: sum(cs.v), razeProceeds: sum(rz.v), razes: n(rz.c), coalitionSiegeSpent: sum(csg.v), bountiesPaid: sum(bt.v) },
    boss: { funded: sum(bf.v), claimed: sum(bc.v), sealed: n(bs.c) },
    prize: { toWinner: sum(pz.w), toTreasury: sum(pz.t), paid: n(pz.c) },
    chests: chests.map((c) => ({ mint: String(c.mint), fundedLamports: String(c.funded), siegeSpentLamports: String(c.spent), razeLamports: String(c.raze) })),
    series: series.map((s) => ({ t: n(s.t), funded: String(s.funded), spent: String(s.spent) })),
  };
}

export const SECTIONS = { revenue, settlement, builders, market, craft, cranks, war } as const;
export type Section = keyof typeof SECTIONS;

/** One section for a period; the window is computed once per call. */
export async function economy(db: Pool, conn: Connection | null, section: Section, period: Period, now?: number): Promise<unknown> {
  const w = await windowOf(db, period, now);
  switch (section) {
    case 'revenue': return revenue(db, conn, w);
    case 'craft': return craft(db, conn, w);
    case 'settlement': return settlement(db, w);
    case 'builders': return builders(db, w);
    case 'market': return market(db, w);
    case 'cranks': return cranks(db, w);
    case 'war': return war(db, w);
  }
}
