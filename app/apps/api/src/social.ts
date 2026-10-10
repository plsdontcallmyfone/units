// Changed by Hookwars: new file, the social layer's reads: wallet profiles, agent timelines, feeds, threads, leaderboards, live activity, the hide record.
/**
 * Every figure here is a chain fact: an account read now, or an event or memo the indexer holds.
 * Nothing is estimated; a figure neither holds is `null` with the reason next to it.
 *
 * Operator settings (none is a protocol parameter, none has a default in code):
 * - `MEMO_MIN_PROOF`: passports below this proof level are left out of feeds unless `all=1`
 *   (11 4.5; the parameter is to set, so without it no proof filter applies and the response says so).
 * - `SOCIAL_MAX_POSTS_PER_HOUR`, `SOCIAL_MAX_REACTIONS_PER_HOUR`: an author's posts or reactions
 *   past this count in one clock hour are not shown or counted (spam limits, Lineage pattern).
 * - `SOCIAL_ADMINS`: the admins whose hides apply; every hide stays in the public record.
 */
import { PublicKey, type Connection } from '@solana/web3.js';
import bs58 from 'bs58';
import type { Pool } from 'pg';
import { hookwars } from '@hookwars/sdk';
import { HttpError } from './guard.ts';

/** The admins whose hides apply (operator setting, comma-separated addresses). */
export function socialAdmins(env: Record<string, string | undefined> = process.env): string[] {
  return (env.SOCIAL_ADMINS ?? '').split(',').map((s) => s.trim()).filter(Boolean);
}

const MAX_ROWS = 200;
const PAGE = 50;
const ADDR = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;
const MSG_ID = /^[1-9A-HJ-NP-Za-km-z]{64,90}:\d{1,3}$/;

const json = (v: unknown): unknown => JSON.parse(JSON.stringify(v, (_k, x) => (typeof x === 'bigint' ? x.toString() : x instanceof PublicKey ? x.toBase58() : x)));
const envInt = (k: string): number | null => { const v = Number(process.env[k]); return Number.isInteger(v) && v >= 0 && process.env[k] !== undefined && process.env[k] !== '' ? v : null; };

function addr(v: string | undefined, what: string): string {
  if (!v || !ADDR.test(v)) throw new HttpError(400, 'BadRequest', `"${what}" is not an address.`);
  try { return new PublicKey(v).toBase58(); } catch { throw new HttpError(400, 'BadRequest', `"${what}" is not an address.`); }
}

// ---------------------------------------------------------------- chain reads --

/** `Profile` at `["profile", wallet]` (social, 11 3.3): counters by id (generated social IDL). */
export function decodeProfile(d: Buffer): { wallet: string; counters: bigint[]; createdAt: number; updatedAt: number } {
  const p = hookwars.profileCodec.decode(d);
  return { wallet: p.wallet.toBase58(), counters: p.counters, createdAt: Number(p.createdAt), updatedAt: Number(p.updatedAt) };
}

/** `SkillTable` at `["skills"]`: `{id, counter, thresholds[8]}` each (generated social IDL). */
export function decodeSkills(d: Buffer): { id: number; counter: number; thresholds: bigint[] }[] {
  return hookwars.skillTableCodec.decode(d).skills.map((k) => ({ id: k.id, counter: k.counter, thresholds: k.thresholds }));
}

/** `hookwars_common::economy::level`: thresholds met, stopping at the first 0. */
export function level(thresholds: bigint[], value: bigint): number {
  let l = 0;
  for (const t of thresholds) { if (t === 0n || value < t) break; l++; }
  return l;
}

/** `Profile.counters` length (`hookwars_common::economy::COUNTERS`). */
const COUNTER_COUNT = 16;

/** Counter ids (`hookwars_common::economy::counter`) and skill ids. */
export const COUNTERS = ['itemsAuthored', 'templatesRegistered', 'licencesSold', 'licenceRevenueLamports', 'itemsSold', 'itemsCrafted', 'repairs', 'bookFills', 'treatiesHeld', 'raids'] as const;
export const SKILLS = ['builder', 'crafter', 'trader', 'diplomat'] as const;

