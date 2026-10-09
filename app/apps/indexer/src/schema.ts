// Changed by Hookwars: mint lookup tables; views over the market, social and agents events.
/**
 * The indexer's Postgres schema (docs/spec/06-app.md 2.3). Amounts are `numeric(39,0)`, addresses
 * `text`, times `timestamptz` plus raw unix seconds. Every event row is keyed by
 * `(signature, ordinal)`, so re-indexing is idempotent.
 *
 * - `events`: every decoded event of every program, raw (upstream and Hookwars alike).
 * - `ev_<event>`: one typed table per Hookwars event, generated from the SDK's event schemas, so a
 *   schema change in a program shows up as a column change here. 06's tables (`raids`, `sieges`,
 *   `bounties` and the rest) are views over these.
 * - State tables kept current by handlers: cursors, program info, slots, items, item owners,
 *   proposals, vote locks, holding hook data.
 */
import { hookwars } from '@hookwars/sdk';

type Ty = hookwars.Ty;

export const snake = (s: string): string => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
export const eventTable = (program: string, name: string): string => `ev_${program}_${snake(name).replace(/^_/, '')}`;

function sqlType(ty: Ty): string {
  if (typeof ty === 'string') {
    switch (ty) {
      case 'bool': return 'boolean';
      case 'pubkey': case 'string': return 'text';
      default: return 'numeric(39,0)';
    }
  }
  if ('option' in ty) return sqlType(ty.option);
  if ('enum' in ty) return 'text';
  if ('bytes' in ty) return 'text';
  return 'jsonb';
}

/** The column of an event field. Base columns are `signature, ordinal, slot, ts`; an event's own
 * `slot` is a slot index (u8, renamed `slot_index`) or a chain slot (renamed `event_slot`); its `ts`
 * fills the base `ts` and is not repeated. */
export function colName(field: string, ty: Ty): string | null {
  if (field === 'ts') return null;
  if (field === 'slot') return ty === 'u8' ? 'slot_index' : 'event_slot';
  return snake(field);
}

const EVENT_COLS = 'signature text not null, ordinal int not null, slot bigint not null, ts bigint, block_time timestamptz';

export function eventTablesDdl(): string[] {
  const out: string[] = [];
  for (const [program, specs] of Object.entries(hookwars.EVENT_SPECS)) {
    for (const [name, fields] of specs) {
      const cols = fields.flatMap(([f, ty]) => { const c = colName(f, ty); return c ? [`"${c}" ${sqlType(ty)}`] : []; }).join(', ');
      out.push(`create table if not exists ${eventTable(program, name)} (${EVENT_COLS}, ${cols ? `${cols}, ` : ''}primary key (signature, ordinal))`);
    }
  }
  return out;
}

export const CORE_DDL: string[] = [
  `create table if not exists cursors (program text primary key, address text not null, last_signature text, last_slot bigint, updated_at timestamptz)`,
  `create table if not exists events (${EVENT_COLS}, program text not null, program_id text, name text not null, via text not null, data jsonb not null, primary key (signature, ordinal))`,
  `create index if not exists events_name on events (name, slot desc)`,
  `create table if not exists transactions (signature text primary key, slot bigint not null, block_time timestamptz, logs_truncated boolean not null default false, programs text[] not null)`,
  `create table if not exists program_info (program text primary key, address text not null, deployed boolean, upgrade_authority text, executable_hash text, checked_at timestamptz)`,
  // State kept by handlers (06 2.3 "slots", "items", "item_owners", "proposals", "vote_locks", "holding_hook_data").
  `create table if not exists slots (mint text not null, slot int not null, kind int, equip_rule int, max_cut_bps int, may_refuse boolean, may_write_data boolean, may_answer_touch boolean, data_offset int, data_len int, equip_vault text, locked_program text, item text, program text, flags int, pool_flags int, data_epoch int, updated_slot bigint, primary key (mint, slot))`,
  `create table if not exists items (item text primary key, item_mint text, template_id int, params jsonb, manifest jsonb, author text, royalty_bps int, level int, source text, closed boolean not null default false, created_slot bigint)`,
  `create table if not exists item_owners (item_mint text primary key, owner text, updated_slot bigint)`,
  `create table if not exists proposals (proposal text primary key, mint text, slot int, nonce numeric(39,0), proposer text, item text, vote_end bigint, executable_at bigint, status text, votes_for numeric(39,0), votes_against numeric(39,0), eligible numeric(39,0), updated_slot bigint)`,
  `create table if not exists vote_locks (holding text primary key, mint text, owner text, amount numeric(39,0), until bigint, updated_slot bigint)`,
  `create table if not exists holding_hook_data (holding text primary key, mint text, owner text, data text, updated_slot bigint)`,
  `create table if not exists templates (template_id int primary key, program text, code_hash text, deploy_slot numeric(39,0), kind int, field_count int, field_min jsonb, field_max jsonb, name text, status text not null default 'active', registered_slot bigint)`,
  `create table if not exists mint_tables (mint text primary key, lookup_table text not null, updated_slot bigint)`,
  `create table if not exists bot_posts (signature text not null, ordinal int not null, channel text not null, posted_at timestamptz not null default now(), primary key (signature, ordinal, channel))`,
];

/** Views with 06 2.3's table names over the typed event tables. */
export const VIEWS_DDL: string[] = [
  `create or replace view raids as select signature, ordinal, slot, ts, mint, rival, trader, volume, points, loot_ticket from ev_items_raid_marked`,
  `create or replace view shield_takes as select signature, ordinal, slot, ts, mint, owner, cut from ev_items_shield_taken`,
  `create or replace view settlements as select * from ev_items_equip_settled`,
  `create or replace view royalty_claims as select * from ev_armory_royalty_claimed`,
  `create or replace view sieges as select signature, ordinal, slot, ts, mint, rival_mint, spent, bought, bounty, cranker, captured_total, null::numeric as rival_price, null::numeric as rival_twap, 'executed' as status from ev_war_siege_executed
     union all select signature, ordinal, slot, ts, mint, rival_mint, null, null, null, null, null, rival_price, rival_twap, 'waited' from ev_war_siege_waited`,
  `create or replace view counter_strikes as select * from ev_war_counter_strike_executed`,
  `create or replace view razes as select * from ev_war_razed`,
  `create or replace view captured_returns as select * from ev_war_captured_returned`,
  `create or replace view bounties as select * from ev_war_bounty_claimed`,
  `create or replace view quests as select * from ev_war_quest_claimed`,
  `create or replace view route_swaps as select * from ev_swap_route_swapped`,
  `create or replace view war_chests as select * from ev_war_war_chest_created`,
  `create or replace view prizes as select * from ev_war_prize_paid`,
  // Market and social (10): sales are the only price history the site shows.
  `create or replace view item_sales as select * from ev_market_sold`,
  `create or replace view item_listings as select * from ev_market_listed`,
  `create or replace view badge_awards as select * from ev_social_badge_awarded`,
  `create or replace view passports as select * from ev_agents_passport_registered`,
];

export function allDdl(): string[] {
  return [...CORE_DDL, ...eventTablesDdl(), ...VIEWS_DDL];
}
