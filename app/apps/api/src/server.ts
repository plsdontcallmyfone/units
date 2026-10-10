// Changed by Hookwars: read routes for agents, market, commissions, guilds and badges; prepares get the database for mint lookup tables; social routes.
/**
 * The Hookwars API (docs/spec/06-app.md 3.3) on node:http. JSON in and out, bigint as decimal
 * strings, `null` for anything not read, errors as upstream's `ApiErrorBody`.
 */
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { randomUUID } from 'node:crypto';
import { Connection, PublicKey } from '@solana/web3.js';
import type { Pool } from 'pg';
import sharp from 'sharp';
import { hookwars } from '@hookwars/sdk';
import { findBannedWords, PARAMS, type PrizeVaultInfo, type SeasonInfo } from '@hookwars/shared';
import * as reads from './reads.ts';
import * as xreads from './reads-expansion.ts';
import { prepare, PrepareError } from './prepares.ts';
import { submit } from './submit.ts';
import { socialRoute } from './social.ts';
import * as explorer from './explorer.ts';
import * as launchPlan from './launch-plan.ts';
import { clientKey, clusterName, HttpError, intParam, RateLimiter, readJsonBody } from './guard.ts';

export interface Deps { db: Pool | null; conn: Connection; rpcUrl: string }

const json = (res: ServerResponse, status: number, body: unknown): void => {
  const text = JSON.stringify(body, (_k, v) => (typeof v === 'bigint' ? v.toString() : v));
  res.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store' });
  res.end(text);
};

/** Operator settings for the rate limits (A-4); none is a protocol parameter. */
const envNum = (k: string, d: number): number => { const v = Number(process.env[k]); return Number.isFinite(v) && v > 0 ? v : d; };
const prepareLimit = new RateLimiter(envNum('RATE_PREPARE_CAPACITY', 20), envNum('RATE_PREPARE_PER_SEC', 0.5));
const readLimit = new RateLimiter(envNum('RATE_READ_CAPACITY', 120), envNum('RATE_READ_PER_SEC', 4));

/** A short-lived cache for the aggregate reads (A-7). */
const cache = new Map<string, { at: number; value: unknown }>();
async function cached<T>(key: string, ttlMs: number, f: () => Promise<T>): Promise<T> {
  const c = cache.get(key);
  if (c && Date.now() - c.at < ttlMs) return c.value as T;
  const value = await f();
  cache.set(key, { at: Date.now(), value });
  return value;
}

async function currentSeason(conn: Connection): Promise<number | null> {
  try {
    const info = await conn.getAccountInfo(hookwars.WAR_CONFIG, 'confirmed');
    return info ? hookwars.warConfigCodec.decode(info.data).currentSeason : null;
  } catch { return null; }
}

async function season(conn: Connection, n: number): Promise<SeasonInfo | null> {
  const info = await conn.getAccountInfo(hookwars.seasonAddress(n), 'confirmed');
  if (!info) return null;
  const s = hookwars.seasonCodec.decode(info.data);
  const w = s.weights;
  return {
    number: s.number, startsAt: Number(s.startsAt), endsAt: Number(s.endsAt), challengeEndsAt: null,
    weights: { raidVolumeWon: String(w.raidVolumeWon), sieges: String(w.sieges), siegeSpend: String(w.siegeSpend), timesBesieged: String(w.timesBesieged), counterStrikes: String(w.counterStrikes), treatySecs: String(w.treatySecs) },
    penalizeBesieged: s.penalizeBesieged, table: [],
    leader: s.leader ? { mint: s.leader.toBase58(), score: String(s.leaderScore) } : null,
    finalized: s.finalized, winner: null, prizePaid: String(s.prizePaid),
  };
}