const profileAddress = hookwars.profileAddress;
const skillsAddress = hookwars.skillsAddress;

async function levelsOf(conn: Connection, wallet: PublicKey) {
  const [p, s] = await conn.getMultipleAccountsInfo([profileAddress(wallet), skillsAddress()], 'confirmed');
  if (!p) return { profile: null, counters: null, skills: null, reason: 'This wallet has not opened a profile, so no counters are recorded for it.' };
  const prof = decodeProfile(p.data);
  const counters = Object.fromEntries(COUNTERS.map((c, i) => [c, prof.counters[i]!.toString()]));
  if (!s) return { profile: profileAddress(wallet).toBase58(), openedAt: prof.createdAt, counters, skills: null, reason: 'No skill table on this cluster yet, so levels cannot be computed.' };
  const skills = decodeSkills(s.data).map((k) => ({
    id: k.id, name: SKILLS[k.id] ?? `skill ${k.id}`, counter: COUNTERS[k.counter] ?? `counter ${k.counter}`,
    value: prof.counters[k.counter]?.toString() ?? '0', level: level(k.thresholds, prof.counters[k.counter] ?? 0n),
    next: k.thresholds.find((t) => t > (prof.counters[k.counter] ?? 0n))?.toString() ?? null,
  }));
  return { profile: profileAddress(wallet).toBase58(), openedAt: prof.createdAt, counters, skills, reason: null };
}

/** Token holdings: `Holding` accounts of the token program with this owner (offset 42). */
async function holdingsOf(conn: Connection, db: Pool | null, wallet: PublicKey) {
  const res = await conn.getProgramAccounts(hookwars.TOKEN_ID, { commitment: 'confirmed', filters: [
    { memcmp: { offset: 0, bytes: bs58.encode(hookwars.holdingCodec.disc) } }, { memcmp: { offset: 42, bytes: wallet.toBase58() } },
  ] });
  const rows = res.slice(0, MAX_ROWS).flatMap((r) => { try { return [{ holding: r.pubkey.toBase58(), ...hookwars.holdingCodec.decode(r.account.data) }]; } catch { return []; } }).filter((h) => h.amount > 0n);
  const names = db ? await rowsOf<{ mint: string; name: string; symbol: string }>(db, 'select mint, name, symbol from ev_token_mint_created where mint = any($1)', [rows.map((r) => r.mint.toBase58())]) : [];
  const by = new Map(names.map((n) => [n.mint, n]));
  return rows.map((h) => ({ holding: h.holding, mint: h.mint.toBase58(), amount: h.amount.toString(), voteLocked: h.voteLocked.toString(), name: by.get(h.mint.toBase58())?.name ?? null, symbol: by.get(h.mint.toBase58())?.symbol ?? null }));
}

async function guildsOf(conn: Connection, db: Pool | null, wallet: string) {
  const res = await conn.getProgramAccounts(hookwars.SOCIAL_ID, { commitment: 'confirmed', filters: [{ memcmp: { offset: 0, bytes: bs58.encode(hookwars.guildCodec.disc) } }] });
  const officerOf = res.slice(0, MAX_ROWS).flatMap((r) => { try { const g = hookwars.guildCodec.decode(r.account.data); return g.officers.some((o) => o.toBase58() === wallet) ? [{ id: g.id, name: g.name, role: 'officer' }] : []; } catch { return []; } });
  const founded = db ? await rowsOf<{ id: number; name: string }>(db, 'select id, name from ev_social_guild_created where founder = $1', [wallet]) : [];
  const deposits = db ? await rowsOf<{ id: number; lamports: string }>(db, 'select id, sum(lamports)::text as lamports from ev_social_guild_deposit where "from" = $1 group by id', [wallet]) : [];
  const out = new Map<number, { id: number; name: string | null; roles: string[]; depositedLamports: string | null }>();
  const put = (id: number, name: string | null, role: string) => { const e = out.get(id) ?? { id, name, roles: [], depositedLamports: null }; if (!e.roles.includes(role)) e.roles.push(role); e.name = e.name ?? name; out.set(id, e); };
  officerOf.forEach((g) => put(g.id, g.name, 'officer'));
  founded.forEach((g) => put(Number(g.id), g.name, 'founder'));
  deposits.forEach((d) => { put(Number(d.id), null, 'depositor'); out.get(Number(d.id))!.depositedLamports = d.lamports; });
  return [...out.values()];
}

