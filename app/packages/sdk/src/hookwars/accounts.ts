/**
 * Account decoders for Hookwars.
 *
 * - `Mint` with the slot table and `Holding` with the vote lock follow the M1 Rust code
 *   (`programs/bordrless_token/src/state.rs`, branch main at M1), so they are exact.
 * - Every other account follows the field tables of its spec section, in table order, because its
 *   program is being built on another branch. Each codec says which section it follows and is listed
 *   in INTEGRATION.md section 2 to be replaced by the generated IDL coder (or checked against it).
 * - Arrays whose length is a parameter still "to measure" (`RAID_TABLE_LEN`, `MAX_CAPTURED`,
 *   `LOOT_TABLE_LEN`, `OBS_RING_LEN`) are sized from the account's data length.
 */
import { PublicKey } from '@solana/web3.js';
import { PARAM_FIELDS_MIN } from '@hookwars/shared';
import { accountCodec, decode, discriminator, fixedSize, Reader, type Field, type Ty } from './codec.ts';

/** PARAM_FIELDS (00 section 6). The smallest value the spec allows until the armory build fixes it
 * (INTEGRATION.md A1). */
export let PARAM_FIELDS = PARAM_FIELDS_MIN;
export function setParamFields(n: number): void { PARAM_FIELDS = n; }

/** Token program `MAX_SLOTS` (constants.rs, M1 build constant). */
export const MAX_SLOTS = 4;

// ---------------------------------------------------------------- token (01, M1 code) --

export const SLOT_BOUNDS: Ty = { struct: [['maxCutBps', 'u16'], ['mayRefuse', 'bool'], ['mayWriteData', 'bool'], ['mayAnswerTouch', 'bool']] };

export const SLOT: Ty = {
  struct: [
    ['kind', 'u8'], ['equipRule', 'u8'], ['bounds', SLOT_BOUNDS], ['dataOffset', 'u8'], ['dataLen', 'u8'],
    ['item', 'pubkey'], ['program', 'pubkey'], ['flags', 'u16'], ['poolFlags', 'u16'], ['equipVault', 'pubkey'],
    ['signerBump', 'u8'], ['launchSignerBump', 'u8'], ['dataEpoch', 'u8'], ['extraCount', 'u8'],
  ],
};

export interface SlotBoundsData { maxCutBps: number; mayRefuse: boolean; mayWriteData: boolean; mayAnswerTouch: boolean }
export interface SlotData {
  kind: number; equipRule: number; bounds: SlotBoundsData; dataOffset: number; dataLen: number;
  item: PublicKey; program: PublicKey; flags: number; poolFlags: number; equipVault: PublicKey;
  signerBump: number; launchSignerBump: number; dataEpoch: number; extraCount: number;
}

export interface SlotMintData {
  version: number; decimals: number; supply: bigint; maxSupply: bigint;
  mintAuthority: PublicKey | null; freezeAuthority: PublicKey | null; hookAuthority: PublicKey | null; metadataAuthority: PublicKey | null;
  hookProgram: PublicKey | null; hookFlags: number; name: string; symbol: string; uri: string;
  createdAt: bigint; creator: PublicKey; hookSignerBump: number; reserved: Buffer;
  slotAuthority: PublicKey | null; slotCount: number; slots: SlotData[];
}

export const MINT_FIELDS: Field[] = [
  ['version', 'u8'], ['decimals', 'u8'], ['supply', 'u64'], ['maxSupply', 'u64'],
  ['mintAuthority', { option: 'pubkey' }], ['freezeAuthority', { option: 'pubkey' }], ['hookAuthority', { option: 'pubkey' }], ['metadataAuthority', { option: 'pubkey' }],
  ['hookProgram', { option: 'pubkey' }], ['hookFlags', 'u16'], ['name', 'string'], ['symbol', 'string'], ['uri', 'string'],
  ['createdAt', 'i64'], ['creator', 'pubkey'], ['hookSignerBump', 'u8'], ['reserved', { bytes: 31 }],
  ['slotAuthority', { option: 'pubkey' }], ['slotCount', 'u8'], ['slots', { array: [SLOT, MAX_SLOTS] }],
];

