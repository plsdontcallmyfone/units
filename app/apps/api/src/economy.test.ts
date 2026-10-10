// Changed by Hookwars: new file (economy panel).
/**
 * The economy panel's sums against a real Postgres (DATABASE_URL; skipped without one), on fixture
 * event rows written straight into the indexer's typed tables. Each case checks a figure is the sum
 * of exactly the rows it names: bridged SOL only, the window respected, token-side amounts kept per
 * token and never added to SOL, and nothing recorded reading as null.
 */
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import pg from 'pg';
import { allDdl } from '@hookwars/indexer/schema.ts';
import { FIXED_ADDRESSES } from '@hookwars/shared';
import { builders, cranks, craft, economy, market, parsePeriod, revenue, settlement, war, windowOf, type Window } from './economy.ts';

const url = process.env.DATABASE_URL ?? '';
const withDb = url ? describe : describe.skip;
const schema = `econtest_${process.pid}`;
let db: pg.Pool;

const BSOL = FIXED_ADDRESSES.bridgedSolMint;
const NOW = 2_000_000_000;
const DAY = 86_400;
let seq = 0;
async function ins(table: string, row: Record<string, unknown>, ts = NOW - 3600): Promise<void> {
  seq += 1;
  const cols = ['signature', 'ordinal', 'slot', 'ts', ...Object.keys(row)];
  const vals = [`sig${seq}`, 0, seq, ts, ...Object.values(row)];
  await db.query(`insert into ${table} (${cols.map((c) => `"${c}"`).join(',')}) values (${vals.map((_, i) => `$${i + 1}`).join(',')})`, vals);
}
const W = (from: number, bucketSecs = DAY): Window => ({ period: '7d', from, to: NOW, bucketSecs, seasonNumber: null });

beforeAll(async () => {
  if (!url) return;
  const admin = new pg.Pool({ connectionString: url, max: 1 });
  await admin.query(`create schema if not exists ${schema}`); await admin.end();
  db = new pg.Pool({ connectionString: url, max: 2, options: `-c search_path=${schema}` });
  for (const s of allDdl()) await db.query(s);
});
afterAll(async () => {
  if (!url) return;
  await db.query(`drop schema if exists ${schema} cascade`);
  await db.end();
});

describe('periods', () => {
  it('an unknown period falls back to 7d', () => {
    expect(parsePeriod('bogus')).toBe('7d');
    expect(parsePeriod('season')).toBe('season');
    expect(parsePeriod(null)).toBe('7d');
  });
});