// ---------------------------------------------------------------- database reads --

async function rowsOf<T>(db: Pool, sql: string, args: unknown[]): Promise<T[]> {
  return (await db.query(sql, args)).rows as T[];
}

/** The feed filter as SQL, shared by every feed: valid posts, hides by current admins removed,
 * passports below the proof floor removed (unless `all`), authors over the hourly cap removed. */
function feedFilter(all: boolean, args: unknown[]): { sql: string; minProof: number | null; postsPerHour: number | null } {
  const minProof = envInt('MEMO_MIN_PROOF');
  const postsPerHour = envInt('SOCIAL_MAX_POSTS_PER_HOUR');
  args.push(socialAdmins());
  const adminsArg = `$${args.length}`;
  let sql = `m.valid and m.kind = 'status' and not exists (select 1 from social_hides h where h.ref = m.id and h.admin = any(${adminsArg}::text[]))`;
  sql += ` and (m.passport is null or exists (select 1 from passport_current pc where pc.passport = m.passport and pc.status = 0`;
  if (minProof !== null && !all) { args.push(minProof); sql += ` and pc.proof >= $${args.length}`; }
  sql += '))';
  if (postsPerHour !== null) {
    args.push(postsPerHour);
    sql += ` and (select count(*) from social_messages o where o.author = m.author and o.valid and o.kind = 'status' and date_trunc('hour', o.block_time) = date_trunc('hour', m.block_time) and (o.slot, o.id) < (m.slot, m.id)) < $${args.length}`;
  }
  return { sql, minProof, postsPerHour };
}

const ITEM_COLS = `m.id, m.signature, m.slot, m.block_time, m.kind, m.author, m.author_kind, m.passport, m.text, m.model, m.mint, m.guild, m.thread, m.re, m.to_addr, m.body`;

/** Adds names, reaction counts, reply counts and postage to message rows. */
async function decorate(db: Pool, rows: Record<string, unknown>[]): Promise<(Record<string, unknown> & { passportName: string | null; proof: number | null; reactions: Record<string, number>; replies: number; postageLamports: string | null })[]> {
  if (!rows.length) return [];
  const ids = rows.map((r) => String(r.id));
  const reactionsPerHour = envInt('SOCIAL_MAX_REACTIONS_PER_HOUR');
  const reacts = await rowsOf<{ ref: string; reaction: string; n: number }>(db,
    `select ref, reaction, count(*)::int as n from social_reactions r where ref = any($1)
     ${reactionsPerHour === null ? '' : `and (select count(*) from social_reactions o where o.reactor = r.reactor and date_trunc('hour', o.block_time) = date_trunc('hour', r.block_time) and (o.slot, o.message_id) < (r.slot, r.message_id)) < ${reactionsPerHour}`}
     group by ref, reaction`, [ids]);
  const replies = await rowsOf<{ thread: string; n: number }>(db, `select thread, count(*)::int - 1 as n from social_messages where valid and kind = 'status' and thread = any($1) group by thread`, [ids]);
  // Postage names a message by its reference (11 4.4, sha256 of the message id), or, paid in the
  // message's own transaction (runtime gap R-3: a post cannot know its own id), by sha256 of the
  // memo bytes; the second form counts only within that transaction, so equal texts never share it.
  const posted = await rowsOf<{ id: string; postage: string }>(db,
    `select m.id, sum(p.postage)::text as postage from social_messages m join social_postage p
       on p.reference = m.reference or (p.signature = m.signature and p.reference = encode(sha256(convert_to(m.raw, 'UTF8')), 'hex'))
     where m.id = any($1) group by m.id`, [ids]);
  const passports = [...new Set(rows.map((r) => r.passport).filter(Boolean))];
  const names = passports.length ? await rowsOf<{ passport: string; name: string; proof: number }>(db, 'select passport, name, proof from passport_current where passport = any($1)', [passports]) : [];
  const byP = new Map(names.map((n) => [n.passport, n]));
  return rows.map((r) => ({
    ...r,
    passportName: r.passport ? byP.get(String(r.passport))?.name ?? null : null,
    proof: r.passport ? byP.get(String(r.passport))?.proof ?? null : null,
    reactions: Object.fromEntries(hookwars.REACTIONS.map((k) => [k, reacts.find((x) => x.ref === r.id && x.reaction === k)?.n ?? 0])),
    replies: Math.max(0, replies.find((x) => x.thread === r.id)?.n ?? 0),
    postageLamports: posted.find((p) => p.id === String(r.id))?.postage ?? null,
  }));
}

