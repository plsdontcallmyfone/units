// The actions a model may propose, and the strict parser for its reply. Anything that does not
// parse exactly is dropped with a reason; nothing a model writes is executed as code.
import { PublicKey } from '@solana/web3.js';
import { fromJson, isObj, type Obj } from './memo.ts';
import type { Role } from './directive.ts';

export type Action =
  | { type: 'create_item'; templateId: number; params: number[]; royaltyBps: number }
  | { type: 'list_item'; itemMint: string; priceLamports: bigint; expiresAt: bigint }
  | { type: 'offer_lease'; itemMint: string; tokenMint: string; slot: number; rentBps: number; feeLamports: bigint; termSecs: number }
  | { type: 'message'; kind: 'offer' | 'counter' | 'accept' | 'listing' | 'treaty' | 'ack'; to: string; thread: string; re: string; body: Obj; expiresAt: bigint }
  | { type: 'commit'; messageId: string; hash: string }
  | { type: 'post_bond'; proposalA: string; proposalB: string; treatyItem: string }
  | { type: 'crank'; route: string; body: Record<string, string | number | boolean> }
  | { type: 'trade'; side: 'buy' | 'sell'; mint: string; amountIn: bigint; minOut: bigint; reason: string }
  | { type: 'set_access'; itemMint: string; mode: number; licencePrice: bigint }
  | { type: 'status'; voice: string };

/** The role each action needs. */
export function roleOf(a: Action): Role {
  switch (a.type) {
    case 'create_item': return 'author';
    case 'list_item': case 'offer_lease': case 'set_access': return 'market';
    case 'message': return a.kind === 'listing' ? 'market' : 'diplomat';
    case 'commit': case 'post_bond': return 'diplomat';
    case 'crank': return 'cranker';
    case 'trade': return 'trader';
    case 'status': return 'reporter';
  }
}

export interface Parsed { actions: Action[]; dropped: { index: number; why: string }[] }

class Bad extends Error {}
const need = (c: boolean, why: string): void => { if (!c) throw new Bad(why); };
const int = (x: unknown, what: string, max: number): number => { need(typeof x === 'number' && Number.isSafeInteger(x) && x >= 0 && x <= max, `${what} must be an integer from 0 to ${max}`); return x as number; };
const u64 = (x: unknown, what: string): bigint => {
  need((typeof x === 'string' && /^(0|[1-9]\d{0,19})$/.test(x)) || (typeof x === 'number' && Number.isSafeInteger(x) && x >= 0), `${what} must be a non-negative integer (a string for large values)`);
  const v = BigInt(x as string | number);
  need(v < 1n << 64n, `${what} is above u64`);
  return v;
};
const key = (x: unknown, what: string): string => { need(typeof x === 'string', `${what} must be a base58 address`); try { return new PublicKey(x as string).toBase58(); } catch { throw new Bad(`${what} must be a base58 address`); } };
const text = (x: unknown, what: string, max: number): string => { need(typeof x === 'string' && (x as string).length <= max, `${what} must be text of at most ${max} characters`); return x as string; };

const MESSAGE_KINDS = ['offer', 'counter', 'accept', 'listing', 'treaty', 'ack'] as const;

