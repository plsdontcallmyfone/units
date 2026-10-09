/**
 * Hookwars events (06 2.1). Self-CPI events (`emit_cpi!`) arrive as inner instructions to the
 * program's event authority: `EVENT_IX_TAG` (Anchor's `sha256("anchor:event")[..8]`), then the
 * event's discriminator, then Borsh. `hookwars_items` callbacks are leaves and emit with `emit!`
 * (R18): `Program data: <base64>` log lines, attributed to the program by the invoke and success
 * lines around them.
 *
 * Token events follow the M1 Rust (`programs/bordrless_token/src/events.rs`). Every other new
 * event follows its spec table, in table order (INTEGRATION.md section 2).
 */
import { createHash } from 'node:crypto';
import { PublicKey } from '@solana/web3.js';
import { eventCodec, type EventCodec, type Field, type Ty } from './codec.ts';
import { EQUIP_CONFIG, MANIFEST, PARAM_FIELDS, SCORE_WEIGHTS, SLOT_BOUNDS } from './accounts.ts';
import { ARMORY_ID, ITEMS_ID, LAUNCH_ID, SWAP_ID, TOKEN_ID, WAR_ID, COMPANION_ID } from './addresses.ts';

export const EVENT_IX_TAG = createHash('sha256').update('anchor:event').digest().subarray(0, 8);

const params = (): Ty => ({ array: ['u32', PARAM_FIELDS] });
const slotAmount: Ty = { struct: [['slot', 'u8'], ['item', 'pubkey'], ['amount', 'u64']] };
const slotSpec: Ty = {
  struct: [['kind', 'u8'], ['equipRule', 'u8'], ['bounds', SLOT_BOUNDS], ['noticeSecs', 'u32'], ['dataLen', 'u8'], ['launchItem', { option: 'pubkey' }], ['ruleData', { vec: 'u8' }]],
};

type Spec = [name: string, fields: Field[]];