withDb('economy reads', () => {
  it('an empty database reads as nulls and empty lists, never zeros made up', async () => {
    const r = await revenue(db, null, W(NOW - 7 * DAY));
    expect(r.totalLamports).toBeNull();
    expect(r.sources.every((s) => s.lamports === null && s.events === 0)).toBe(true);
    const s = await settlement(db, W(NOW - 7 * DAY));
    expect(s.quote).toMatchObject({ holderRoyalty: null, author: null, rent: null, bounty: null, destinations: null, settles: 0 });
    const m = await market(db, W(NOW - 7 * DAY));
    expect(m.sales).toEqual({ count: 0, volumeLamports: null, feeLamports: null, resaleLamports: null });
    const w = await war(db, W(NOW - 7 * DAY));
    expect(w.inflows.funded).toBeNull();
    const season = await windowOf(db, 'season', NOW);
    expect(season.seasonNumber).toBeNull();
  });

  it('revenue: bridged SOL by source, the DEX share on SOL-quoted pools only, the window respected', async () => {
    await ins('ev_swap_pool_created', { pool: 'PoolSol', quote_mint: BSOL, base_mint: 'TokA' }, NOW - 30 * DAY);
    await ins('ev_swap_pool_created', { pool: 'PoolUsd', quote_mint: 'UsdMint', base_mint: 'TokB' }, NOW - 30 * DAY);
    await ins('ev_swap_swapped', { pool: 'PoolSol', protocol_fee: 100 });
    await ins('ev_swap_swapped', { pool: 'PoolSol', protocol_fee: 50 }, NOW - 2 * DAY);
    await ins('ev_swap_swapped', { pool: 'PoolUsd', protocol_fee: 999 });
    await ins('ev_swap_swapped', { pool: 'PoolSol', protocol_fee: 7 }, NOW - 9 * DAY);
    await ins('ev_items_protocol_fee', { source: 2, mint: BSOL, amount: 30 });
    await ins('ev_items_protocol_fee', { source: 2, mint: 'TokA', amount: 5000 });
    await ins('ev_market_protocol_fee', { source: 4, mint: BSOL, amount: 11 });
    await ins('ev_book_protocol_fee', { source: 6, mint: BSOL, amount: 4 });
    await ins('ev_craft_protocol_fee', { source: 7, mint: BSOL, amount: 3 });
    await ins('ev_market_sold', { item: 'ItemS', item_mint: 'ImS', seller: 'S', buyer: 'B', price: 1000, fee: 20, resale: 50 });
    const r = await revenue(db, null, W(NOW - 7 * DAY));
    const by = Object.fromEntries(r.sources.map((s) => [s.source, s.lamports]));
    expect(by).toMatchObject({ dex: '150', itemRun: '30', licence: '11', bookFill: '4', recipe: '3', sale: '20', lease: null, launchLp: null });
    expect(r.totalLamports).toBe('218');
    expect(r.otherMints).toEqual([{ mint: 'TokA', source: 'itemRun', amount: '5000', events: 1 }]);
    const t = r.series.reduce((a, b) => a + Object.values(b.bySource).reduce((x, y) => x + Number(y), 0), 0);
    expect(t).toBe(218);
    expect(r.series.length).toBe(2);
  });

  it('settlement: the quote waterfall sums author, rent, holder, bounty and destinations; token sides stay per token', async () => {
    await ins('ev_items_equip_settled', { mint: 'TokA', slot: 1, item: 'Item1', royalty_token: 10, royalty_quote: 60, amount_token: 80, amount_quote: 300, burned: 5, bounty_token: 2, bounty_quote: 9 });
    await ins('ev_items_equip_settled', { mint: 'TokB', slot: 0, item: 'Item2', royalty_token: 1, royalty_quote: 40, amount_token: 0, amount_quote: 100, burned: 0, bounty_token: 0, bounty_quote: 1 });
    await ins('ev_items_author_share_paid', { mint: 'TokA', slot: 1, item: 'Item1', template_id: 3, author_token: 1, author_quote: 6 });
    await ins('ev_items_lease_rent_paid', { mint: 'TokA', slot: 1, item: 'Item1', lessor: 'Lessor', rent_token: 3, rent_quote: 12 });
    const s = await settlement(db, W(NOW - 7 * DAY));
    expect(s.quote).toEqual({ holderRoyalty: '100', author: '6', rent: '12', bounty: '10', destinations: '400', settles: 2 });
    expect(s.dexShare).toBe('150');
    const a = s.tokenSide.find((x) => x.mint === 'TokA')!;
    expect(a).toMatchObject({ settles: 1, holderRoyalty: '10', author: '1', rent: '3', bounty: '2', destinations: '80', burned: '5', protocol: '5000' });
  });

  it('builders: templates by author share, holders by royalties and licence income, items by what they settled', async () => {
    await db.query(`insert into templates (template_id, name) values (3, 'Raid')`);
    await db.query(`insert into items (item, item_mint, template_id) values ('Item1', 'Im1', 3), ('Item2', 'Im2', 3), ('ItemS', 'ImS', 3)`);
    await db.query(`insert into item_owners (item_mint, owner) values ('Im1', 'Owner1')`);
    await ins('ev_armory_royalty_claimed', { item: 'Item1', cut_mint: BSOL, claimant: 'Owner1', amount: 70 });
    await ins('ev_armory_royalty_claimed', { item: 'Item1', cut_mint: 'TokA', claimant: 'Owner1', amount: 9999 });
    await ins('ev_market_licence_bought', { item: 'Item2', token_mint: 'TokC', payer: 'P', holder: 'Owner2', price: 100, protocol: 11, author: 9, to_holder: 80, ends_at: NOW + DAY, renewal: false });
    const b = await builders(db, W(NOW - 7 * DAY));
    expect(b.templates).toEqual([{ templateId: 3, name: 'Raid', authorLamports: '6', payments: 1 }]);
    expect(b.holders).toEqual([{ wallet: 'Owner2', royaltyLamports: '0', licenceLamports: '80' }, { wallet: 'Owner1', royaltyLamports: '70', licenceLamports: '0' }]);
    expect(b.items[0]).toMatchObject({ item: 'Item1', itemMint: 'Im1', name: 'Raid', owner: 'Owner1', settledLamports: String(60 + 9 + 300 + 6 + 12), settles: 1 });
  });

  it('market: sales, live licences, active leases with their fees, open commissions, floors from active listings only', async () => {
    await ins('ev_market_listed', { item: 'Item1', item_mint: 'Im1', seller: 'Owner1', price_lamports: 500, expires_at: 0 });
    await ins('ev_market_listed', { item: 'Item2', item_mint: 'Im2', seller: 'Owner2', price_lamports: 300, expires_at: 0 });
    await ins('ev_market_delisted', { item_mint: 'Im2', seller: 'Owner2' });
    await ins('ev_market_lease_offered', { item: 'Item2', lessor: 'Owner2', token_mint: 'TokB', slot_index: 0, rent_bps: 1000, fee_lamports: 25, term_secs: 100 });
    await ins('ev_market_lease_started', { item: 'Item2', payer: 'TokBDao', starts_at: NOW - 3600, ends_at: NOW + DAY });
    await ins('ev_market_commission_opened', { commission: 'C1', creator: 'X', token_mint: 'TokA', slot_index: 1, bounty_lamports: 900, closes_at: NOW + DAY, brief_uri: 'u' });
    await ins('ev_market_commission_opened', { commission: 'C2', creator: 'X', token_mint: 'TokA', slot_index: 2, bounty_lamports: 100, closes_at: NOW + DAY, brief_uri: 'u' });
    await ins('ev_market_commission_paid', { commission: 'C2', item: 'Item1', submitter: 'Y', bounty_lamports: 100 });
    const m = await market(db, W(NOW - 7 * DAY));
    expect(m.sales).toEqual({ count: 1, volumeLamports: '1000', feeLamports: '20', resaleLamports: '50' });
    expect(m.activeListings).toBe(1);
    expect(m.classes[0]).toMatchObject({ templateId: 3, name: 'Raid', listings: 1, floorLamports: '500', lastSaleLamports: '1000', sales: 1 });
    expect(m.licences).toMatchObject({ live: 1, bought: 1, incomeLamports: '100', protocolLamports: '11', authorLamports: '9', holderLamports: '80' });
    expect(m.leases).toEqual({ active: 1, started: 1, feesLamports: '25', rentLamports: '12' });
    expect(m.commissions).toEqual({ open: 1, openBountyLamports: '900', paid: 1, paidLamports: '100', refunded: 0 });
  });

  it('craft: drops by source, recipe uses and fees, fills and volume; the chain figures stay empty without a connection', async () => {
    await ins('ev_craft_dropped', { source: 0, material_id: 1, caller_program: 'Items', recipient: 'R', measured: 1000, amount: 40, season: 1 });
    await ins('ev_craft_dropped', { source: 1, material_id: 1, caller_program: 'War', recipient: 'R', measured: 10, amount: 2, season: 1 });
    await ins('ev_craft_crafted', { recipe: 'Rec1', crafter: 'C', template_id: 3, fee: 15, reference: 'r' });
    await ins('ev_craft_repaired', { recipe: 'Rec2', item: 'Item1', holder: 'H', restored: 5, used: 0, fee: 5, reference: 'r' });
    await ins('ev_book_filled', { market: 'Mkt1', maker_order: 1, maker: 'M', taker: 'T', side: 0, price: 7, size: 3, taker_fee: 1, maker_fee: 0, reference: 'r' });
    await ins('ev_book_class_filled', { bid: 'Bid1', item: 'Item1', seller: 'S', bidder: 'B', price: 444, taker_fee: 4, maker_fee: 0, reference: 'r' });
    await db.query(`insert into wear (item, max_charges, used, dormant, repairs) values ('Item1', 10, 10, true, 2), ('Item2', 10, 1, false, 0)`);
    const c = await craft(db, null, W(NOW - 7 * DAY));
    expect(c.chain).toBe(false);
    expect(c.dropsBySource).toEqual([{ source: 0, materialId: 1, amount: '40', drops: 1 }, { source: 1, materialId: 1, amount: '2', drops: 1 }]);
    expect(c.recipes).toEqual({ crafts: 1, repairs: 1, feesLamports: '20' });
    expect(c.classFills).toEqual({ count: 1, volumeLamports: '444' });
    expect(c.wear).toEqual({ tracked: 2, dormant: 1, repairs: 2 });
    expect(c.materials).toEqual([]);
  });

  it('cranks: bounties by crank type, top crankers named by their events, agent records by kind', async () => {
    await ins('ev_war_siege_executed', { mint: 'TokA', rival_mint: 'TokB', spent: 1000, bought: 5, bounty: 8, cranker: 'K1', captured_total: 5 });
    await ins('ev_war_razed', { mint: 'TokA', rival_mint: 'TokB', sold: 2, got: 600, bounty: 4, cranker: 'K2', captured_left: 3 });
    await ins('ev_book_expired', { market: 'Mkt1', id: 2, owner: 'O', cranker: 'K1', bounty: 1 });
    await ins('ev_agents_passport_registered', { passport: 'Pass1', operator: 'Op', agent_key: 'Ak', name: 'scout', kinds: 1, badge_mint: 'Bm' });
    await ins('ev_agents_agent_credited', { passport: 'Pass1', kind: 3, value: 70 });
    await ins('ev_agents_agent_credited', { passport: 'Pass1', kind: 4, value: 400 });
    const k = await cranks(db, W(NOW - 7 * DAY));
    const by = Object.fromEntries(k.byKind.map((x) => [x.kind, x]));
    expect(by.settle).toEqual({ kind: 'settle', runs: 2, bountyLamports: '10' });
    expect(by.siege).toEqual({ kind: 'siege', runs: 1, bountyLamports: '8' });
    expect(by.prize).toEqual({ kind: 'prize', runs: 0, bountyLamports: null });
    expect(k.top).toEqual([{ cranker: 'K1', runs: 2, bountyLamports: '9' }, { cranker: 'K2', runs: 1, bountyLamports: '4' }]);
    expect(k.agents).toEqual([{ passport: 'Pass1', name: 'scout', byKind: { royaltyClaim: '70', crank: '400' }, records: 2 }]);
  });

  it('war: inflows, outflows, boss pool and prize; chests by funding; season window from the last opened season', async () => {
    await ins('ev_war_war_funded', { mint: 'TokA', amount: 5000, balance: 5000, funded_total: 5000 });
    await ins('ev_war_war_funded', { mint: 'TokB', amount: 700, balance: 700, funded_total: 700 }, NOW - 40 * DAY);
    await ins('ev_war_boss_pool_funded', { season: 1, amount: 30, funded: 30 });
    await ins('ev_war_prize_paid', { season: 1, winner: 'TokA', to_winner: 90, to_treasury: 10, bounty: 1, cranker: 'K3' });
    await ins('ev_war_season_opened', { season: 2, starts_at: NOW - 2 * DAY, ends_at: NOW + 12 * DAY }, NOW - 2 * DAY);
    const w = await war(db, W(NOW - 7 * DAY));
    expect(w.inflows).toMatchObject({ funded: '5000', fundings: 1 });
    expect(w.outflows).toMatchObject({ siegeSpent: '1000', sieges: 1, razeProceeds: '600', razes: 1 });
    expect(w.boss).toEqual({ funded: '30', claimed: null, sealed: 0 });
    expect(w.prize).toEqual({ toWinner: '90', toTreasury: '10', paid: 1 });
    expect(w.chests[0]).toEqual({ mint: 'TokA', fundedLamports: '5000', siegeSpentLamports: '1000', razeLamports: '600' });
    const season = await windowOf(db, 'season', NOW);
    expect(season).toMatchObject({ from: NOW - 2 * DAY, seasonNumber: 2, bucketSecs: DAY });
    const all = await economy(db, null, 'war', 'all', NOW) as { window: Window; inflows: { funded: string } };
    expect(all.inflows.funded).toBe('5700');
    expect((await windowOf(db, '24h', NOW)).bucketSecs).toBe(3600);
  });
});