/** Ranks posted messages above unposted ones within the page (11 4.5), newest first otherwise. */
const rankPosted = <T extends { postageLamports: string | null }>(items: T[]): T[] => [...items].sort((a, b) => Number(b.postageLamports !== null) - Number(a.postageLamports !== null));

export type FeedScope = 'global' | 'following' | 'token' | 'guild' | 'author';

export async function feed(db: Pool, q: URLSearchParams): Promise<unknown> {
  const scope = (q.get('scope') ?? 'global') as FeedScope;
  const all = q.get('all') === '1';
  const args: unknown[] = [];
  const f = feedFilter(all, args);
  let where = f.sql;
  if (scope === 'following') {
    const viewer = addr(q.get('viewer') ?? undefined, 'viewer');
    args.push(viewer);
    where += ` and m.author in (select target from social_follows where follower = $${args.length} and following)`;
  } else if (scope === 'token') {
    args.push(addr(q.get('mint') ?? undefined, 'mint'));
    where += ` and m.mint = $${args.length}`;
  } else if (scope === 'guild') {
    const g = Number(q.get('guild'));
    if (!Number.isInteger(g) || g < 0) throw new HttpError(400, 'BadRequest', '"guild" must be a guild id.');
    args.push(g);
    where += ` and m.guild = $${args.length}`;
  } else if (scope === 'author') {
    args.push(addr(q.get('author') ?? undefined, 'author'));
    where += ` and m.author = $${args.length}`;
  } else if (scope !== 'global') throw new HttpError(400, 'BadRequest', 'Unknown feed scope.');
  if (q.get('roots') !== '0') where += ' and m.thread = m.id';
  const before = q.get('before');
  if (before) { const b = Number(before); if (!Number.isInteger(b) || b < 0) throw new HttpError(400, 'BadRequest', '"before" must be a slot.'); args.push(b); where += ` and m.slot < $${args.length}`; }
  const rows = await rowsOf<Record<string, unknown>>(db, `select ${ITEM_COLS} from social_messages m where ${where} order by m.slot desc, m.id desc limit ${PAGE}`, args);
  const items = rankPosted(await decorate(db, rows));
  return json({
    scope, items, next: rows.length === PAGE ? String(rows[rows.length - 1]!.slot) : null,
    filter: { minProof: f.minProof, proofFilterApplied: f.minProof !== null && !all, postsPerHour: f.postsPerHour, note: f.minProof === null ? 'MEMO_MIN_PROOF is not set on this deployment, so posts from every proof level are shown.' : null },
  });
}

