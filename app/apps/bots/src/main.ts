/**
 * Polls the API's /v1/feed and posts new events once each, keyed by (signature, ordinal) in a local
 * state file, so a replay of the same range posts nothing twice (06 section 8). Disabled unless TELEGRAM_BOT_TOKEN + TELEGRAM_CHAT_ID or
 * X_BEARER_TOKEN are set; without them it logs what it would post and exits.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import type { BattleEvent, Page } from '@hookwars/shared';
import { postFor } from './posts.ts';

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

async function send(text: string): Promise<void> {
  if (tg) await fetch(`https://api.telegram.org/bot${tg.token}/sendMessage`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ chat_id: tg.chat, text, disable_web_page_preview: true }) });
  if (x) await fetch('https://api.twitter.com/2/tweets', { method: 'POST', headers: { authorization: `Bearer ${x}`, 'content-type': 'application/json' }, body: JSON.stringify({ text }) });
}

const r = await fetch(`${api}/v1/feed`);
const feed = (await r.json()) as Page<BattleEvent>;
const fresh = feed.items.filter((e) => !posted.has(`${e.signature}:${e.ordinal}`)).reverse();
for (const e of fresh) {
  const text = postFor(e, cfg);
  posted.add(`${e.signature}:${e.ordinal}`);
  if (!text) continue;
  if (!tg && !x) console.log(`[disabled, would post] ${text}`);
  else await send(text);
}
writeFileSync(state, JSON.stringify([...posted]));
console.log(JSON.stringify({ enabled: Boolean(tg || x), seen: feed.items.length, new: fresh.length }));