const MINT_DISC = discriminator('account', 'Mint');

/** Decodes a token `Mint`. Mints written before the slot table (upstream layout) decode with
 * `slotCount 0` and no slots: Anchor allocates the full account, so the tail is zeros. */
export function decodeSlotMint(data: Buffer): SlotMintData {
  if (!data.subarray(0, 8).equals(MINT_DISC)) throw new Error('not a Mint account');
  return decode<SlotMintData>({ struct: MINT_FIELDS }, data, 8);
}

/** The slots a mint uses (`slots[..slot_count]`). */
export function activeSlots(m: SlotMintData): SlotData[] {
  return m.slots.slice(0, Math.min(m.slotCount, MAX_SLOTS));
}

export interface SlotHoldingData {
  version: number; bump: number; mint: PublicKey; owner: PublicKey; amount: bigint;
  delegate: PublicKey | null; delegatedAmount: bigint; frozen: boolean; hookData: Buffer;
  voteLocked: bigint; voteLockUntil: bigint;
}
export const holdingCodec = accountCodec<SlotHoldingData>('Holding', [
  ['version', 'u8'], ['bump', 'u8'], ['mint', 'pubkey'], ['owner', 'pubkey'], ['amount', 'u64'],
  ['delegate', { option: 'pubkey' }], ['delegatedAmount', 'u64'], ['frozen', 'bool'], ['hookData', { bytes: 64 }],
  ['voteLocked', 'u64'], ['voteLockUntil', 'i64'],
]);
/** The amount a vote lock holds at `now` (01 4.3: Holding::locked_at). */
export function lockedAt(h: SlotHoldingData, now: bigint): bigint {
  return now < h.voteLockUntil ? h.voteLocked : 0n;
}

// ---------------------------------------------------------------- armory (02 section 2) --

export const MANIFEST: Ty = {
  struct: [
    ['kind', 'u8'], ['tokenFlags', 'u16'], ['poolFlags', 'u8'], ['maxCutBuyBps', 'u16'], ['maxCutSellBps', 'u16'],
    ['maxCutTransferBps', 'u16'], ['maxDiscountBps', 'u16'], ['mayRefuse', 'bool'], ['mayBurn', 'bool'],
    ['dataBytes', 'u8'], ['readsOtherPools', 'u8'],
  ],
};
export interface ManifestData {
  kind: number; tokenFlags: number; poolFlags: number; maxCutBuyBps: number; maxCutSellBps: number;
  maxCutTransferBps: number; maxDiscountBps: number; mayRefuse: boolean; mayBurn: boolean; dataBytes: number; readsOtherPools: number;
}

export interface ArmoryConfigData { version: number; bump: number; admin: PublicKey; pendingAdmin: PublicKey | null; itemsMinted: bigint; templates: number }
/** 02 2.1. */
export const armoryConfigCodec = accountCodec<ArmoryConfigData>('ArmoryConfig', [
  ['version', 'u8'], ['bump', 'u8'], ['admin', 'pubkey'], ['pendingAdmin', { option: 'pubkey' }], ['itemsMinted', 'u64'], ['templates', 'u16'],
]);

export interface TemplateData {
  version: number; bump: number; id: number; program: PublicKey; codeHash: Buffer; deploySlot: bigint | null;
  kind: number; fieldCount: number; fieldMin: number[]; fieldMax: number[]; openAuthoring: boolean; lootEnabled: boolean;
  forgeEnabled: boolean; maxLevel: number; lootRoyaltyBps: number; status: number; name: string; registeredBy: PublicKey; createdAt: bigint;
}
/** 02 2.3. */
export function templateCodec(F = PARAM_FIELDS) {
  return accountCodec<TemplateData>('Template', [
    ['version', 'u8'], ['bump', 'u8'], ['id', 'u16'], ['program', 'pubkey'], ['codeHash', { bytes: 32 }], ['deploySlot', { option: 'u64' }],
    ['kind', 'u8'], ['fieldCount', 'u8'], ['fieldMin', { array: ['u32', F] }], ['fieldMax', { array: ['u32', F] }],
    ['openAuthoring', 'bool'], ['lootEnabled', 'bool'], ['forgeEnabled', 'bool'], ['maxLevel', 'u8'], ['lootRoyaltyBps', 'u16'],
    ['status', 'u8'], ['name', 'string'], ['registeredBy', 'pubkey'], ['createdAt', 'i64'],
  ]);
}