export async function thread(db: Pool, id: string): Promise<unknown> {
  if (!MSG_ID.test(id)) throw new HttpError(400, 'BadRequest', 'Not a message id.');
  const root = await rowsOf<Record<string, unknown>>(db, `select ${ITEM_COLS}, m.valid, m.error, m.raw from social_messages m where m.id = $1`, [id]);
  if (!root.length) return null;
  const threadId = String(root[0]!.thread ?? id);
  const args: unknown[] = [];
  const f = feedFilter(true, args);
  args.push(threadId);
  const msgs = await rowsOf<Record<string, unknown>>(db, `select ${ITEM_COLS} from social_messages m where ${f.sql.replace(`m.kind = 'status'`, `m.kind <> 'react' and m.kind <> 'follow' and m.kind <> 'unfollow' and m.kind <> 'hide'`)} and m.thread = $${args.length} order by m.slot asc, m.id asc limit ${MAX_ROWS}`, args);
  const hides = await rowsOf<Record<string, unknown>>(db, 'select message_id, admin, ref, reason, slot from social_hides where ref = $1', [id]);
  return json({ thread: threadId, focus: id, messages: await decorate(db, msgs), hidden: hides.filter((h) => socialAdmins().includes(String(h.admin))), raw: root[0]!.valid ? null : { raw: root[0]!.raw, error: root[0]!.error } });
}

export async function follows(db: Pool, address: string): Promise<unknown> {
  const a = addr(address, 'address');
  const followers = await rowsOf<{ follower: string; slot: string }>(db, 'select follower, slot from social_follows where target = $1 and following order by slot desc limit $2', [a, MAX_ROWS]);
  const following = await rowsOf<{ target: string; slot: string }>(db, 'select target, slot from social_follows where follower = $1 and following order by slot desc limit $2', [a, MAX_ROWS]);
  const counts = (await rowsOf<{ followers: number; following: number }>(db, `select (select count(*)::int from social_follows where target = $1 and following) as followers, (select count(*)::int from social_follows where follower = $1 and following) as following`, [a]))[0];
  return json({ address: a, ...counts, followerList: followers, followingList: following });
}

/** One wallet's profile (06 "portfolio", 11 3.3, 10). */
export async function walletProfile(conn: Connection, db: Pool | null, walletKey: string): Promise<unknown> {
  const wallet = new PublicKey(addr(walletKey, 'wallet'));
  const w = wallet.toBase58();
  const [lv, holdings, guilds] = await Promise.all([levelsOf(conn, wallet), holdingsOf(conn, db, wallet), guildsOf(conn, db, w)]);
  if (!db) return json({ wallet: w, levels: lv, holdings, guilds, indexed: null, reason: 'The indexer database is not reachable, so items, royalties, badges and posts are not shown.' });
  const [owned, authored, royaltiesQuote, claims, badges, passports, raids, follow] = await Promise.all([
    rowsOf(db, `select i.item, i.item_mint, i.template_id, i.level, i.author, t.name as template_name from item_owners o join items i on i.item_mint = o.item_mint left join templates t on t.template_id = i.template_id where o.owner = $1 and not i.closed order by i.created_slot desc limit $2`, [w, MAX_ROWS]),
    rowsOf<{ n: number; adopted: number }>(db, `select count(*)::int as n, (select count(distinct e.mint)::int from ev_token_slot_equipped e join items i2 on i2.item = e.new_item where i2.author = $1) as adopted from items where author = $1`, [w]),
    rowsOf<{ lamports: string | null }>(db, `select sum(s.royalty_quote)::text as lamports from ev_items_equip_settled s join items i on i.item = s.item where i.author = $1`, [w]),
    rowsOf(db, `select cut_mint, count(*)::int as claims, sum(amount)::text as amount from ev_armory_royalty_claimed where claimant = $1 group by cut_mint order by claims desc limit 50`, [w]),
    rowsOf(db, `select a.signature, a.slot, a.ts, a.id from ev_social_badge_awarded a where a.recipient = $1 order by a.slot desc limit 100`, [w]),
    rowsOf(db, `select passport, name, proof, status, case when operator = $1 then 'operator' else 'agent key' end as role from passport_current where operator = $1 or agent_key = $1`, [w]),
    rowsOf<{ volume: string | null; n: number }>(db, `select sum(volume)::text as volume, count(*)::int as n from ev_items_raid_marked where trader = $1`, [w]),
    follows(db, w),
  ]);
  const posts = await feed(db, new URLSearchParams({ scope: 'author', author: w, all: '1' }));
  return json({
    wallet: w, levels: lv, holdings, guilds, passports, itemsOwned: owned,
    authored: { items: authored[0]?.n ?? 0, adoptedByMints: authored[0]?.adopted ?? 0 },
    royalties: { settledToAuthoredItemsLamports: royaltiesQuote[0]?.lamports ?? null, claimsByMint: claims },
    raids: { count: raids[0]?.n ?? 0, volumeLamports: raids[0]?.volume ?? null },
    badges, follows: follow, posts,
  });
}

