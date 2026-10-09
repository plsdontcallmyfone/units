/**
 * Account decoders for units.
 *
 * - The token program, the armory, the items program and the war program are decoded from their
 *   generated IDLs (`../../idl/*.json`, built from main at the appint integration), so field names
 *   and layouts are the programs' own. Types are generated into `idl-types.gen.ts`.
 * - Two accounts are still spec layouts because their programs are not on main yet:
 *   `Observations` (the DEX, M3a: 03 3.1) and `RaidLedger` (the items callbacks, M3b: 04 2.9).
 *   Arrays whose length is a parameter still to measure are sized from the account's data length.
 *   INTEGRATION.md lists both.
 */
import { PublicKey } from '@solana/web3.js';
import { decode, discriminator, fixedSize, Reader, type Field, type Ty } from './codec.ts';
import { idlAccountCodec, structFields } from './from-idl.ts';
import type { Armory, Items, Token, War } from './idl-types.gen.ts';

/** PARAM_FIELDS of the armory build (crates/hookwars-common `PARAM_FIELDS`), read from the IDL. */
export const PARAM_FIELDS: number = (() => {
  const f = structFields('armory', 'Item').find(([n]) => n === 'params');
  const ty = f?.[1];
  if (ty && typeof ty === 'object' && 'array' in ty) return ty.array[1];
  throw new Error('armory IDL: Item.params is not an array');
})();
/** Kept for callers of the spec-era API; the value now comes from the IDL. */
export function setParamFields(n: number): void {
  if (n !== PARAM_FIELDS) throw new Error(`PARAM_FIELDS is ${PARAM_FIELDS} in the armory build, not ${n}`);
}

// ---------------------------------------------------------------- token (01, IDL) --

export type SlotBoundsData = Token.SlotBounds;
export type SlotData = Token.Slot;
export type SlotMintData = Token.Mint;
export type SlotHoldingData = Token.Holding;

const mintCodec = idlAccountCodec<Token.Mint>('token', 'Mint');
/** Token program `MAX_SLOTS`: the length of `Mint.slots` in the IDL. */
export const MAX_SLOTS: number = (() => {
  const ty = structFields('token', 'Mint').find(([n]) => n === 'slots')?.[1];
  if (ty && typeof ty === 'object' && 'array' in ty) return ty.array[1];
  throw new Error('token IDL: Mint.slots is not an array');
})();

/** Decodes a token `Mint`. Mints written before the slot table decode with `slotCount 0`. */
export function decodeSlotMint(data: Buffer): SlotMintData {
  return mintCodec.decode(data);
}
/** The slots a mint uses (`slots[..slot_count]`). */
export function activeSlots(m: SlotMintData): SlotData[] {
  return m.slots.slice(0, Math.min(m.slotCount, MAX_SLOTS));
}

export const holdingCodec = idlAccountCodec<Token.Holding>('token', 'Holding');
/** The amount a vote lock holds at `now` (01 4.3: `Holding::locked_at`). */
export function lockedAt(h: SlotHoldingData, now: bigint): bigint {
  return now < h.voteLockUntil ? h.voteLocked : 0n;
}

// ---------------------------------------------------------------- armory (02, IDL) --

export type ManifestData = Armory.Manifest;
export type ArmoryConfigData = Armory.ArmoryConfig;
export type TemplateData = Armory.Template;
export type ItemData = Armory.Item;
export type PerformanceRuleData = Armory.PerformanceRule;
export type SlotStateData = Armory.SlotState;
export type ProposalData = Armory.Proposal;
export type VoteLockData = Armory.VoteLock;
export type ForgeCounterData = Armory.ForgeCounter;
export type EquipConfigData = Armory.EquipConfig;

export const armoryConfigCodec = idlAccountCodec<Armory.ArmoryConfig>('armory', 'ArmoryConfig');
export const pendingParamsCodec = idlAccountCodec<Armory.PendingParams>('armory', 'PendingParams');
const templateCodecInstance = idlAccountCodec<Armory.Template>('armory', 'Template');
const itemCodecInstance = idlAccountCodec<Armory.Item>('armory', 'Item');
/** The `Template` codec (the `F` argument of the spec-era API is checked, not used). */
export function templateCodec(F = PARAM_FIELDS) { setParamFields(F); return templateCodecInstance; }
/** The `Item` codec (the `F` argument of the spec-era API is checked, not used). */
export function itemCodec(F = PARAM_FIELDS) { setParamFields(F); return itemCodecInstance; }
export const slotStateCodec = idlAccountCodec<Armory.SlotState>('armory', 'SlotState');
export const proposalCodec = idlAccountCodec<Armory.Proposal>('armory', 'Proposal');
export const voteLockCodec = idlAccountCodec<Armory.VoteLock>('armory', 'VoteLock');
export const forgeCounterCodec = idlAccountCodec<Armory.ForgeCounter>('armory', 'ForgeCounter');

// ---------------------------------------------------------------- items (04, IDL) --

export type EquipStateData = Items.EquipState;
export const equipStateCodec = idlAccountCodec<Items.EquipState>('items', 'EquipState');

// ---------------------------------------------------------------- war (05, IDL) --

export type WarConfigData = War.WarConfig;
export type WarParamsData = War.WarParams;
export type SeasonCountersData = War.SeasonCounters;
export type CapturedData = War.Captured;
export type WarStateData = War.WarState;
export type ScoreWeightsData = War.ScoreWeights;
export type SeasonData = War.Season;
export type LootEntryData = War.LootEntry;
export type LootTableData = War.LootTable;
export type RollRequestData = War.RollRequest;
export type QuestMarkData = War.QuestMark;