export interface ItemData {
  version: number; bump: number; itemMint: PublicKey; templateId: number; params: number[]; manifest: ManifestData;
  author: PublicKey; royaltyBps: number; level: number; source: number; equippedCount: number; royaltyOwnerBump: number; createdAt: bigint;
}
/** 02 2.4. */
export function itemCodec(F = PARAM_FIELDS) {
  return accountCodec<ItemData>('Item', [
    ['version', 'u8'], ['bump', 'u8'], ['itemMint', 'pubkey'], ['templateId', 'u16'], ['params', { array: ['u32', F] }],
    ['manifest', MANIFEST], ['author', 'pubkey'], ['royaltyBps', 'u16'], ['level', 'u8'], ['source', 'u8'],
    ['equippedCount', 'u32'], ['royaltyOwnerBump', 'u8'], ['createdAt', 'i64'],
  ]);
}

export const PERFORMANCE_RULE: Ty = {
  struct: [['metric', 'u8'], ['windowSecs', 'u32'], ['baseWindowSecs', 'u32'], ['op', 'u8'], ['ratioBps', 'u16'], ['holdSecs', 'u32']],
};
export interface PerformanceRuleData { metric: number; windowSecs: number; baseWindowSecs: number; op: number; ratioBps: number; holdSecs: number }

export interface SlotStateData {
  version: number; bump: number; mint: PublicKey; slot: number; launchItem: PublicKey | null; rule: PerformanceRuleData | null;
  openProposal: bigint | null; nextNonce: bigint; conditionSince: bigint | null; lastCheck: bigint;
}
/** 02 2.7. */
export const slotStateCodec = accountCodec<SlotStateData>('SlotState', [
  ['version', 'u8'], ['bump', 'u8'], ['mint', 'pubkey'], ['slot', 'u8'], ['launchItem', { option: 'pubkey' }],
  ['rule', { option: PERFORMANCE_RULE }], ['openProposal', { option: 'u64' }], ['nextNonce', 'u64'],
  ['conditionSince', { option: 'i64' }], ['lastCheck', 'i64'],
]);

export interface ProposalData {
  version: number; bump: number; mint: PublicKey; slot: number; nonce: bigint; proposer: PublicKey; item: PublicKey | null;
  createdAt: bigint; voteEnd: bigint; executableAt: bigint; votesFor: bigint; votesAgainst: bigint; status: number;
}
/** 02 2.8. */
export const proposalCodec = accountCodec<ProposalData>('Proposal', [
  ['version', 'u8'], ['bump', 'u8'], ['mint', 'pubkey'], ['slot', 'u8'], ['nonce', 'u64'], ['proposer', 'pubkey'],
  ['item', { option: 'pubkey' }], ['createdAt', 'i64'], ['voteEnd', 'i64'], ['executableAt', 'i64'],
  ['votesFor', 'u64'], ['votesAgainst', 'u64'], ['status', 'u8'],
]);

export interface VoteLockData { proposal: PublicKey; voter: PublicKey; amount: bigint; support: boolean; bump: number }
/** 02 2.9. */
export const voteLockCodec = accountCodec<VoteLockData>('VoteLock', [
  ['proposal', 'pubkey'], ['voter', 'pubkey'], ['amount', 'u64'], ['support', 'bool'], ['bump', 'u8'],
]);

