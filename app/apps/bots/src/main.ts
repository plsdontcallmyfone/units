// Changed by Hookwars: send reports success only on 2xx (app audit A-10).
/**
 * Polls the API's /v1/feed and posts new events once each, keyed by (signature, ordinal) in a local
 * state file, so a replay of the same range posts nothing twice (06 section 8). Disabled unless TELEGRAM_BOT_TOKEN + TELEGRAM_CHAT_ID or
 * X_BEARER_TOKEN are set; without them it logs what it would post and exits.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import type { BattleEvent, Page } from '@hookwars/shared';
import { postFor } from './posts.ts';
import { deliver } from './deliver.ts';

const api = process.env.API_URL ?? 'http://127.0.0.1:9961';
const state = process.env.BOTS_STATE ?? '/root/hw-app-bots-posted.json';
const tg = process.env.TELEGRAM_BOT_TOKEN && process.env.TELEGRAM_CHAT_ID ? { token: process.env.TELEGRAM_BOT_TOKEN, chat: process.env.TELEGRAM_CHAT_ID } : null;
const x = process.env.X_BEARER_TOKEN ?? null;
const cfg = {
  siteUrl: process.env.SITE_URL ?? 'http://127.0.0.1:9960',
  explorerTx: (s: string) => `https://explorer.solana.com/tx/${s}?cluster=${process.env.CLUSTER ?? 'devnet'}`,
  minRaidLamports: process.env.MIN_RAID_LAMPORTS ? BigInt(process.env.MIN_RAID_LAMPORTS) : null,
};

const posted = new Set<string>(existsSync(state) ? (JSON.parse(readFileSync(state, 'utf8')) as string[]) : []);

/** Sends to every enabled channel; true only when each answered 2xx (A-10). */
async function send(text: string): Promise<boolean> {
  let ok = true;
  if (tg) {
    const r = await fetch(`https://api.telegram.org/bot${tg.token}/sendMessage`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ chat_id: tg.chat, text, disable_web_page_preview: true }) });
    ok &&= r.ok;
  }
  if (x) {
    const r = await fetch('https://api.twitter.com/2/tweets', { method: 'POST', headers: { authorization: `Bearer ${x}`, 'content-type': 'application/json' }, body: JSON.stringify({ text }) });
    ok &&= r.ok;
  }
  return ok;
}

const r = await fetch(`${api}/v1/feed`);
const feed = (await r.json()) as Page<BattleEvent>;
const events = [...feed.items].reverse();
const result = await deliver(
  events,
  posted,
  (e) => postFor(e, cfg),
  async (text) => {
    if (!tg && !x) { console.log(`[disabled, would post] ${text}`); return true; }
    return send(text);
  },
  (p) => writeFileSync(state, JSON.stringify([...p])),
);
console.log(JSON.stringify({ enabled: Boolean(tg || x), seen: feed.items.length, ...result }));
