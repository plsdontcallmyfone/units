// Changed by Hookwars: every program's events from its IDL (craft and book too); log events for items and the launchpad.
/**
 * Hookwars events (06 2.1). Self-CPI events (`emit_cpi!`) arrive as inner instructions to the
 * program's event authority: `EVENT_IX_TAG` (Anchor's `sha256("anchor:event")[..8]`), then the
 * event's discriminator, then Borsh. `hookwars_items` callbacks are leaves and emit with `emit!`
 * (R18): `Program data: <base64>` log lines, attributed to the program by the invoke and success
 * lines around them.
 *
 * Every program's events come from its generated IDL (`from-idl.ts`): token, armory, items, war,
 * companion, launchpad, DEX, agents, market, social, craft and book.
 */
import { PublicKey } from '@solana/web3.js';
import { EVENT_IX_TAG as UPSTREAM_EVENT_IX_TAG } from '../events.ts';
import { eventCodec, type EventCodec, type Field } from './codec.ts';
import { idlEventSpecs } from './from-idl.ts';
import { AGENTS_ID, ARMORY_ID, BOOK_ID, CRAFT_ID, ITEMS_ID, LAUNCH_ID, MARKET_ID, SOCIAL_ID, SWAP_ID, TOKEN_ID, WAR_ID, COMPANION_ID } from './addresses.ts';

/** `sha256("anchor:event")[..8]` as Anchor writes it (little-endian u64), upstream's constant. */
export const EVENT_IX_TAG = UPSTREAM_EVENT_IX_TAG;

type Spec = [name: string, fields: Field[]];

/** The programs whose events come from their IDLs (every program the app reads; the economy
 * events that were spec tables before are in the regenerated agents and social IDLs). */
const IDL_PROGRAMS = ['token', 'armory', 'items', 'war', 'companion', 'launch', 'swap', 'agents', 'market', 'social', 'craft', 'book'] as const;

/** Per program: the event schemas, from the IDLs. */
export const EVENT_SPECS: Record<string, Spec[]> = Object.fromEntries(IDL_PROGRAMS.map((p) => [p, idlEventSpecs(p)]));

/** Events still written as spec layouts (none: every event comes from an IDL). */
export const SPEC_EVENTS: Record<string, string[]> = {};

export const PROGRAM_OF: Record<string, PublicKey> = {
  token: TOKEN_ID, armory: ARMORY_ID, swap: SWAP_ID, launch: LAUNCH_ID, companion: COMPANION_ID, items: ITEMS_ID, war: WAR_ID,
  agents: AGENTS_ID, market: MARKET_ID, social: SOCIAL_ID, craft: CRAFT_ID, book: BOOK_ID,
};

export interface HookwarsEvent { program: string; name: string; data: Record<string, unknown>; ordinal: number; via: 'cpi' | 'log' }

let codecs: Map<string, { program: string; codec: EventCodec<Record<string, unknown>> }> | null = null;
function allCodecs() {
  if (codecs) return codecs;
  codecs = new Map();
  for (const [program, specs] of Object.entries(EVENT_SPECS)) {
    for (const [name, fields] of specs) {
      const c = eventCodec<Record<string, unknown>>(name, fields);
      codecs.set(`${program}:${c.disc.toString('hex')}`, { program, codec: c });
    }
  }
  return codecs;
}

/** Decodes one event body (discriminator first) for a program, or null when it is not one of ours. */
export function decodeEventBody(program: string, body: Buffer): { name: string; data: Record<string, unknown> } | null {
  const key = `${program}:${body.subarray(0, 8).toString('hex')}`;
  const hit = allCodecs().get(key);
  if (!hit) return null;
  return { name: hit.codec.name, data: hit.codec.decode(body) };
}

/** Encodes an event body (for tests and fixtures). */
export function encodeEventBody(program: string, name: string, data: Record<string, unknown>): Buffer {
  const fields = EVENT_SPECS[program]?.find(([n]) => n === name)?.[1];
  if (!fields) throw new Error(`unknown event ${program}:${name}`);
  return eventCodec<Record<string, unknown>>(name, fields).encode(data);
}

/** Programs that emit some events with `emit!` (program logs): `hookwars_items` callbacks (R18),
 * the launchpad's `PoolItemCuts` (03 M3b notes), and the book's fills, evictions and expiries and
 * the `ProtocolFee` events of book, craft and licences (review 3 I-5). */
export const LOG_EVENT_PROGRAMS = ['items', 'launch', 'book', 'craft', 'market'] as const;

/** Log events from a transaction's log lines: `Program data:` lines emitted while one of
 * `programs` is the innermost running program, attributed to it. */
export function programLogEvents(logs: readonly string[], programs: readonly string[] = LOG_EVENT_PROGRAMS): { program: string; name: string; data: Record<string, unknown>; line: number }[] {
  const ids = new Map(programs.map((p) => [PROGRAM_OF[p]!.toBase58(), p]));
  const stack: string[] = [];
  const out: { program: string; name: string; data: Record<string, unknown>; line: number }[] = [];
  logs.forEach((line, i) => {
    const inv = /^Program (\w+) invoke \[\d+\]$/.exec(line);
    if (inv) { stack.push(inv[1]!); return; }
    if (/^Program (\w+) (success|failed)/.test(line)) { stack.pop(); return; }
    const data = /^Program data: (.+)$/.exec(line);
    const program = data ? ids.get(stack[stack.length - 1] ?? '') : undefined;
    if (data && program) {
      const ev = decodeEventBody(program, Buffer.from(data[1]!, 'base64'));
      if (ev) out.push({ program, ...ev, line: i });
    }
  });
  return out;
}

/** Item log events (R18); `programLogEvents` restricted to `hookwars_items`. */
export function itemLogEvents(logs: readonly string[], items: PublicKey = ITEMS_ID): { name: string; data: Record<string, unknown>; line: number }[] {
  void items;
  return programLogEvents(logs, ['items']).map(({ name, data, line }) => ({ name, data, line }));
}

/** True when the runtime truncated the logs (06 2.1: item log events are then hints only). */
export function logsTruncated(logs: readonly string[]): boolean {
  return logs.some((l) => l === 'Log truncated' || l.startsWith('Log truncated'));
}

/** Programs by base58 id, for attributing self-CPI events. */
export function programNameOf(id: string): string | null {
  for (const [name, key] of Object.entries(PROGRAM_OF)) if (key.toBase58() === id) return name;
  return null;
}

/** Decodes a self-CPI event instruction's data, given the program that emitted it. */
export function decodeCpiEvent(program: string, ixData: Buffer): { name: string; data: Record<string, unknown> } | null {
  if (ixData.length < 16 || !ixData.subarray(0, 8).equals(EVENT_IX_TAG)) return null;
  return decodeEventBody(program, ixData.subarray(8));
}