export const warConfigCodec = idlAccountCodec<War.WarConfig>('war', 'WarConfig');
const warStateCodec = idlAccountCodec<War.WarState>('war', 'WarState');
/** Decodes `WarState` (05 2.4; `MAX_CAPTURED` is the IDL's array length). */
export function decodeWarState(data: Buffer): WarStateData { return warStateCodec.decode(data); }
export const seasonCodec = idlAccountCodec<War.Season>('war', 'Season');
const lootTableCodec = idlAccountCodec<War.LootTable>('war', 'LootTable');
/** Decodes a `LootTable`; `entries` keeps the full fixed array, `count` says how many are used. */
export function decodeLootTable(data: Buffer, F = PARAM_FIELDS): LootTableData { setParamFields(F); return lootTableCodec.decode(data); }
/** The entries a loot table uses (`entries[..count]`, `LootTable::active`). */
export function activeLoot(t: LootTableData): LootEntryData[] { return t.entries.slice(0, t.count); }
export const rollRequestCodec = idlAccountCodec<War.RollRequest>('war', 'RollRequest');
export const questMarkCodec = idlAccountCodec<War.QuestMark>('war', 'QuestMark');

// ---------------------------------------------------------------- DEX observation ring (03 3.1, M3a: in the pool account) --

export interface ObservationData { ts: bigint; priceCumulative: bigint; quoteVolume: bigint; swapCount: bigint }
export interface ObservationsData {
  version: number; bump: number; pool: PublicKey; cumulative: bigint; lastPriceQ64: bigint; lastTs: bigint;
  index: number; filled: number; len: number; spacing: number; entries: ObservationData[];
}
const OBS_HEADER: Field[] = [
  ['version', 'u8'], ['bump', 'u8'], ['pool', 'pubkey'], ['cumulative', 'u128'], ['lastPriceQ64', 'u128'], ['lastTs', 'i64'],
  ['index', 'u16'], ['filled', 'u16'], ['len', 'u16'], ['spacing', 'u32'],
];
const OBS_ENTRY: Ty = { struct: [['ts', 'i64'], ['priceCumulative', 'u128'], ['quoteVolume', 'u128'], ['swapCount', 'u64']] };
const OBS_DISC = discriminator('account', 'Observations');
/** `bordrless_swap::state::Pool::LEN`: the ring starts right after the `Pool` fields. */
export const POOL_LEN = 411;
/** Ring header length (`bordrless_core::observations::HEADER_LEN`). */
export const OBS_HEADER_LEN = 96;

/** Decodes the observation ring at the tail of a pool account (`bordrless_swap::obs::ring_of`);
 * the same bytes `exact.windowReadRaw` reads. */
export function decodePoolObservations(poolData: Buffer): ObservationsData {
  const data = poolData.subarray(POOL_LEN);
  if (data.length < OBS_HEADER_LEN || !data.subarray(0, 8).equals(OBS_DISC)) throw new Error('no observation ring in this pool account');
  const r = new Reader(data); r.off = 8;
  const h = r.read({ struct: OBS_HEADER }) as Omit<ObservationsData, 'entries'>;
  r.off = OBS_HEADER_LEN;
  const entries: ObservationData[] = [];
  for (let i = 0; i < h.len; i++) entries.push(r.read(OBS_ENTRY) as ObservationData);
  return { ...h, entries };
}

// ---------------------------------------------------------------- items RaidLedger (04 2.9, spec layout until M3b) --

export interface RaidWindowData { rivalMint: PublicKey; windowStart: bigint; volume: bigint; prevVolume: bigint }
export interface MarkData { clockSlot: bigint; recipient: PublicKey; rival: PublicKey; quoteVolume: bigint; stampedSlots: number }
export interface RaidLedgerData {
  version: number; bump: number; mint: PublicKey; seasonId: number; outboundVolumeSeason: bigint; inbound: RaidWindowData[]; mark: MarkData;
}
const RAID_WINDOW: Ty = { struct: [['rivalMint', 'pubkey'], ['windowStart', 'i64'], ['volume', 'u64'], ['prevVolume', 'u64']] };
const MARK: Ty = { struct: [['clockSlot', 'u64'], ['recipient', 'pubkey'], ['rival', 'pubkey'], ['quoteVolume', 'u64'], ['stampedSlots', 'u8']] };
const RAID_LEDGER_DISC = discriminator('account', 'RaidLedger');

/** 04 2.9 field names, which the war program's reader uses too (05 M4/M5 notes: 04's names
 * `season_id`, `outbound_volume_season`); `RAID_TABLE_LEN` is read from the data length. */
export function decodeRaidLedger(data: Buffer): RaidLedgerData {
  if (!data.subarray(0, 8).equals(RAID_LEDGER_DISC)) throw new Error('not a RaidLedger account');
  const head = 1 + 1 + 32 + 4 + 8;
  const n = Math.floor((data.length - 8 - head - fixedSize(MARK) - 32) / fixedSize(RAID_WINDOW));
  return decode<RaidLedgerData>({ struct: [
    ['version', 'u8'], ['bump', 'u8'], ['mint', 'pubkey'], ['seasonId', 'u32'], ['outboundVolumeSeason', 'u64'],
    ['inbound', { array: [RAID_WINDOW, Math.max(0, n)] }], ['mark', MARK],
  ] }, data, 8);
}