export interface ForgeCounterData { wallet: PublicKey; count: bigint; bump: number }
/** 02 2.10. */
export const forgeCounterCodec = accountCodec<ForgeCounterData>('ForgeCounter', [['wallet', 'pubkey'], ['count', 'u64'], ['bump', 'u8']]);

// ---------------------------------------------------------------- DEX (03 3.1) --

export interface ObservationData { ts: bigint; priceCumulative: bigint; quoteVolume: bigint; swapCount: bigint }
export interface ObservationsData {
  version: number; bump: number; pool: PublicKey; lastPriceQ64: bigint; lastTs: bigint; index: number; filled: number; entries: ObservationData[];
}
const OBS_HEADER: Field[] = [['version', 'u8'], ['bump', 'u8'], ['pool', 'pubkey'], ['lastPriceQ64', 'u128'], ['lastTs', 'i64'], ['index', 'u16'], ['filled', 'u16']];
const OBS_ENTRY: Ty = { struct: [['ts', 'i64'], ['priceCumulative', 'u128'], ['quoteVolume', 'u128'], ['swapCount', 'u64']] };
const OBS_DISC = discriminator('account', 'Observations');

/** 03 3.1: header then `OBS_RING_LEN` entries of 48 bytes; the ring length is read from the data. */
export function decodeObservations(data: Buffer): ObservationsData {
  if (!data.subarray(0, 8).equals(OBS_DISC)) throw new Error('not an Observations account');
  const r = new Reader(data); r.off = 8;
  const h = r.read({ struct: OBS_HEADER }) as Omit<ObservationsData, 'entries'>;
  const size = fixedSize(OBS_ENTRY);
  const n = Math.floor((data.length - r.off) / size);
  const entries: ObservationData[] = [];
  for (let i = 0; i < n; i++) entries.push(r.read(OBS_ENTRY) as ObservationData);
  return { ...h, entries };
}

// ---------------------------------------------------------------- items (04 sections 2.3, 2.9, 4) --

export const EQUIP_CONFIG: Ty = { struct: [['targets', { vec: 'pubkey' }], ['role', 'u8']] };
export interface EquipConfigData { targets: PublicKey[]; role: number }

export interface EquipStateData {
  mint: PublicKey; slot: number; item: PublicKey; templateId: number; config: EquipConfigData; equippedAt: bigint;
  runs: bigint; collectedToken: bigint; poolOwed: bigint; poolSettled: bigint; bump: number;
}
/** 04 section 4 (field order of its table). */
export const equipStateCodec = accountCodec<EquipStateData>('EquipState', [
  ['mint', 'pubkey'], ['slot', 'u8'], ['item', 'pubkey'], ['templateId', 'u16'], ['config', EQUIP_CONFIG], ['equippedAt', 'i64'],
  ['runs', 'u64'], ['collectedToken', 'u64'], ['poolOwed', 'u64'], ['poolSettled', 'u64'], ['bump', 'u8'],
]);

export interface RaidWindowData { rivalMint: PublicKey; windowStart: bigint; volume: bigint; prevVolume: bigint }
export interface MarkData { clockSlot: bigint; recipient: PublicKey; rival: PublicKey; quoteVolume: bigint; stampedSlots: number }
export interface RaidLedgerData {
  version: number; bump: number; mint: PublicKey; seasonId: number; outboundVolumeSeason: bigint; inbound: RaidWindowData[]; mark: MarkData;
}
const RAID_WINDOW: Ty = { struct: [['rivalMint', 'pubkey'], ['windowStart', 'i64'], ['volume', 'u64'], ['prevVolume', 'u64']] };
const MARK: Ty = { struct: [['clockSlot', 'u64'], ['recipient', 'pubkey'], ['rival', 'pubkey'], ['quoteVolume', 'u64'], ['stampedSlots', 'u8']] };
const RAID_LEDGER_DISC = discriminator('account', 'RaidLedger');

