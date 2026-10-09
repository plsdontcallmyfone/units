/**
 * The arsenal templates (docs/spec/08-arsenal.md section 4, 10-expansion.md, 09-agents.md): ids 10 to
 * 45 and the composite (41). Field names, kinds, forge rules and range bytes are the ones the
 * programs run (`crates/hookwars-common` `shape` and `manifest`, the template files in
 * `programs/hookwars_items/src/templates/`); the sentence is 08's "Sentence" line. Floors and
 * ceilings live in each template's on-chain `Template` record (02 2.3); this file never states a
 * value for them.
 */
import type { SlotKind } from './api.ts';
import type { FieldFormat, ForgeRule, TemplateDef, TemplateField } from './templates.ts';

const F = (index: number, name: string, forge: ForgeRule, format: FieldFormat): TemplateField =>
  ({ index, name, floor: 'Template floor', ceiling: 'Template ceiling', forge, format });

const T = (
  id: number, name: string, kind: SlotKind, callbacks: string[], targets: string, dataBytes: number,
  forgeable: boolean, spec: string, fields: TemplateField[],
): TemplateDef => ({ id, name, kind, callbacks, targets, dataBytes, forgeable, spec, fields });

const POOL_BEFORE = ['pool_before_swap'];
const POOL_BOTH = ['pool_before_swap', 'pool_after_swap'];
const POOL_AFTER = ['pool_after_swap'];
const TOKEN = ['before_transfer'];