/** An agent's timeline: its memos and the agents program's events about it, newest first. */
export async function agentTimeline(db: Pool, passportKey: string, q: URLSearchParams): Promise<unknown> {
  const p = addr(passportKey, 'passport');
  const head = (await rowsOf<Record<string, unknown>>(db, 'select passport, operator, name, agent_key, proof, status, registered_slot from passport_current where passport = $1', [p]))[0] ?? null;
  const before = Number(q.get('before') ?? '') || null;
  const memos = await rowsOf<Record<string, unknown>>(db,
    `select ${ITEM_COLS}, m.valid, m.error from social_messages m where (m.passport = $1 or m.to_addr = $1) and ($2::bigint is null or m.slot < $2) order by m.slot desc limit ${PAGE}`, [p, before]);
  const events = await rowsOf<Record<string, unknown>>(db,
    `select signature, ordinal, slot, ts, program, name, data from events where program = 'agents' and data->>'passport' = $1 and ($2::bigint is null or slot < $2) order by slot desc, ordinal desc limit ${PAGE}`, [p, before]);
  const decorated = await decorate(db, memos.filter((m) => m.valid));
  const invalid = memos.filter((m) => !m.valid).map((m): Record<string, unknown> => ({ ...m, invalid: true }));
  const items = [
    ...decorated.map((m) => ({ type: 'memo' as const, slot: Number(m.slot), ...m })),
    ...invalid.map((m) => ({ type: 'memo' as const, slot: Number(m.slot), ...m })),
    ...events.map((e) => ({ type: 'event' as const, slot: Number(e.slot), ...e })),
  ].sort((a, b) => b.slot - a.slot).slice(0, PAGE);
  return json({ passport: p, agent: head, items, next: items.length === PAGE ? String(items[items.length - 1]!.slot) : null });
}

export const BOARDS = ['royalties', 'authors', 'treaties', 'raids', 'cranks', 'levels'] as const;
export type Board = typeof BOARDS[number];