/** 04 2.9; `RAID_TABLE_LEN` is read from the data length (head, windows, mark, 32 reserved). */
export function decodeRaidLedger(data: Buffer): RaidLedgerData {
  if (!data.subarray(0, 8).equals(RAID_LEDGER_DISC)) throw new Error('not a RaidLedger account');
  const head = 1 + 1 + 32 + 4 + 8;
  const n = Math.floor((data.length - 8 - head - fixedSize(MARK) - 32) / fixedSize(RAID_WINDOW));
  return decode<RaidLedgerData>({ struct: [
    ['version', 'u8'], ['bump', 'u8'], ['mint', 'pubkey'], ['seasonId', 'u32'], ['outboundVolumeSeason', 'u64'],
    ['inbound', { array: [RAID_WINDOW, Math.max(0, n)] }], ['mark', MARK],
  ] }, data, 8);
}

// ---------------------------------------------------------------- war (05 section 2) --

export interface WarConfigData {
  version: number; bump: number; prizeVaultBump: number; admin: PublicKey; protocolTreasury: PublicKey; randomnessProgram: PublicKey;
  currentSeason: number; lastWinner: PublicKey | null;
}
/** 05 2.1, up to `last_winner` (`PendingConfig`'s layout is not fixed by the spec; INTEGRATION.md). */
export const warConfigCodec = accountCodec<WarConfigData>('WarConfig', [
  ['version', 'u8'], ['bump', 'u8'], ['prizeVaultBump', 'u8'], ['admin', 'pubkey'], ['protocolTreasury', 'pubkey'],
  ['randomnessProgram', 'pubkey'], ['currentSeason', 'u32'], ['lastWinner', { option: 'pubkey' }],
]);

export const SEASON_COUNTERS: Ty = {
  struct: [['raidVolumeWon', 'u64'], ['sieges', 'u32'], ['siegeSpend', 'u64'], ['timesBesieged', 'u32'], ['counterStrikes', 'u32'], ['treatySecs', 'u64']],
};
export interface SeasonCountersData { raidVolumeWon: bigint; sieges: number; siegeSpend: bigint; timesBesieged: number; counterStrikes: number; treatySecs: bigint }
const CAPTURED: Ty = {
  struct: [['rivalMint', 'pubkey'], ['amount', 'u64'], ['cost', 'u64'], ['capturedAt', 'i64'], ['razeWindowStart', 'i64'], ['razeWindowBase', 'u64'], ['razedInWindow', 'u64']],
};
export interface CapturedData { rivalMint: PublicKey; amount: bigint; cost: bigint; capturedAt: bigint; razeWindowStart: bigint; razeWindowBase: bigint; razedInWindow: bigint }
export interface WarStateData {
  version: number; bump: number; chestBump: number; inboxBump: number; mint: PublicKey; launch: PublicKey;
  lastSeenBalance: bigint; fundedTotal: bigint; spentSiege: bigint; spentCounter: bigint; paidBounties: bigint; paidCranks: bigint; razedProceeds: bigint;
  treatySharedTotal: bigint; lastSiegeAt: bigint; lastCounterAt: bigint; lastTreatyTick: bigint; underSiegeUntil: bigint; siegeByChest: PublicKey;
  captured: CapturedData[]; seasonId: number; season: SeasonCountersData; prevSeason: SeasonCountersData;
}
const WAR_STATE_DISC = discriminator('account', 'WarState');
const WAR_STATE_HEAD: Field[] = [
  ['version', 'u8'], ['bump', 'u8'], ['chestBump', 'u8'], ['inboxBump', 'u8'], ['mint', 'pubkey'], ['launch', 'pubkey'],
  ['lastSeenBalance', 'u64'], ['fundedTotal', 'u64'], ['spentSiege', 'u64'], ['spentCounter', 'u64'], ['paidBounties', 'u64'],
  ['paidCranks', 'u64'], ['razedProceeds', 'u64'], ['treatySharedTotal', 'u64'], ['lastSiegeAt', 'i64'], ['lastCounterAt', 'i64'],
  ['lastTreatyTick', 'i64'], ['underSiegeUntil', 'i64'], ['siegeByChest', 'pubkey'],
];
/** 05 2.4; `MAX_CAPTURED` read from the data length (head, entries, season id, two counters, 64 reserved). */
export function decodeWarState(data: Buffer): WarStateData {
  if (!data.subarray(0, 8).equals(WAR_STATE_DISC)) throw new Error('not a WarState account');
  const head = fixedSize({ struct: WAR_STATE_HEAD });
  const tail = 4 + 2 * fixedSize(SEASON_COUNTERS) + 64;
  const n = Math.floor((data.length - 8 - head - tail) / fixedSize(CAPTURED));
  return decode<WarStateData>({ struct: [
    ...WAR_STATE_HEAD, ['captured', { array: [CAPTURED, Math.max(0, n)] }], ['seasonId', 'u32'], ['season', SEASON_COUNTERS], ['prevSeason', SEASON_COUNTERS],
  ] }, data, 8);
}

