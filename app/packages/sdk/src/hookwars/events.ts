// Changed by Hookwars: every program's events from its IDL; log events for items and the launchpad; economy events not yet in the IDLs.
/**
 * Hookwars events (06 2.1). Self-CPI events (`emit_cpi!`) arrive as inner instructions to the
 * program's event authority: `EVENT_IX_TAG` (Anchor's `sha256("anchor:event")[..8]`), then the
 * event's discriminator, then Borsh. `hookwars_items` callbacks are leaves and emit with `emit!`
 * (R18): `Program data: <base64>` log lines, attributed to the program by the invoke and success
 * lines around them.
 *
 * Every program's events come from its generated IDL (`from-idl.ts`): token, armory, items, war,
 * companion, launchpad, DEX, agents, market and social.
 */
import { PublicKey } from '@solana/web3.js';
import { EVENT_IX_TAG as UPSTREAM_EVENT_IX_TAG } from '../events.ts';
import { eventCodec, type EventCodec, type Field } from './codec.ts';
import { idlEventSpecs } from './from-idl.ts';
import { AGENTS_ID, ARMORY_ID, ITEMS_ID, LAUNCH_ID, MARKET_ID, SOCIAL_ID, SWAP_ID, TOKEN_ID, WAR_ID, COMPANION_ID } from './addresses.ts';

/** `sha256("anchor:event")[..8]` as Anchor writes it (little-endian u64), upstream's constant. */
export const EVENT_IX_TAG = UPSTREAM_EVENT_IX_TAG;

type Spec = [name: string, fields: Field[]];

/** Spec tables for events not in any IDL yet. The app IDLs predate the economy merge (11): its
 * directive, postage and profile events are written from the program source until the IDLs are
 * regenerated (an IDL event of the same name wins). */
const SPEC_ONLY: Record<string, Spec[]> = {
  agents: [
    ['DirectiveSet', [['passport', 'pubkey'], ['seq', 'u32'], ['memoHash', { bytes: 32 }], ['ts', 'i64']]],
    ['Committed', [['passport', 'pubkey'], ['reference', { bytes: 32 }], ['hash', { bytes: 32 }], ['ts', 'i64']]],
    ['MessagePosted', [['passport', 'pubkey'], ['reference', { bytes: 32 }], ['postage', 'u64'], ['ts', 'i64']]],
  ],
  social: [
    ['ProfileOpened', [['wallet', 'pubkey'], ['ts', 'i64']]],
    ['WalletRecorded', [['wallet', 'pubkey'], ['callerProgram', 'pubkey'], ['counter', 'u8'], ['value', 'u64'], ['total', 'u64'], ['ts', 'i64']]],
  ],
};

/** The programs whose events come from their IDLs. */
const IDL_PROGRAMS = ['token', 'armory', 'items', 'war', 'companion', 'launch', 'swap', 'agents', 'market', 'social'] as const;

function buildSpecs(): Record<string, Spec[]> {
  const out: Record<string, Spec[]> = {};
  for (const p of IDL_PROGRAMS) out[p] = idlEventSpecs(p);
  for (const [p, specs] of Object.entries(SPEC_ONLY)) {
    const have = new Set((out[p] ?? []).map(([n]) => n));
    out[p] = [...(out[p] ?? []), ...specs.filter(([n]) => !have.has(n))];
  }
  return out;
}

/** Per program: the event schemas (IDL first, then the remaining spec tables). */
export const EVENT_SPECS: Record<string, Spec[]> = buildSpecs();

/** Which events of `EVENT_SPECS` are still spec layouts (for INTEGRATION.md and the indexer). */
export const SPEC_EVENTS: Record<string, string[]> = Object.fromEntries(
  Object.entries(SPEC_ONLY).map(([p, specs]) => [p, specs.map(([n]) => n).filter((n) => !(IDL_PROGRAMS as readonly string[]).includes(p) || !idlEventSpecs(p).some(([m]) => m === n))]),
);

export const PROGRAM_OF: Record<string, PublicKey> = {
  token: TOKEN_ID, armory: ARMORY_ID, swap: SWAP_ID, launch: LAUNCH_ID, companion: COMPANION_ID, items: ITEMS_ID, war: WAR_ID,
  agents: AGENTS_ID, market: MARKET_ID, social: SOCIAL_ID,
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

/** Programs that emit some events with `emit!` (program logs): `hookwars_items` callbacks (R18)
 * and the launchpad's `PoolItemCuts` (03 M3b notes). */
export const LOG_EVENT_PROGRAMS = ['items', 'launch'] as const;

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