/** Templates 10 to 45 as built. */
export const ARSENAL: readonly TemplateDef[] = [
  T(10, 'Size Tiers', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 't1_lamports', 'keep', 'lamports'), F(1, 't2_lamports', 'keep', 'lamports'),
    F(2, 'cut1_bps', 'towardCeiling', 'bps'), F(3, 'cut2_bps', 'towardCeiling', 'bps'), F(4, 'cut3_bps', 'towardCeiling', 'bps'),
  ]),
  T(11, 'Side Skew', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 'buy_cut_bps', 'towardCeiling', 'bps'), F(1, 'sell_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(12, 'Launch Decay', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 'start_cut_bps', 'towardCeiling', 'bps'), F(1, 'end_cut_bps', 'keep', 'bps'), F(2, 'decay_secs', 'towardCeiling', 'secs'),
  ]),
  T(13, 'Velocity Fee', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 'window_secs', 'keep', 'secs'), F(1, 'swaps_threshold', 'towardCeiling', 'count'),
    F(2, 'cut_per_excess_bps', 'towardCeiling', 'bps'), F(3, 'max_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(14, 'Impact Fee', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 'cut_per_impact_bps', 'towardCeiling', 'bps'), F(1, 'max_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(15, 'Volatility Fee', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 'short_secs', 'keep', 'secs'), F(1, 'long_secs', 'keep', 'secs'), F(2, 'trigger_bps', 'towardFloor', 'bps'), F(3, 'cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(16, 'Rush Hour', 'pool', POOL_BOTH, 'none', 0, true, '08 4.1', [
    F(0, 'start_hour', 'keep', 'count'), F(1, 'hours', 'keep', 'count'), F(2, 'inside_cut_bps', 'towardCeiling', 'bps'), F(3, 'outside_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(17, 'Cooldown', 'defense', TOKEN, 'none', 5, true, '08 4.2', [F(0, 'cooldown_secs', 'towardCeiling', 'secs')]),
  T(18, 'Daily Sell Cap', 'defense', TOKEN, 'none', 11, true, '08 4.2', [F(0, 'cap_bps', 'towardFloor', 'bps')]),
  T(19, 'Max Transaction', 'defense', TOKEN, 'none', 0, true, '08 4.2', [F(0, 'max_tx_bps', 'towardFloor', 'bps')]),
  T(20, 'Flash Guard', 'defense', TOKEN, 'none', 5, true, '08 4.2', [F(0, 'min_slots', 'towardCeiling', 'count')]),
  T(21, 'Dump Brake', 'pool', POOL_AFTER, 'none', 0, true, '08 4.2', [
    F(0, 'short_secs', 'keep', 'secs'), F(1, 'long_secs', 'keep', 'secs'), F(2, 'drop_bps', 'towardFloor', 'bps'), F(3, 'sell_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(22, 'Dust Guard', 'defense', TOKEN, 'none', 0, true, '08 4.2', [F(0, 'min_amount', 'keep', 'units')]),
  T(23, 'Guest List', 'pool', POOL_BEFORE, 'the mint buyers must hold', 0, true, '08 4.2', [
    F(0, 'min_hold', 'towardFloor', 'units'), F(1, 'open_after_secs', 'keep', 'secs'),
  ]),
  T(24, 'Loyalty Pot', 'pool', [...POOL_AFTER, ...TOKEN], 'none', 5, true, '08 4.3', [
    F(0, 'sell_cut_bps', 'towardCeiling', 'bps'), F(1, 'epoch_secs', 'keep', 'secs'),
  ]),
  T(25, 'Holder Stream', 'pool', POOL_BOTH, 'none', 0, true, '08 4.3', [
    F(0, 'buy_cut_bps', 'towardCeiling', 'bps'), F(1, 'sell_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(26, 'Streak', 'reward', TOKEN, 'none', 6, true, '08 4.3', [F(0, 'max_streak', 'towardCeiling', 'count')]),
  T(27, 'Ally Pass', 'pool', POOL_BEFORE, 'the ally mint', 0, true, '08 4.4', [
    F(0, 'min_hold', 'towardFloor', 'units'), F(1, 'discount_bps', 'towardCeiling', 'bps'),
  ]),
  T(28, 'Embargo', 'pool', POOL_BEFORE, 'one or more embargoed mints', 0, true, '08 4.4', [F(0, 'cut_bps', 'towardCeiling', 'bps')]),
  T(29, 'Mercenary', 'pool', [...POOL_AFTER, ...TOKEN], 'none', 11, true, '08 4.5', [F(0, 'points_per_unit', 'towardCeiling', 'count')]),
  T(30, 'Garrison', 'pool', POOL_BEFORE, 'none', 0, true, '08 4.5', [F(0, 'discount_bps', 'towardCeiling', 'bps')]),
  T(31, 'War Levy', 'pool', POOL_AFTER, 'none', 0, true, '08 4.5', [
    F(0, 'trigger_lamports', 'towardFloor', 'lamports'), F(1, 'sell_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(32, 'Sell Burn', 'pool', POOL_BEFORE, 'none', 0, true, '08 4.6', [F(0, 'burn_bps', 'towardCeiling', 'bps')]),
  T(33, 'Target Burn', 'pool', POOL_BOTH, 'none', 0, true, '08 4.6', [
    F(0, 'burn_bps', 'towardCeiling', 'bps'), F(1, 'target_supply_bps', 'keep', 'bps'),
  ]),
  T(34, 'Gift Ember', 'fee', TOKEN, 'none', 0, true, '08 4.6', [F(0, 'cut_bps', 'towardCeiling', 'bps')]),
  T(35, 'Rank Badge', 'reward', TOKEN, 'none', 6, true, '08 4.7', [
    F(0, 'unit_lamports', 'towardFloor', 'lamports'), F(1, 'rank_step', 'keep', 'count'), F(2, 'max_rank', 'keep', 'count'),
  ]),
  T(36, 'Referral', 'pool', POOL_BEFORE, 'none', 0, true, '08 4.7', [F(0, 'cut_bps', 'towardCeiling', 'bps')]),
  T(37, 'Sell Ladder', 'fee', TOKEN, 'none', 0, true, '08 4.7', [
    F(0, 'step_bps', 'keep', 'bps'), F(1, 'cut_per_step_bps', 'towardCeiling', 'bps'), F(2, 'max_cut_bps', 'towardCeiling', 'bps'),
  ]),
  T(38, 'First Blood', 'pool', POOL_BEFORE, 'none', 0, true, '08 4.7', [
    F(0, 'discount_bps', 'towardCeiling', 'bps'), F(1, 'min_lamports', 'keep', 'lamports'),
  ]),
  T(39, 'Guild Tag', 'reward', [...TOKEN, 'on_touch'], 'none', 3, true, '08 4.7', [F(0, 'version', 'keep', 'count')]),
  T(40, 'Patience', 'pool', [...POOL_AFTER, ...TOKEN], 'none', 5, true, '08 4.7', [
    F(0, 'min_age_secs', 'towardFloor', 'secs'), F(1, 'discount_bps', 'towardCeiling', 'bps'), F(2, 'source', 'keep', 'mode'),
  ]),
  T(41, 'Composite', 'pool', [], 'per module', 0, false, '08 2', [F(0, 'module_count', 'keep', 'count'), F(1, 'layout', 'keep', 'mode')]),
  T(42, 'Soulbound', 'defense', TOKEN, 'none', 0, false, '09 4.3', []),
  T(43, 'Coalition', 'relation', [], 'none', 0, false, '10 8', [F(0, 'coalition_id', 'keep', 'count'), F(1, 'max_contribution_bps', 'keep', 'bps')]),
  T(44, 'Boss', 'pool', POOL_AFTER, 'none', 0, false, '10 11.1', [F(0, 'window_secs', 'keep', 'secs')]),
  T(45, 'Rivalry', 'relation', [], 'the rival mint', 0, false, '10 11.3', [
    F(0, 'starts_at', 'keep', 'count'), F(1, 'duration_secs', 'keep', 'secs'), F(2, 'budget_bps', 'keep', 'bps'),
  ]),
];

/** Template families for the site's template pages (08 section 4 headings). */
export const FAMILY_OF: Record<number, 'Fee' | 'Defense' | 'Reward' | 'Relation' | 'War' | 'Burn' | 'Social' | 'Utility'> = {
  1: 'War', 2: 'War', 3: 'Defense', 4: 'Relation', 5: 'Relation', 6: 'Relation', 7: 'Fee', 8: 'Fee', 9: 'War',
  10: 'Fee', 11: 'Fee', 12: 'Fee', 13: 'Fee', 14: 'Fee', 15: 'Fee', 16: 'Fee',
  17: 'Defense', 18: 'Defense', 19: 'Defense', 20: 'Defense', 21: 'Defense', 22: 'Defense', 23: 'Defense',
  24: 'Reward', 25: 'Reward', 26: 'Reward', 27: 'Relation', 28: 'Relation',
  29: 'War', 30: 'War', 31: 'War', 32: 'Burn', 33: 'Burn', 34: 'Burn',
  35: 'Social', 36: 'Social', 37: 'Fee', 38: 'Social', 39: 'Social', 40: 'Reward',
  41: 'Utility', 42: 'Social', 43: 'Relation', 44: 'War', 45: 'War',
};

type V = (i: number) => string;

/** 08's sentence of an arsenal template, filled with the item's own fields (`v`). */
export function arsenalSentence(id: number, v: V, target: string, params: readonly number[]): string | null {
  switch (id) {
    case 10: return `Trades under ${v(0)} pay ${v(2)}, up to ${v(1)} pay ${v(3)}, larger trades pay ${v(4)}.`;
    case 11: return `Buys pay ${v(0)}, sells pay ${v(1)}.`;
    case 12: return `Trades pay ${v(0)} at launch, falling to ${v(1)} over ${v(2)}.`;
    case 13: return `When more than ${v(1)} trades happen in ${v(0)}, each extra trade adds ${v(2)}, up to ${v(3)}.`;
    case 14: return `Trades that move the price pay ${v(0)} per 1% moved, up to ${v(1)}.`;
    case 15: return `When the price moves more than ${v(2)} from its average, trades pay ${v(3)}.`;
    case 16: return `From ${params[0] ?? 0}:00 UTC for ${params[1] ?? 0} hours trades pay ${v(2)}; otherwise ${v(3)}.`;
    case 17: return `After buying, a wallet waits ${v(0)} before selling or sending.`;
    case 18: return `A wallet may sell or send at most ${v(0)} of its holding per day.`;
    case 19: return `No single transfer may move more than ${v(0)} of the supply.`;
    case 20: return `Tokens cannot be sold within ${v(0)} slots of being bought.`;
    case 21: return `While the price is more than ${v(2)} under its average, sells pay ${v(3)}.`;
    case 22: return `Wallet sends under ${v(0)} are refused.`;
    case 23: return `For the first ${v(1)}, only holders of at least ${v(0)} ${target} can buy.`;
    case 24: return `Sells pay ${v(0)} into a pot; every ${v(1)}, wallets that held the whole period can claim their share.`;
    case 25: return `${v(0)} of buys and ${v(1)} of sells are streamed to all holders.`;
    case 26: return `Buy on consecutive days without selling to build a streak, up to ${v(0)}.`;
    case 27: return `Holders of at least ${v(0)} ${target} pay ${v(1)} less on buys.`;
    case 28: return `Buyers who arrive by selling ${target} pay ${v(0)} extra.`;
    case 29: return `Any buyer arriving from another token earns ${v(0)} raid points per unit.`;
    case 30: return `While under siege, buyers pay ${v(0)} less.`;
    case 31: return `While a raid of at least ${v(0)} is under way, sells pay ${v(1)} to the war chest.`;
    case 32: return `${v(0)} of every sell is burned.`;
    case 33: return `${v(0)} of every trade is burned until the supply reaches ${v(1)} of the start.`;
    case 34: return `${v(0)} of every wallet-to-wallet send is burned.`;
    case 35: return `Every ${v(0)} bought earns a unit; each ${v(1)} units is a rank, up to ${v(2)}.`;
    case 36: return `Buyers who name a referrer send ${v(0)} of their buys to that referrer.`;
    case 37: return `Selling a bigger share of your holding at once costs more: ${v(1)} per ${v(0)} sold, up to ${v(2)}.`;
    case 38: return `The first buy of at least ${v(1)} each day pays ${v(0)} less.`;
    case 39: return 'Holders can wear a guild tag.';
    case 40: return `Wallets that held for ${v(0)} pay ${v(1)} less when they sell.`;
    case 41: return `A composite of ${params[0] ?? 0} modules, run in order in one slot.`;
    case 42: return 'This badge never leaves the agent it was issued to.';
    case 43: return `Member of coalition ${params[0] ?? 0}, contributing at most ${v(1)} of its war chest.`;
    case 44: return `Every buy that arrives by selling another units token counts as a raid on the boss, over windows of ${v(0)}.`;
    case 45: return `Rivalry with ${target} for ${v(1)}, with ${v(2)} of the war chest set aside for it.`;
    default: return null;
  }
}