export const SCORE_WEIGHTS: Ty = {
  struct: [['raidVolumeWon', 'u64'], ['sieges', 'u64'], ['siegeSpend', 'u64'], ['timesBesieged', 'u64'], ['counterStrikes', 'u64'], ['treatySecs', 'u64']],
};
export interface ScoreWeightsData { raidVolumeWon: bigint; sieges: bigint; siegeSpend: bigint; timesBesieged: bigint; counterStrikes: bigint; treatySecs: bigint }
export interface SeasonData {
  number: number; startsAt: bigint; endsAt: bigint; weights: ScoreWeightsData; penalizeBesieged: boolean; eta: bigint;
  leader: PublicKey | null; leaderScore: bigint; finalized: boolean; prizePaid: bigint;
}
/** 05 10.2 (field order of its table). */
export const seasonCodec = accountCodec<SeasonData>('Season', [
  ['number', 'u32'], ['startsAt', 'i64'], ['endsAt', 'i64'], ['weights', SCORE_WEIGHTS], ['penalizeBesieged', 'bool'], ['eta', 'i64'],
  ['leader', { option: 'pubkey' }], ['leaderScore', 'i128'], ['finalized', 'bool'], ['prizePaid', 'u64'],
]);

export interface LootEntryData { templateId: number; weight: number; ranges: { min: number; max: number }[] }
export interface LootTableData { season: number; entries: LootEntryData[]; eta: bigint }
const LOOT_DISC = discriminator('account', 'LootTable');
/** 05 8.3; `LOOT_TABLE_LEN` read from the data length. */
export function decodeLootTable(data: Buffer, F = PARAM_FIELDS): LootTableData {
  if (!data.subarray(0, 8).equals(LOOT_DISC)) throw new Error('not a LootTable account');
  const entry: Ty = { struct: [['templateId', 'u16'], ['weight', 'u32'], ['ranges', { array: [{ struct: [['min', 'u32'], ['max', 'u32']] }, F] }]] };
  const n = Math.floor((data.length - 8 - 4 - 8) / fixedSize(entry));
  return decode<LootTableData>({ struct: [['season', 'u32'], ['entries', { array: [entry, Math.max(0, n)] }], ['eta', 'i64']] }, data, 8);
}

export interface RollRequestData { owner: PublicKey; mint: PublicKey; holding: PublicKey; season: number; requestedSlot: bigint; oracleProgram: PublicKey; oracleAccount: PublicKey; bump: number }
/** 05 8.2. */
export const rollRequestCodec = accountCodec<RollRequestData>('RollRequest', [
  ['owner', 'pubkey'], ['mint', 'pubkey'], ['holding', 'pubkey'], ['season', 'u32'], ['requestedSlot', 'u64'],
  ['oracleProgram', 'pubkey'], ['oracleAccount', 'pubkey'], ['bump', 'u8'],
]);

export interface QuestMarkData { lastPeriod: number; lastForgeCount: bigint; bump: number }
/** 05 section 9. */
export const questMarkCodec = accountCodec<QuestMarkData>('QuestMark', [['lastPeriod', 'u32'], ['lastForgeCount', 'u64'], ['bump', 'u8']]);