/** Leaderboards from chain facts only (Lineage pattern, 10 agent league). */
export async function leaderboard(conn: Connection, db: Pool, board: string | null): Promise<unknown> {
  const b: Board = (BOARDS as readonly string[]).includes(board ?? '') ? board as Board : 'royalties';
  const limit = 50;
  let rows: Record<string, unknown>[] = [];
  let unit = '';
  let source = '';
  let note: string | null = null;
  switch (b) {
    case 'royalties':
      unit = 'lamports'; source = 'EquipSettled.royalty_quote of items each author created';
      rows = await rowsOf(db, `select i.author as who, sum(s.royalty_quote)::text as value, count(*)::int as events from ev_items_equip_settled s join items i on i.item = s.item group by i.author having sum(s.royalty_quote) > 0 order by sum(s.royalty_quote) desc limit ${limit}`, []);
      break;
    case 'authors':
      unit = 'tokens equipping'; source = 'ItemCreated by author, SlotEquipped of those items (distinct tokens)';
      rows = await rowsOf(db, `select i.author as who, count(distinct e.mint)::text as value, count(distinct i.item)::int as events from items i left join ev_token_slot_equipped e on e.new_item = i.item group by i.author order by count(distinct e.mint) desc, count(distinct i.item) desc limit ${limit}`, []);
      break;
    case 'treaties':
      unit = 'treaties held'; source = 'TreatyHeld by passport';
      rows = await rowsOf(db, `select t.passport as who, count(*)::text as value, (select count(*)::int from ev_agents_treaty_broken b where b.passport = t.passport) as events from ev_agents_treaty_held t group by t.passport order by count(*) desc limit ${limit}`, []);
      note = 'events = treaties broken by the same passport.';
      break;
    case 'raids':
      unit = 'lamports'; source = 'RaidMarked.volume by trader';
      rows = await rowsOf(db, `select trader as who, sum(volume)::text as value, count(*)::int as events from ev_items_raid_marked group by trader order by sum(volume) desc limit ${limit}`, []);
      break;
    case 'cranks':
      unit = 'cranks landed'; source = 'SiegeExecuted.cranker';
      rows = await rowsOf(db, `select cranker as who, count(*)::text as value, sum(bounty)::text as bounty from ev_war_siege_executed group by cranker order by count(*) desc limit ${limit}`, []);
      note = 'A failed crank is never on chain, so this counts cranks that landed; it is not a success rate.';
      break;
    case 'levels': {
      unit = 'levels (sum over skills)'; source = 'WalletRecorded totals and the SkillTable on chain';
      const info = await conn.getAccountInfo(skillsAddress(), 'confirmed');
      if (!info) { note = 'No skill table on this cluster yet, so no levels can be computed.'; break; }
      const skills = decodeSkills(info.data);
      const totals = await rowsOf<{ wallet: string; counter: number; total: string }>(db, `select distinct on (wallet, counter) wallet, counter::int, total::text from ev_social_wallet_recorded order by wallet, counter, slot desc, ordinal desc`, []);
      const by = new Map<string, bigint[]>();
      for (const t of totals) { const c = by.get(t.wallet) ?? Array<bigint>(COUNTER_COUNT).fill(0n); c[t.counter] = BigInt(t.total); by.set(t.wallet, c); }
      rows = [...by.entries()].map(([who, c]) => {
        const per = skills.map((k) => ({ skill: SKILLS[k.id] ?? `skill ${k.id}`, level: level(k.thresholds, c[k.counter] ?? 0n) }));
        return { who, value: String(per.reduce((s, x) => s + x.level, 0)), skills: per };
      }).filter((r) => r.value !== '0').sort((x, y) => Number(y.value) - Number(x.value)).slice(0, limit);
      break;
    }
  }
  const who = rows.map((r) => String(r.who));
  const names = who.length ? await rowsOf<{ passport: string; name: string; operator: string; agent_key: string }>(db, 'select passport, name, operator, agent_key from passport_current where passport = any($1) or agent_key = any($1)', [who]) : [];
  return json({ board: b, boards: BOARDS, unit, source, note, items: rows.map((r, i) => ({ rank: i + 1, ...r, agent: names.find((n) => n.passport === r.who || n.agent_key === r.who) ?? null })) });
}

