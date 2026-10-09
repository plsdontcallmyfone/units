/**
 * Hookwars events (06 2.1). Self-CPI events (`emit_cpi!`) arrive as inner instructions to the
 * program's event authority: `EVENT_IX_TAG` (Anchor's `sha256("anchor:event")[..8]`), then the
 * event's discriminator, then Borsh. `hookwars_items` callbacks are leaves and emit with `emit!`
 * (R18): `Program data: <base64>` log lines, attributed to the program by the invoke and success
 * lines around them.
 *
 * Events of the token program, the armory, the items program, the war program and the companion
 * come from their generated IDLs (`from-idl.ts`). The DEX's and the launchpad's new events (M3a,
 * M3b) and the items callback events (M3b) are still spec tables, in table order; an IDL event of
 * the same name replaces the spec one (INTEGRATION.md section 2).
 */
import { PublicKey } from '@solana/web3.js';
import { EVENT_IX_TAG as UPSTREAM_EVENT_IX_TAG } from '../events.ts';
import { eventCodec, type EventCodec, type Field, type Ty } from './codec.ts';
import { idlEventSpecs } from './from-idl.ts';
import { ARMORY_ID, ITEMS_ID, LAUNCH_ID, SWAP_ID, TOKEN_ID, WAR_ID, COMPANION_ID } from './addresses.ts';

/** `sha256("anchor:event")[..8]` as Anchor writes it (little-endian u64), upstream's constant. */
export const EVENT_IX_TAG = UPSTREAM_EVENT_IX_TAG;

const slotAmount: Ty = { struct: [['slot', 'u8'], ['item', 'pubkey'], ['amount', 'u64']] };
const SLOT_BOUNDS: Ty = { struct: [['maxCutBps', 'u16'], ['mayRefuse', 'bool'], ['mayWriteData', 'bool'], ['mayAnswerTouch', 'bool']] };
const slotSpec: Ty = {
  struct: [['kind', 'u8'], ['equipRule', 'u8'], ['bounds', SLOT_BOUNDS], ['noticeSecs', 'u32'], ['dataLen', 'u8'], ['launchItem', { option: 'pubkey' }], ['ruleData', { vec: 'u8' }]],
};

type Spec = [name: string, fields: Field[]];

/** Spec tables for programs and events not on main yet (DEX M3a, launchpad M3b, items callbacks M3b). */
const SPEC_ONLY: Record<string, Spec[]> = {
  swap: [
    ['RouteSwapped', [['trader', 'pubkey'], ['routeInputMint', 'pubkey'], ['routeOutputMint', 'pubkey'], ['amountIn', 'u64'], ['amountOut', 'u64'], ['pools', { vec: 'pubkey' }], ['slot', 'u64'], ['ts', 'i64']]],
    ['ObservationsCreated', [['pool', 'pubkey'], ['observations', 'pubkey'], ['len', 'u16']]],
  ],
  launch: [
    ['PoolItemCuts', [['launch', 'pubkey'], ['pool', 'pubkey'], ['mint', 'pubkey'], ['side', 'u8'], ['discountBps', 'u16'], ['cuts', { vec: slotAmount }], ['burns', { vec: slotAmount }], ['poolCutsDelta', 'u64'], ['slot', 'u64'], ['ts', 'i64']]],
    ['LaunchPrepared', [['mint', 'pubkey'], ['creator', 'pubkey'], ['slots', { vec: slotSpec }]]],
    ['PoolRegistryRefreshed', [['mint', 'pubkey'], ['pool', 'pubkey'], ['items', { vec: { struct: [['slot', 'u8'], ['item', 'pubkey']] } }]]],
  ],
  items: [
    ['RaidMarked', [['mint', 'pubkey'], ['rival', 'pubkey'], ['trader', 'pubkey'], ['volume', 'u64'], ['points', 'u32'], ['lootTicket', 'bool']]],
    ['ShieldTaken', [['mint', 'pubkey'], ['owner', 'pubkey'], ['cut', 'u64']]],
    ['ItemCut', [['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['side', 'u8'], ['amount', 'u64']]],
    ['EquipSettled', [['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['royaltyToken', 'u64'], ['royaltyQuote', 'u64'], ['destination', 'pubkey'], ['amountToken', 'u64'], ['amountQuote', 'u64'], ['bounty', 'u64']]],
  ],
};

/** The programs whose events come from their IDLs. */
const IDL_PROGRAMS = ['token', 'armory', 'items', 'war', 'companion'] as const;

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

/** Item log events (R18) from a transaction's log lines: `Program data:` lines emitted while
 * `hookwars_items` is the innermost running program. */
export function itemLogEvents(logs: readonly string[], items: PublicKey = ITEMS_ID): { name: string; data: Record<string, unknown>; line: number }[] {
  const stack: string[] = [];
  const out: { name: string; data: Record<string, unknown>; line: number }[] = [];
  const id = items.toBase58();
  logs.forEach((line, i) => {
    const inv = /^Program (\w+) invoke \[\d+\]$/.exec(line);
    if (inv) { stack.push(inv[1]!); return; }
    if (/^Program (\w+) (success|failed)/.test(line)) { stack.pop(); return; }
    const data = /^Program data: (.+)$/.exec(line);
    if (data && stack[stack.length - 1] === id) {
      const ev = decodeEventBody('items', Buffer.from(data[1]!, 'base64'));
      if (ev) out.push({ ...ev, line: i });
    }
  });
  return out;
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
