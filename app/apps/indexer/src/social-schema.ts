// Changed by Hookwars: new file, the social layer's tables and views (memo messages, follows, reactions, hides, profiles).
/**
 * Social tables (11 4.2 to 4.5, 10). Every row comes from a memo or an event on chain; nothing is
 * entered by hand. A memo that does not parse, or whose sender did not sign it, is kept raw with its
 * error and is never threaded or counted.
 *
 * - `social_messages`: one row per Memo program instruction of an indexed author's transaction,
 *   keyed by message id `<signature>:<instruction index>`.
 * - `social_follows`, `social_reactions`: state from `follow`, `unfollow` and `react` messages, last
 *   message wins by slot.
 * - `social_hides`: every `hide` message, the public record; the API applies only those whose
 *   author is a current admin (an operator setting), so the record never changes.
 * - `social_postage`: `MessagePosted` events by hex reference, to rank posted messages first.
 * - `social_authors`: the addresses whose signatures the indexer walks (agent keys, operators,
 *   wallets with a profile, extra addresses from the operator's settings), one cursor each.
 */
export const SOCIAL_DDL: string[] = [
  `create table if not exists social_messages (
    id text primary key, signature text not null, ix int not null, slot bigint not null, block_time timestamptz,
    signers text[] not null, raw text not null, valid boolean not null, error text,
    kind text, from_addr text, to_addr text, thread text, re text, body jsonb, expires_at numeric(39,0),
    author_kind text, author text, passport text, text text, model text, mint text, guild bigint,
    reference text not null)`,
  `create index if not exists social_messages_slot on social_messages (slot desc)`,
  `create index if not exists social_messages_thread on social_messages (thread, slot)`,
  `create index if not exists social_messages_author on social_messages (author, slot desc)`,
  `create index if not exists social_messages_kind on social_messages (kind, slot desc)`,
  `create index if not exists social_messages_mint on social_messages (mint, slot desc)`,
  `create index if not exists social_messages_guild on social_messages (guild, slot desc)`,
  `create index if not exists social_messages_ref on social_messages (reference)`,
  `create table if not exists social_follows (follower text not null, target text not null, following boolean not null, message_id text not null, slot bigint not null, primary key (follower, target))`,
  `create index if not exists social_follows_target on social_follows (target) where following`,
  `create table if not exists social_reactions (reactor text not null, ref text not null, reaction text not null, message_id text not null, slot bigint not null, block_time timestamptz, primary key (reactor, ref, reaction))`,
  `create index if not exists social_reactions_ref on social_reactions (ref)`,
  `create table if not exists social_hides (message_id text primary key, admin text not null, ref text not null, reason text, slot bigint not null, block_time timestamptz)`,
  `create table if not exists social_postage (signature text not null, ordinal int not null, reference text not null, passport text not null, postage numeric(39,0) not null, slot bigint not null, primary key (signature, ordinal))`,
  `create index if not exists social_postage_ref on social_postage (reference)`,
  `create table if not exists social_authors (address text primary key, role text not null, last_signature text, last_slot bigint, updated_at timestamptz)`,
];

/** Views over the agents and social event tables (they must exist first). */
export const SOCIAL_VIEWS_DDL: string[] = [
  // The agent key of a passport at any slot: the registration, then each rotation.
  `create or replace view passport_key_history as
     select passport, agent_key as key, slot from ev_agents_passport_registered
     union all select passport, new_key as key, slot from ev_agents_agent_key_rotated`,
  `create or replace view passport_current as
     select r.passport, r.operator, r.name, r.slot as registered_slot,
       coalesce((select h.key from passport_key_history h where h.passport = r.passport order by h.slot desc limit 1), r.agent_key) as agent_key,
       coalesce((select p."new" from ev_agents_proof_changed p where p.passport = r.passport order by p.slot desc, p.ordinal desc limit 1), 0)::int as proof,
       coalesce((select s.status from ev_agents_passport_status s where s.passport = r.passport order by s.slot desc, s.ordinal desc limit 1), 0)::int as status
     from ev_agents_passport_registered r`,
  `create or replace view directives as select * from ev_agents_directive_set`,
  `create or replace view commitments as select * from ev_agents_committed`,
  `create or replace view profiles_opened as select * from ev_social_profile_opened`,
  `create or replace view wallet_counters as select * from ev_social_wallet_recorded`,
];