/** Live activity: agents events and agent memos after `since` (a slot), oldest first, for polling. */
export async function live(db: Pool, q: URLSearchParams): Promise<unknown> {
  const since = Number(q.get('since') ?? '0');
  if (!Number.isInteger(since) || since < 0) throw new HttpError(400, 'BadRequest', '"since" must be a slot.');
  const events = await rowsOf<Record<string, unknown>>(db, `select signature, ordinal, slot, ts, block_time, program, name, data from events where program = 'agents' and slot > $1 order by slot desc, ordinal desc limit ${PAGE}`, [since]);
  const memos = await rowsOf<Record<string, unknown>>(db, `select ${ITEM_COLS} from social_messages m where m.valid and m.passport is not null and m.slot > $1 order by m.slot desc limit ${PAGE}`, [since]);
  const names = await rowsOf<{ passport: string; name: string }>(db, 'select passport, name from passport_current where passport = any($1)', [[...new Set([...events.map((e) => (e.data as Record<string, unknown>)?.passport), ...memos.map((m) => m.passport)].filter(Boolean))]]);
  const byP = new Map(names.map((n) => [n.passport, n.name]));
  const items = [
    ...events.map((e) => ({ type: 'event', key: `${e.signature}:${e.ordinal}`, slot: Number(e.slot), name: e.name, signature: e.signature, passport: (e.data as Record<string, unknown>)?.passport ?? null, data: e.data, blockTime: e.block_time })),
    ...memos.map((m) => ({ type: 'memo', key: String(m.id), slot: Number(m.slot), name: m.kind, signature: m.signature, passport: m.passport, text: m.text, model: m.model, blockTime: m.block_time })),
  ].map((x) => ({ ...x, agentName: x.passport ? byP.get(String(x.passport)) ?? null : null })).sort((a, b) => b.slot - a.slot).slice(0, PAGE);
  const tip = (await rowsOf<{ slot: string | null }>(db, 'select max(slot)::text as slot from (select max(slot) as slot from events union all select max(slot) from social_messages) t', []))[0]?.slot ?? null;
  return json({ since, tip, items });
}

/** The public record of every hide memo, whether its author is a current admin or not. */
export async function hides(db: Pool): Promise<unknown> {
  const admins = socialAdmins();
  const rows = await rowsOf<Record<string, unknown>>(db, 'select h.message_id, h.admin, h.ref, h.reason, h.slot, h.block_time, m.author as ref_author, m.text as ref_text from social_hides h left join social_messages m on m.id = h.ref order by h.slot desc limit $1', [MAX_ROWS]);
  return json({ admins, items: rows.map((r) => ({ ...r, applied: admins.includes(String(r.admin)) })) });
}

/** The social routes; null when the path is not one of them. */
export async function socialRoute(p: string, q: URLSearchParams, conn: Connection, db: Pool | null): Promise<{ status: number; body: unknown } | null> {
  let m: RegExpExecArray | null;
  const isSocial = p.startsWith('/v1/social/') || /^\/v1\/u\/\w+$/.test(p) || /^\/v1\/agents\/\w+\/timeline$/.test(p);
  if (!isSocial) return null;
  if ((m = /^\/v1\/u\/(\w{32,44})$/.exec(p))) return { status: 200, body: await walletProfile(conn, db, m[1]!) };
  if (!db) return { status: 503, body: { error: 'The database is not reachable.', code: 'NoDatabase' } };
  if ((m = /^\/v1\/agents\/(\w{32,44})\/timeline$/.exec(p))) return { status: 200, body: await agentTimeline(db, m[1]!, q) };
  if (p === '/v1/social/feed') return { status: 200, body: await feed(db, q) };
  if ((m = /^\/v1\/social\/threads\/(\w{64,90}:\d{1,3})$/.exec(p))) {
    const t = await thread(db, m[1]!);
    return t ? { status: 200, body: t } : { status: 404, body: { error: 'The indexer holds no such message.' } };
  }
  if ((m = /^\/v1\/social\/follows\/(\w{32,44})$/.exec(p))) return { status: 200, body: await follows(db, m[1]!) };
  if (p === '/v1/social/leaderboards') return { status: 200, body: await leaderboard(conn, db, q.get('board')) };
  if (p === '/v1/social/live') return { status: 200, body: await live(db, q) };
  if (p === '/v1/social/hides') return { status: 200, body: await hides(db) };
  return { status: 404, body: { error: 'Not found.' } };
}
