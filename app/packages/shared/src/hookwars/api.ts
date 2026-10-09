/**
 * The Hookwars API types (docs/spec/06-app.md 3.2), beside upstream's `api.ts`. Conventions are
 * upstream's: `Address` and `Amount` are strings, times are unix seconds, and anything the backend
 * could not read is `null` (a dash on the site, never zero).
 */
import type { Address, Amount } from '../api.ts';

export type SlotKind = 'fee' | 'reward' | 'defense' | 'relation' | 'pool' | 'locked' | 'war';
export type EquipRule = 'locked' | 'vote' | 'performance';
export type ItemSource = 'authored' | 'loot' | 'forged';
export type ProposalStatus = 'open' | 'passed' | 'failed' | 'executed' | 'cancelled';
export type Role = 'none' | 'pay' | 'receive';

/** On-chain codes (00 4.1, 4.2; crates/bordrless-hook slot_kind, equip_rule). */
export const SLOT_KINDS: readonly SlotKind[] = ['fee', 'reward', 'defense', 'relation', 'pool', 'locked', 'war'];
export const EQUIP_RULES: readonly EquipRule[] = ['locked', 'vote', 'performance'];
export const ITEM_SOURCES: readonly ItemSource[] = ['authored', 'loot', 'forged'];
export const PROPOSAL_STATUSES: readonly ProposalStatus[] = ['open', 'passed', 'failed', 'executed', 'cancelled'];
export const ROLES: readonly Role[] = ['none', 'pay', 'receive'];

/** 01 1.1, fixed for life. */
export interface SlotBounds { maxCutBps: number; mayRefuse: boolean; mayWriteData: boolean; mayAnswerTouch: boolean }

/** 04 2.7 manifest as stored in the Item (02 2.4). `marks` is pool_flags bit 2. */
export interface ItemManifest {
  kind: SlotKind; tokenFlags: number; poolFlags: number;
  maxCutBuyBps: number; maxCutSellBps: number; maxCutTransferBps: number; maxDiscountBps: number;
  mayRefuse: boolean; mayBurn: boolean; dataBytes: number; readsOtherPools: number; marks: boolean;
}

export interface TemplateInfo {
  templateId: number;
  name: string;
  sentence: string;
  program: Address;
  kind: SlotKind;
  fields: { index: number; name: string; min: number; max: number; forge: 'towardCeiling' | 'towardFloor' | 'keep' | 'floorWhenBothOn' | 'none' }[];
  codeHash: string;
  verified: boolean | null;
  upgradeAuthority: Address | null;
  upgradeable: boolean | null;
  status: 'active' | 'retired';
  openAuthoring: boolean; lootEnabled: boolean; forgeEnabled: boolean; maxLevel: number;
  items: number;
}

export interface RoyaltyPosition {
  cutMint: Address; symbol: string;
  unsettled: Amount | null;
  settlesToRoyalty: Amount | null;
  claimable: Amount | null;
  claimedTotal: Amount | null;
}

export interface ItemSummary {
  item: Address; itemMint: Address;
  templateId: number; templateName: string; kind: SlotKind;
  params: number[];
  paramsText: string;
  manifest: ItemManifest;
  level: number; source: ItemSource;
  author: Address; owner: Address | null;
  royaltyBps: number;
  runs: Amount | null;
  royalties: RoyaltyPosition[];
  equippedOn: { mint: Address; symbol: string; slot: number; targets: Address[]; role: Role }[];
  closed: boolean;
}

export interface ProposalInfo {
  proposal: Address; mint: Address; slot: number; nonce: number;
  proposer: Address; item: ItemSummary | null;
  voteEnd: number; executableAt: number;
  status: ProposalStatus;
  votesFor: Amount; votesAgainst: Amount;
  eligibleNow: Amount | null;
  quorumBps: number | null;
  needsSettleFirst: boolean;
  myVote: { amount: Amount; support: boolean; until: number } | null;
}

export interface SlotInfo {
  slot: number; kind: SlotKind; equipRule: EquipRule; bounds: SlotBounds;
  noticeSecs: number | null;
  dataRange: { offset: number; len: number };
  dataEpoch: number;
  item: ItemSummary | null;
  targets: Address[]; role: Role;
  locked: { program: Address; label: 'kit' } | null;
  launchItem: Address | null;
  performance: { rule: Record<string, number>; conditionSince: number | null } | null;
  openProposal: ProposalInfo | null;
}

export interface WarOrdersInfo {
  item: Address; level: number;
  siegeThreshold: Amount; siegeSpendBps: number; siegeTwapSecs: number;
  counterDropBps: number; counterShortSecs: number; counterLongSecs: number; counterIntervalSecs: number; counterSpendBps: number;
  /** Peace returns are not a War orders field: a Treaty item's `returns_captured` (04 3.5) gates them (05 M4/M5 notes). */
  razeEnabled: boolean;
  bountyRate: Amount; crankBountyBps: number;
}

