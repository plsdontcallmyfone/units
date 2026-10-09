/**
 * Every parameter Hookwars introduces (docs/spec/00-overview.md section 6) and the per-template
 * floors and ceilings named in 04 section 3. A value is `null` until a test measured it or the owner
 * decided it; the site prints a dash for `null`, never a guess. When the programs land, the backend
 * fills these from the on-chain configs (`ArmoryConfig`, `WarConfig`, the DEX and launch configs) and
 * from the program constants; INTEGRATION.md lists where each comes from.
 */
export type ParamSource = 'measured' | 'owner' | 'build-constant';

export interface ParamDef {
  name: string;
  meaning: string;
  setBy: ParamSource[];
  /** Known value, or null ("to set"). */
  value: number | null;
  /** Where the value came from when it is known. */
  source: string | null;
}

const p = (name: string, meaning: string, setBy: ParamSource[], value: number | null = null, source: string | null = null): ParamDef =>
  ({ name, meaning, setBy, value, source });

export const PARAMS: readonly ParamDef[] = [
  p('MAX_SLOTS', 'slots per mint', ['measured'], 4, 'bordrless_token constants.rs MAX_SLOTS (M1 build constant, proposed from tests/budgets.rs)'),
  p('MAX_CUTTING_SLOTS', 'slots that may answer cuts on one transfer', ['measured'], 3, 'M1 proposal: MAX_DELTAS (3); 2 when the Locked slot can cut'),
  p('MAX_ROYALTY_BPS', 'ceiling on an item royalty share of its own cuts', ['owner']),
  p('MIN_NOTICE_SECS', 'shortest slot notice', ['owner']),
  p('MAX_NOTICE_SECS', 'longest slot notice', ['owner']),
  p('VOTE_PERIOD_SECS', 'holder vote length', ['owner']),
  p('VOTE_QUORUM_BPS', 'quorum of eligible supply', ['owner']),
  p('OBS_RING_LEN', 'observation ring entries per pool', ['measured']),
  p('MIN_TWAP_SECS', 'shortest window a TWAP read may use', ['owner']),
  p('MAX_ROUTE_HOPS', 'hops in one swap_route', ['measured']),
  p('MAX_CRANK_BOUNTY_BPS', 'crank bounty ceiling', ['owner']),
  p('SIEGE_MAX_SPEND_BPS', 'siege spend per call, of the chest', ['owner']),
  p('SIEGE_INTERVAL_SECS', 'spacing of sieges', ['owner']),
  p('COUNTER_STRIKE_MAX_SPEND_BPS', 'counter-strike spend per call', ['owner']),
  p('RAZE_MAX_BPS_PER_INTERVAL', 'share of captured holdings sold per raze interval', ['owner']),
  p('RAZE_INTERVAL_SECS', 'raze interval', ['owner']),
  p('BOUNTY_MAX_PER_CLAIM', 'bounty cap per claim, lamports', ['owner']),
  p('LOOT_MIN_RAID_LAMPORTS', 'smallest raid buy that earns a loot ticket', ['owner']),
  p('SEASON_SECS', 'season length', ['owner']),
  p('CHALLENGE_SECS', 'challenge window after a season ends', ['owner']),
  p('SEASON_PRIZE_SHARE_BPS', 'share of protocol fees paid to the last winner', ['owner']),
  p('WAR_BPS_MAX', 'ceiling on a companion war_bps', ['owner']),
  p('MAX_POOL_ITEM_CUT_BPS', 'ceiling on all Pool-item cuts on one side', ['owner']),
  p('PARAM_FIELDS', 'u32 parameter fields per item (at least 11, the War orders template)', ['build-constant']),
  p('FORGE_GAIN_BPS', 'share of the remaining distance to the ceiling a forge closes', ['owner']),
  p('POINT_UNIT_LAMPORTS', 'quote volume per raid point', ['owner']),
  p('SIEGE_UNIT_LAMPORTS', 'unit of the War orders siege threshold', ['owner']),
  p('MAX_CAPTURED', 'captured holdings per war state', ['measured']),
  p('RAID_TABLE_LEN', 'rivals tracked per raid ledger', ['measured']),
  p('RAID_WINDOW_SECS', 'raid volume window', ['owner']),
  p('SIEGE_MAX_PREMIUM_BPS', 'largest rival premium over its TWAP a siege accepts', ['owner']),
  p('SIEGE_SLIPPAGE_BPS', 'siege output margin under the quote', ['owner']),
  p('COUNTER_STRIKE_MIN_INTERVAL_SECS', 'shortest counter-strike spacing', ['owner']),
  p('ROLL_EXPIRY_SECS', 'loot roll expiry', ['owner']),
  p('LOOT_TABLE_LEN', 'entries per loot table', ['measured']),
  p('MAX_ITEM_TARGETS', 'targets per equip', ['owner']),
  p('ADMIN_TIMELOCK_SECS', 'delay on admin setters', ['owner']),
  p('QUEST_RAID_POINTS', 'raid points the Raid quest needs', ['owner']),
  p('QUEST_PERIOD_SECS', 'one quest claim per period', ['owner']),
];

/** The smallest PARAM_FIELDS the spec allows (04: the War orders template uses 11 fields). The
 * account decoders use it until the armory build fixes the constant; INTEGRATION.md item A1. */
export const PARAM_FIELDS_MIN = 11;

export function param(name: string): number | null {
  return PARAMS.find((x) => x.name === name)?.value ?? null;
}