async function prizeVault(conn: Connection): Promise<PrizeVaultInfo> {
  const [vault, cfg] = await conn.getMultipleAccountsInfo([hookwars.PRIZE_VAULT, hookwars.WAR_CONFIG], 'confirmed');
  let lastWinner: string | null = null;
  try { if (cfg) lastWinner = hookwars.warConfigCodec.decode(cfg.data).lastWinner?.toBase58() ?? null; } catch { /* not decodable */ }
  return { vault: hookwars.PRIZE_VAULT.toBase58(), lamports: vault ? String(vault.lamports) : null, quoteHolding: null, lastWinner, shareBps: PARAMS.find((p) => p.name === 'SEASON_PRIZE_SHARE_BPS')?.value ?? null };
}

/** 06 7.2: a card from one indexed event and nothing else; an unknown signature is a 404. */
async function card(db: Pool, signature: string, ordinal: number): Promise<Buffer | null> {
  const r = (await db.query('select * from events where signature = $1 and ordinal = $2', [signature, ordinal])).rows[0];
  if (!r) return null;
  const d = r.data ?? {};
  const line = ((): string => {
    switch (r.name) {
      case 'RaidMarked': return `raided ${String(d.rival).slice(0, 4)} with ${(Number(d.volume) / 1e9).toString()} SOL`;
      case 'SiegeExecuted': return `won a siege on ${String(d.rivalMint).slice(0, 4)}`;
      case 'Forged': return `forged a level ${d.level} item`;
      case 'BountyClaimed': return `claimed ${(Number(d.paid) / 1e9).toString()} SOL in bounties`;
      case 'SeasonFinalized': return `season ${d.season} winner`;
      default: return r.name;
    }
  })();
  if (findBannedWords(line).length) return null;
  const esc = (s: string) => s.replace(/[<>&"]/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;' })[c]!);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630"><rect width="1200" height="630" fill="#0e0f12"/>
<text x="80" y="140" fill="#9aa0aa" font-family="Geist, Inter, Arial, sans-serif" font-size="36">units</text>
<text x="80" y="330" fill="#f2f3f5" font-family="Geist, Inter, Arial, sans-serif" font-size="72" font-weight="600">${esc(line)}</text>
<text x="80" y="560" fill="#6b7280" font-family="Geist, Inter, Arial, sans-serif" font-size="28">${esc(signature.slice(0, 8))}... slot ${r.slot}</text></svg>`;
  return sharp(Buffer.from(svg)).png().toBuffer();
}

export function handler(deps: Deps) {
  return async (req: IncomingMessage, res: ServerResponse): Promise<void> => {
    const url = new URL(req.url ?? '/', 'http://local');
    const p = url.pathname.replace(/\/+$/, '');
    const q = url.searchParams;
    const db = deps.db;
    try {
      const client = clientKey(req);
      if (req.method === 'POST' && p.startsWith('/v1/') && p.endsWith('/prepare')) {
        if (!prepareLimit.take(client)) return json(res, 429, { error: 'Too many requests; wait a little.', code: 'RateLimited' });
        return json(res, 200, await prepare(deps.conn, p.slice(4), await readJsonBody(req), db));
      }
      // Changed by Hookwars: wallet-signed transactions go out through the API (A-2), limited as prepares are.
      if (req.method === 'POST' && p === '/v1/submit') {
        if (!prepareLimit.take(client)) return json(res, 429, { error: 'Too many requests; wait a little.', code: 'RateLimited' });
        return json(res, 200, await submit(deps.conn, await readJsonBody(req)));
      }
      // Changed by Hookwars (explorer v2, launch page): the launch walked step by step, a simulation per step.
      if (req.method === 'POST' && (p === '/v1/launch/plan' || p === '/v1/simulate')) {
        if (!prepareLimit.take(client)) return json(res, 429, { error: 'Too many requests; wait a little.', code: 'RateLimited' });
        const body = await readJsonBody(req);
        return json(res, 200, p === '/v1/simulate' ? await launchPlan.simulate(deps.conn, body) : await launchPlan.launchPlan(deps.conn, body));
      }
      if (!readLimit.take(client)) return json(res, 429, { error: 'Too many requests; wait a little.', code: 'RateLimited' });
      if (req.method !== 'GET') return json(res, 405, { error: 'Method not allowed.' });
      if (p === '/v1/status') return json(res, 200, await reads.status(db, { cluster: clusterName(deps.rpcUrl), slot: () => deps.conn.getSlot('confirmed') }));
      if (p === '/v1/params') return json(res, 200, PARAMS);
      if (p === '/v1/quests') return json(res, 200, reads.QUESTS);
      if (p === '/v1/prize-vault') return json(res, 200, await prizeVault(deps.conn));
      if (p === '/v1/seasons/current') {
        const n = await currentSeason(deps.conn);
        return json(res, 200, n === null || n === 0 ? null : await season(deps.conn, n));
      }
      let m: RegExpExecArray | null;
      if ((m = /^\/v1\/seasons\/(\d+)$/.exec(p))) return json(res, 200, await season(deps.conn, intParam(m[1], 'season', 0, 4_294_967_295)));
      if ((m = /^\/v1\/seasons\/(\d+)\/loot$/.exec(p))) {
        const info = await deps.conn.getAccountInfo(hookwars.lootTableAddress(intParam(m[1], 'season', 0, 4_294_967_295)), 'confirmed');
        if (!info) return json(res, 200, null);
        const t = hookwars.decodeLootTable(info.data);
        return json(res, 200, { season: t.season, eta: Number(t.eta), entries: t.entries.filter((e) => e.weight > 0).map((e) => ({ templateId: e.templateId, templateName: String(e.templateId), weight: e.weight, ranges: e.ranges })) });
      }
      // Agents, market and social (09, 10): chain state read directly, history from the indexer when it is up.
      // A missing account reads as null (the page says what is missing), as /war does.
      // Social layer: profiles, timelines, feeds, threads, leaderboards, live, hides.
      const social = await socialRoute(p, q, deps.conn, db);
      if (social) return json(res, social.status, social.body);
      if (p === '/v1/agents') return json(res, 200, await cached(`agents:${q.get('sort') ?? ''}`, 15_000, () => xreads.agentsLeague(deps.conn, q.get('sort'))));
      if ((m = /^\/v1\/agents\/(\w{32,44})$/.exec(p))) { const a = await xreads.agent(deps.conn, m[1]!); return json(res, 200, a); }
      if (p === '/v1/market/listings') return json(res, 200, await cached('listings', 10_000, () => xreads.listings(deps.conn, db)));
      if ((m = /^\/v1\/market\/items\/(\w{32,44})$/.exec(p))) { const it = await xreads.marketItem(deps.conn, db, m[1]!); return json(res, 200, it); }
      // The hook economy (app pass v3): craft, the order books and item wear.
      if (p === '/v1/craft') return json(res, 200, await cached('craft', 15_000, () => xreads.craftOverview(deps.conn)));
      if (p === '/v1/book') return json(res, 200, await cached('books', 10_000, () => xreads.books(deps.conn, null)));
      if ((m = /^\/v1\/book\/(\d{1,5})$/.exec(p))) return json(res, 200, await xreads.books(deps.conn, Number(m[1])));
      if ((m = /^\/v1\/items\/(\w{32,44})\/wear$/.exec(p))) return json(res, 200, await xreads.itemWear(deps.conn, m[1]!));
      if (p === '/v1/market/leases') return json(res, 200, await cached('leases', 10_000, () => xreads.leases(deps.conn)));
      if (p === '/v1/market/collections') return json(res, 200, await cached('collections', 30_000, () => xreads.collections(deps.conn)));
      if (p === '/v1/commissions') return json(res, 200, await cached('commissions', 10_000, () => xreads.commissions(deps.conn)));
      if ((m = /^\/v1\/commissions\/(\w{32,44})$/.exec(p))) { const c = await xreads.commission(deps.conn, m[1]!); return json(res, 200, c); }
      if (p === '/v1/guilds') return json(res, 200, await cached('guilds', 15_000, () => xreads.guilds(deps.conn)));
      if ((m = /^\/v1\/guilds\/(\d+)$/.exec(p))) { const g = await xreads.guild(deps.conn, intParam(m[1], 'guild', 0, 4_294_967_295)); return json(res, 200, g); }
      if (p === '/v1/badges') return json(res, 200, await cached('badges', 15_000, () => xreads.badges(deps.conn, db)));
      // Changed by Hookwars (explorer v2): chain reads, the indexer optional.
      if (p === '/v1/launch/config') return json(res, 200, await launchPlan.launchConfig(deps.conn));
      if (p === '/v1/launch/buy-quote') return json(res, 200, await launchPlan.buyQuote(deps.conn, q));
      if (p.startsWith('/v1/explorer/')) { const r = await explorer.route(deps.conn, db, p, q); if (r) return json(res, r.status, r.body); }
      if (!db) return json(res, 503, { error: 'The database is not reachable.', code: 'NoDatabase' });
      if (p === '/v1/templates') return json(res, 200, await reads.templates(db));
      if (p === '/v1/items') return json(res, 200, await reads.items(db, q));
      if ((m = /^\/v1\/items\/(\w+)$/.exec(p))) {
        const it = await reads.item(db, m[1]!);
        return it ? json(res, 200, it) : json(res, 404, { error: 'No such item.' });
      }
      if ((m = /^\/v1\/launches\/(\w+)\/slots$/.exec(p))) return json(res, 200, await reads.slots(db, m[1]!));
      if ((m = /^\/v1\/launches\/(\w+)\/proposals$/.exec(p))) return json(res, 200, await reads.proposals(db, m[1]!, q.get('status')));
      if ((m = /^\/v1\/launches\/(\w+)\/generals$/.exec(p))) { const mint = m[1]!; return json(res, 200, await cached(`generals:${mint}`, 30_000, async () => reads.generals(db, mint, await currentSeason(deps.conn)))); }
      if ((m = /^\/v1\/launches\/(\w+)\/treaties$/.exec(p))) return json(res, 200, []);
      if ((m = /^\/v1\/launches\/(\w+)\/war$/.exec(p))) {
        const mint = new PublicKey(m[1]!);
        const info = await deps.conn.getAccountInfo(hookwars.warStateAddress(mint), 'confirmed');
        return json(res, 200, info ? { mint: mint.toBase58(), chest: hookwars.warChestAddress(mint).toBase58() } : null);
      }
      if (p === '/v1/launches') {
        const rows = (await db.query(`select data from events where name = 'LaunchCreated' order by slot desc limit 100`)).rows;
        return json(res, 200, { items: rows.map((r) => r.data), next: null });
      }
      if (p === '/v1/map') return json(res, 200, await cached('map', 30_000, () => reads.warMap(db, Math.floor(Date.now() / 1000))));
      if (p === '/v1/feed') return json(res, 200, await reads.feed(db, q));
      if ((m = /^\/v1\/wallet\/(\w+)\/war$/.exec(p))) {
        const owner = m[1]!;
        return json(res, 200, { holdings: [], items: (await reads.items(db, new URLSearchParams({ owner }))).items, rolls: [] });
      }
      if ((m = /^\/v1\/cards\/(\w+)\/(\d+)\.png$/.exec(p))) {
        const png = await card(db, m[1]!, Number(m[2]));
        if (!png) return json(res, 404, { error: 'The indexer holds no such event.' });
        res.writeHead(200, { 'content-type': 'image/png' });
        return void res.end(png);
      }
      return json(res, 404, { error: 'Not found.' });
    } catch (e) {
      if (e instanceof PrepareError || e instanceof HttpError) return json(res, e.status, { error: e.message, code: e.code });
      // A-8: the detail stays in the server log; the client gets a request id.
      const id = randomUUID();
      console.error(`[api ${id}] ${req.method} ${p}:`, e instanceof Error ? e.stack ?? e.message : String(e));
      return json(res, 500, { error: 'The backend could not read this.', requestId: id });
    }
  };
}

export function serve(deps: Deps, port: number, host = '127.0.0.1'): Server {
  const s = createServer((req, res) => { void handler(deps)(req, res); });
  s.listen(port, host);
  return s;
}