export interface CapturedInfo { rivalMint: Address; symbol: string; amount: Amount; cost: Amount; capturedAt: number; shareOfSupply: number | null; razeAllowanceLeft: Amount | null }

export interface SiegeReadiness {
  rival: Address; rivalSymbol: string;
  rolling: Amount; threshold: Amount | null;
  nextAt: number | null;
  blocked: string | null;
}

export interface WarInfo {
  mint: Address;
  chest: Address | null; chestBalance: Amount | null; fundedTotal: Amount | null;
  spent: { siege: Amount; counter: Amount; bounties: Amount; cranks: Amount } | null;
  razedProceeds: Amount | null;
  treatyInbox: { address: Address; balance: Amount | null; sharedTotal: Amount | null } | null;
  orders: WarOrdersInfo | null;
  underSiege: { byChest: Address; byMint: Address | null; until: number } | null;
  captured: CapturedInfo[];
  inbound: { rival: Address; symbol: string; rolling: Amount }[];
  siege: SiegeReadiness[];
  counter: { ready: boolean; shortTwap: string | null; longTwap: string | null; nextAt: number | null; blocked: string | null };
  season: Record<string, string> | null;
}

export interface TreatyInfo {
  treatyItem: Address;
  kind: 'treaty' | 'tribute';
  partyA: { mint: Address; symbol: string; equipped: boolean; role: Role };
  partyB: { mint: Address; symbol: string; equipped: boolean; role: Role };
  active: boolean;
  params: number[]; paramsText: string;
  returnsCaptured: boolean;
  secsThisSeason: Amount | null;
}

export type BattleKind =
  | 'raid' | 'siege' | 'siege_waited' | 'counter_strike' | 'raze' | 'return' | 'treaty_on' | 'treaty_off'
  | 'treaty_shared' | 'equip' | 'proposal' | 'settle' | 'bounty' | 'roll' | 'loot' | 'forge' | 'quest'
  | 'season' | 'prize';

export interface BattleEvent { kind: BattleKind; ts: number; signature: string; ordinal: number; mint: Address; otherMint: Address | null; actor: Address | null; amount: Amount | null; detail: Record<string, string> }

export interface MyHoldingWar {
  mint: Address;
  raidPoints: number | null; tickets: number | null;
  bountyClaimable: Amount | null;
  voteLocked: { amount: Amount; until: number } | null;
  shieldOrigin: { rival: Address; until: number } | null;
  halfLifeSince: number | null;
  quests: { questId: 1 | 2; name: 'Raid' | 'Forge'; claimable: boolean; lastPeriod: number | null; reason: string | null }[];
}

export interface General { owner: Address; raidPoints: number; raidVolume: Amount; rank: number }

export interface LootRollInfo { roll: Address; mint: Address; status: 'requested' | 'revealed' | 'cancelled'; requestedAt: number; expiresAt: number | null; item: ItemSummary | null }

export interface LootTableInfo { season: number; eta: number; entries: { templateId: number; templateName: string; weight: number; ranges: { min: number; max: number }[] }[] }

export interface SeasonInfo {
  number: number; startsAt: number; endsAt: number; challengeEndsAt: number | null;
  weights: Record<string, Amount>; penalizeBesieged: boolean;
  table: { mint: Address; symbol: string; score: string; counters: Record<string, string> }[];
  leader: { mint: Address; score: string } | null;
  finalized: boolean; winner: Address | null;
  prizePaid: Amount | null;
}

export interface PrizeVaultInfo { vault: Address; lamports: Amount | null; quoteHolding: Amount | null; lastWinner: Address | null; shareBps: number | null }

export interface SlotSpecInput {
  kind: SlotKind; equipRule: EquipRule; bounds: SlotBounds; noticeSecs: number; dataLen: number;
  launchItem: Address | null; targets: Address[]; role: Role;
  rule: Record<string, number> | null;
}

export type MapEdgeKind = 'treaty' | 'tribute' | 'rivalry' | 'raid' | 'siege' | 'captured';
export interface WarMap {
  nodes: { mint: Address; symbol: string; image: string | null; chest: Amount | null; underSiege: boolean }[];
  edges: { kind: MapEdgeKind; from: Address; to: Address; weight: Amount | null; since: number | null }[];
}

export interface Page<T> { items: T[]; next: string | null }

export interface QuestInfo { questId: 1 | 2; name: 'Raid' | 'Forge'; sentence: string }

/** What `/v1/status` reports: which programs exist on the cluster the backend reads. */
export interface HookwarsStatus {
  cluster: string;
  rpcReachable: boolean;
  slot: number | null;
  programs: { name: string; address: Address; deployed: boolean | null; upgradeAuthority: Address | null; executableHash: string | null }[];
  indexer: { program: string; cursor: string | null; lastSlot: number | null; updatedAt: number | null }[];
  database: boolean;
}