export function parseAction(x: unknown): Action {
  need(!!x && typeof x === 'object' && !Array.isArray(x), 'an action must be an object');
  const a = x as Record<string, unknown>;
  switch (a.type) {
    case 'create_item': {
      need(Array.isArray(a.params) && (a.params as unknown[]).length <= 16, 'params must be a list of at most 16 integers');
      return { type: 'create_item', templateId: int(a.templateId, 'templateId', 0xffff), params: (a.params as unknown[]).map((p, i) => int(p, `params[${i}]`, 0xffff_ffff)), royaltyBps: int(a.royaltyBps, 'royaltyBps', 10_000) };
    }
    case 'list_item':
      return { type: 'list_item', itemMint: key(a.itemMint, 'itemMint'), priceLamports: u64(a.priceLamports, 'priceLamports'), expiresAt: u64(a.expiresAt ?? 0, 'expiresAt') };
    case 'offer_lease':
      return { type: 'offer_lease', itemMint: key(a.itemMint, 'itemMint'), tokenMint: key(a.tokenMint, 'tokenMint'), slot: int(a.slot, 'slot', 255), rentBps: int(a.rentBps, 'rentBps', 10_000), feeLamports: u64(a.feeLamports, 'feeLamports'), termSecs: int(a.termSecs, 'termSecs', 0xffff_ffff) };
    case 'message': {
      need((MESSAGE_KINDS as readonly unknown[]).includes(a.kind), `kind must be one of ${MESSAGE_KINDS.join(', ')}`);
      const to = a.to === '*' ? '*' : key(a.to, 'to');
      let body: Obj;
      try { const v = fromJson(a.body ?? {}); need(isObj(v), 'body must be an object'); body = v as Obj; } catch (e) { throw e instanceof Bad ? e : new Bad('body must be JSON with non-negative integers only'); }
      return { type: 'message', kind: a.kind as (typeof MESSAGE_KINDS)[number], to, thread: text(a.thread ?? '', 'thread', 120), re: text(a.re ?? '', 're', 120), body, expiresAt: u64(a.expiresAt ?? 0, 'expiresAt') };
    }
    case 'commit':
      need(typeof a.hash === 'string' && /^[0-9a-f]{64}$/.test(a.hash), 'hash must be 64 hex characters');
      return { type: 'commit', messageId: text(a.messageId, 'messageId', 120), hash: a.hash as string };
    case 'set_access':
      return { type: 'set_access', itemMint: key(a.itemMint, 'itemMint'), mode: int(a.mode, 'mode', 7), licencePrice: u64(a.licencePrice ?? 0, 'licencePrice') };
    case 'post_bond':
      return { type: 'post_bond', proposalA: key(a.proposalA, 'proposalA'), proposalB: key(a.proposalB, 'proposalB'), treatyItem: key(a.treatyItem, 'treatyItem') };
    case 'crank': {
      need(typeof a.route === 'string' && /^[a-z/-]+$/.test(a.route as string), 'route must be a prepare route name');
      need(!!a.body && typeof a.body === 'object' && !Array.isArray(a.body), 'body must be an object');
      const body: Record<string, string | number | boolean> = {};
      for (const [k, v] of Object.entries(a.body as Record<string, unknown>)) {
        need(['string', 'number', 'boolean'].includes(typeof v), `body.${k} must be a string, number or boolean`);
        body[k] = v as string | number | boolean;
      }
      return { type: 'crank', route: a.route as string, body };
    }
    case 'trade':
      need(a.side === 'buy' || a.side === 'sell', 'side must be buy or sell');
      return { type: 'trade', side: a.side as 'buy' | 'sell', mint: key(a.mint, 'mint'), amountIn: u64(a.amountIn, 'amountIn'), minOut: u64(a.minOut, 'minOut'), reason: text(a.reason, 'reason', 140) };
    case 'status':
      return { type: 'status', voice: text(a.voice ?? '', 'voice', 80) };
    default:
      throw new Bad(`unknown action type ${JSON.stringify(a.type)}`);
  }
}

/** Parses a model reply: one JSON object `{"actions":[...]}`, optionally inside a fenced block. */
export function parseReply(textIn: string, maxActions: number): Parsed {
  const dropped: Parsed['dropped'] = [];
  const fenced = /```(?:json)?\s*([\s\S]*?)```/.exec(textIn);
  const raw = (fenced ? fenced[1]! : textIn).trim();
  let j: unknown;
  try { j = JSON.parse(raw); } catch { return { actions: [], dropped: [{ index: -1, why: 'the reply is not one JSON object' }] }; }
  if (!j || typeof j !== 'object' || !Array.isArray((j as { actions?: unknown }).actions)) return { actions: [], dropped: [{ index: -1, why: 'the reply has no "actions" list' }] };
  const list = (j as { actions: unknown[] }).actions;
  const actions: Action[] = [];
  list.forEach((x, i) => {
    if (actions.length >= maxActions) { dropped.push({ index: i, why: `more than ${maxActions} actions in one tick` }); return; }
    try { actions.push(parseAction(x)); } catch (e) { dropped.push({ index: i, why: e instanceof Bad ? e.message : 'unreadable action' }); }
  });
  return { actions, dropped };
}

/** The JSON shapes the prompt shows the model. */
export const ACTION_GUIDE = `Reply with one JSON object and nothing else: {"actions":[ ... ]}. Each action is one of:
{"type":"create_item","templateId":<id>,"params":[<int>...],"royaltyBps":<0..10000>}            role author
{"type":"list_item","itemMint":"<base58>","priceLamports":"<int>","expiresAt":"<unix or 0>"}       role market
{"type":"offer_lease","itemMint":"<base58>","tokenMint":"<base58>","slot":<n>,"rentBps":<n>,"feeLamports":"<int>","termSecs":<n>}  role market
{"type":"set_access","itemMint":"<base58>","mode":<0..7>,"licencePrice":"<int>"}                  role market (refused until the armory has set_access)
{"type":"message","kind":"offer|counter|accept|listing|treaty|ack","to":"<passport or *>","thread":"<message id or empty>","re":"<message id or empty>","body":{...},"expiresAt":"<unix or 0>"}  role diplomat (listing: market)
{"type":"commit","messageId":"<sig:index>","hash":"<64 hex>"}                                        role diplomat
{"type":"post_bond","proposalA":"<base58>","proposalB":"<base58>","treatyItem":"<base58>"}          role diplomat
{"type":"crank","route":"<allowed route>","body":{...}}                                             role cranker
{"type":"trade","side":"buy|sell","mint":"<base58>","amountIn":"<int: lamports to buy with, or base units to sell>","minOut":"<int>","reason":"<public reason>"}  role trader
{"type":"status","voice":"<a few words in your voice, no numbers>"}                                 role reporter
Use only your roles. Propose nothing when nothing is worth doing: {"actions":[]}.`;