/** Per program: the event schemas. */
export const EVENT_SPECS: Record<string, Spec[]> = {
  token: [
    ['Transferred', [
      ['mint', 'pubkey'], ['source', 'pubkey'], ['destination', 'pubkey'], ['sourceOwner', 'pubkey'], ['destinationOwner', 'pubkey'],
      ['authority', 'pubkey'], ['amount', 'u64'], ['deltas', { vec: { struct: [['holding', 'pubkey'], ['owner', 'pubkey'], ['amount', 'u64'], ['post', 'u64']] } }],
      ['sourcePost', 'u64'], ['destinationPost', 'u64'], ['slot', 'u64'], ['ts', 'i64'],
      ['slotCuts', { vec: { struct: [['slot', 'u8'], ['item', 'pubkey'], ['cut', 'u64']] } }],
    ]],
    ['SlotsInitialized', [
      ['mint', 'pubkey'], ['slotAuthority', { option: 'pubkey' }],
      ['slots', { vec: { struct: [
        ['kind', 'u8'], ['equipRule', 'u8'], ['maxCutBps', 'u16'], ['mayRefuse', 'bool'], ['mayWriteData', 'bool'], ['mayAnswerTouch', 'bool'],
        ['dataOffset', 'u8'], ['dataLen', 'u8'], ['equipVault', 'pubkey'], ['lockedProgram', 'pubkey'],
      ] } }],
    ]],
    ['SlotEquipped', [['mint', 'pubkey'], ['slot', 'u8'], ['oldItem', 'pubkey'], ['newItem', 'pubkey'], ['program', 'pubkey'], ['flags', 'u16'], ['poolFlags', 'u16'], ['dataEpoch', 'u8'], ['ts', 'i64']]],
    ['VoteLockSet', [['mint', 'pubkey'], ['holding', 'pubkey'], ['owner', 'pubkey'], ['amount', 'u64'], ['until', 'i64'], ['ts', 'i64']]],
    ['HookDataWritten', [['mint', 'pubkey'], ['holding', 'pubkey'], ['owner', 'pubkey'], ['data', { bytes: 64 }]]],
  ],
  armory: [
    ['TemplateRegistered', [['templateId', 'u16'], ['program', 'pubkey'], ['codeHash', { bytes: 32 }], ['deploySlot', { option: 'u64' }], ['kind', 'u8'], ['fieldCount', 'u8'], ['fieldMin', params()], ['fieldMax', params()], ['name', 'string'], ['ts', 'i64']]],
    ['TemplateRetired', [['templateId', 'u16'], ['ts', 'i64']]],
    ['ItemCreated', [['item', 'pubkey'], ['itemMint', 'pubkey'], ['templateId', 'u16'], ['params', params()], ['manifest', MANIFEST], ['author', 'pubkey'], ['royaltyBps', 'u16'], ['level', 'u8'], ['source', 'u8'], ['ts', 'i64']]],
    ['LootMinted', [['item', 'pubkey'], ['owner', 'pubkey'], ['templateId', 'u16'], ['params', params()], ['ts', 'i64']]],
    ['Forged', [['burned', { array: ['pubkey', 2] }], ['item', 'pubkey'], ['templateId', 'u16'], ['params', params()], ['level', 'u8'], ['forger', 'pubkey'], ['ts', 'i64']]],
    ['RoyaltyClaimed', [['item', 'pubkey'], ['cutMint', 'pubkey'], ['claimant', 'pubkey'], ['amount', 'u64'], ['ts', 'i64']]],
    ['ProposalCreated', [['mint', 'pubkey'], ['slot', 'u8'], ['nonce', 'u64'], ['proposer', 'pubkey'], ['item', { option: 'pubkey' }], ['voteEnd', 'i64'], ['executableAt', 'i64']]],
    ['VoteLocked', [['proposal', 'pubkey'], ['voter', 'pubkey'], ['support', 'bool'], ['amount', 'u64'], ['until', 'i64']]],
    ['VoteUnlocked', [['proposal', 'pubkey'], ['voter', 'pubkey'], ['amount', 'u64']]],
    ['ProposalResolved', [['proposal', 'pubkey'], ['status', 'u8'], ['votesFor', 'u64'], ['votesAgainst', 'u64'], ['eligible', 'u64'], ['ts', 'i64']]],
    ['EquipApplied', [['mint', 'pubkey'], ['slot', 'u8'], ['oldItem', 'pubkey'], ['newItem', 'pubkey'], ['by', { enum: ['Launch', 'Vote', 'Performance'] }], ['ts', 'i64']]],
    ['PerformanceCondition', [['mint', 'pubkey'], ['slot', 'u8'], ['trueSince', { option: 'i64' }]]],
    ['PerformanceReverted', [['mint', 'pubkey'], ['slot', 'u8'], ['fromItem', 'pubkey'], ['toItem', 'pubkey'], ['ts', 'i64']]],
  ],
  swap: [
    ['RouteSwapped', [['trader', 'pubkey'], ['routeInputMint', 'pubkey'], ['routeOutputMint', 'pubkey'], ['amountIn', 'u64'], ['amountOut', 'u64'], ['pools', { vec: 'pubkey' }], ['slot', 'u64'], ['ts', 'i64']]],
    ['ObservationsCreated', [['pool', 'pubkey'], ['observations', 'pubkey'], ['len', 'u16']]],
  ],
  launch: [
    ['PoolItemCuts', [['launch', 'pubkey'], ['pool', 'pubkey'], ['mint', 'pubkey'], ['side', 'u8'], ['discountBps', 'u16'], ['cuts', { vec: slotAmount }], ['burns', { vec: slotAmount }], ['poolCutsDelta', 'u64'], ['slot', 'u64'], ['ts', 'i64']]],
    ['LaunchPrepared', [['mint', 'pubkey'], ['creator', 'pubkey'], ['slots', { vec: slotSpec }]]],
    ['PoolRegistryRefreshed', [['mint', 'pubkey'], ['pool', 'pubkey'], ['items', { vec: { struct: [['slot', 'u8'], ['item', 'pubkey']] } }]]],
  ],
  companion: [
    ['CompanionWarFunded', [['companion', 'pubkey'], ['mint', 'pubkey'], ['amount', 'u64'], ['total', 'u64']]],
  ],
  items: [
    ['RaidMarked', [['mint', 'pubkey'], ['rival', 'pubkey'], ['trader', 'pubkey'], ['volume', 'u64'], ['points', 'u32'], ['lootTicket', 'bool']]],
    ['ShieldTaken', [['mint', 'pubkey'], ['owner', 'pubkey'], ['cut', 'u64']]],
    ['ItemCut', [['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['side', 'u8'], ['amount', 'u64']]],
    ['EquipSettled', [['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['royaltyToken', 'u64'], ['royaltyQuote', 'u64'], ['destination', 'pubkey'], ['amountToken', 'u64'], ['amountQuote', 'u64'], ['bounty', 'u64']]],
    ['EquipInitialized', [['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['config', EQUIP_CONFIG]]],
    ['EquipClosed', [['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['config', EQUIP_CONFIG]]],
  ],
  war: [
    ['WarChestCreated', [['mint', 'pubkey'], ['chest', 'pubkey'], ['treatyInbox', 'pubkey'], ['warState', 'pubkey']]],
    ['WarFunded', [['mint', 'pubkey'], ['amount', 'u64'], ['balance', 'u64'], ['fundedTotal', 'u64']]],
    ['SiegeExecuted', [['mint', 'pubkey'], ['rivalMint', 'pubkey'], ['spent', 'u64'], ['bought', 'u64'], ['bounty', 'u64'], ['cranker', 'pubkey'], ['capturedTotal', 'u64']]],
    ['SiegeWaited', [['mint', 'pubkey'], ['rivalMint', 'pubkey'], ['rivalPrice', 'u128'], ['rivalTwap', 'u128']]],
    ['CounterStrikeExecuted', [['mint', 'pubkey'], ['spent', 'u64'], ['burned', 'u64'], ['bounty', 'u64'], ['cranker', 'pubkey']]],
    ['Razed', [['mint', 'pubkey'], ['rivalMint', 'pubkey'], ['sold', 'u64'], ['got', 'u64'], ['bounty', 'u64'], ['cranker', 'pubkey'], ['capturedLeft', 'u64']]],
    ['CapturedReturned', [['mint', 'pubkey'], ['rivalMint', 'pubkey'], ['amount', 'u64'], ['treatyItem', 'pubkey']]],
    ['TreatyInflowShared', [['mint', 'pubkey'], ['amount', 'u64'], ['bounty', 'u64'], ['cranker', 'pubkey']]],
    ['TreatyTimeAccrued', [['mint', 'pubkey'], ['treatyItem', 'pubkey'], ['secs', 'u64'], ['season', 'u32']]],
    ['BountyClaimed', [['mint', 'pubkey'], ['owner', 'pubkey'], ['points', 'u32'], ['paid', 'u64']]],
    ['RollRequested', [['roll', 'pubkey'], ['mint', 'pubkey'], ['owner', 'pubkey'], ['holding', 'pubkey'], ['season', 'u32'], ['requestedSlot', 'u64'], ['oracleProgram', 'pubkey'], ['oracleAccount', 'pubkey']]],
    ['RollRevealed', [['roll', 'pubkey'], ['mint', 'pubkey'], ['owner', 'pubkey'], ['item', 'pubkey'], ['templateId', 'u16'], ['params', params()], ['season', 'u32']]],
    ['RollCancelled', [['roll', 'pubkey'], ['mint', 'pubkey'], ['owner', 'pubkey']]],
    ['QuestClaimed', [['questId', 'u8'], ['mint', 'pubkey'], ['owner', 'pubkey'], ['season', 'u32'], ['period', 'u32']]],
    ['SeasonProposed', [['season', 'u32'], ['startsAt', 'i64'], ['endsAt', 'i64'], ['weights', SCORE_WEIGHTS], ['penalizeBesieged', 'bool'], ['eta', 'i64']]],
    ['LootTableProposed', [['season', 'u32'], ['eta', 'i64']]],
    ['SeasonOpened', [['season', 'u32'], ['startsAt', 'i64'], ['endsAt', 'i64']]],
    ['CandidateSubmitted', [['season', 'u32'], ['mint', 'pubkey'], ['score', 'i128'], ['submittedBy', 'pubkey']]],
    ['CandidateChallenged', [['season', 'u32'], ['mint', 'pubkey'], ['score', 'i128'], ['submittedBy', 'pubkey'], ['beaten', 'pubkey']]],
    ['SeasonFinalized', [['season', 'u32'], ['winner', 'pubkey'], ['score', 'i128']]],
    ['PrizePaid', [['season', 'u32'], ['winner', 'pubkey'], ['toWinner', 'u64'], ['toTreasury', 'u64'], ['bounty', 'u64'], ['cranker', 'pubkey']]],
    ['ConfigProposed', [['change', 'u8'], ['eta', 'i64']]],
    ['ConfigApplied', [['change', 'u8'], ['eta', 'i64']]],
    ['PendingCancelled', [['change', 'u8'], ['eta', 'i64']]],
  ],
};

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
