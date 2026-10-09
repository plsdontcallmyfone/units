/**
 * The templates (docs/spec/04-templates.md section 3, then the arsenal in arsenal.ts): ids, slot kinds, fields with their
 * floor and ceiling parameter names and forge rules, and the one fixed site sentence per template
 * (04 2.7: "Nothing else is said about an item's behaviour"). Floors and ceilings are parameter
 * names; their values come from the on-chain `Template` record (02 2.3), never from this file.
 */
import type { SlotKind } from './api.ts';
import { ARSENAL, arsenalSentence } from './arsenal.ts';

export type ForgeRule = 'towardCeiling' | 'towardFloor' | 'keep' | 'floorWhenBothOn' | 'none';
export type FieldFormat = 'bps' | 'ppm' | 'secs' | 'count' | 'flag' | 'mode' | 'units' | 'lamports';

export interface TemplateField {
  index: number;
  name: string;
  floor: string | number;
  ceiling: string | number;
  forge: ForgeRule;
  format: FieldFormat;
}

export interface TemplateDef {
  id: number;
  name: string;
  kind: SlotKind;
  /** Callbacks, as 04's table states them. */
  callbacks: string[];
  /** What the equip's targets mean (04 2.3). */
  targets: string;
  fields: TemplateField[];
  /** Range bytes seen by the item, without the epoch byte (04 2.4). */
  dataBytes: number;
  forgeable: boolean;
  spec: string;
}

const f = (index: number, name: string, floor: string | number, ceiling: string | number, forge: ForgeRule, format: FieldFormat): TemplateField =>
  ({ index, name, floor, ceiling, forge, format });

const BASE_TEMPLATES: readonly TemplateDef[] = [
  {
    id: 1, name: 'Raid', kind: 'pool', callbacks: ['pool_before_swap', 'pool_after_swap', 'before_transfer', 'on_touch'],
    targets: 'rival mints, 1 to RAID_MAX_RIVALS', dataBytes: 11, forgeable: true, spec: '04 3.1',
    fields: [
      f(0, 'discount_bps', 0, 'RAID_MAX_DISCOUNT_BPS', 'towardCeiling', 'bps'),
      f(1, 'toll_bps', 0, 'RAID_MAX_TOLL_BPS', 'towardCeiling', 'bps'),
      f(2, 'points_per_unit', 0, 'RAID_MAX_POINTS_PER_UNIT', 'towardCeiling', 'count'),
    ],
  },
  {
    id: 2, name: 'Shield', kind: 'pool', callbacks: ['pool_after_swap', 'before_transfer'],
    targets: 'rival mints, 1 to SHIELD_MAX_RIVALS', dataBytes: 6, forgeable: true, spec: '04 3.2',
    fields: [
      f(0, 'sell_cut_bps', 0, 'SHIELD_MAX_SELL_CUT_BPS', 'towardCeiling', 'bps'),
      f(1, 'window_secs', 'SHIELD_MIN_WINDOW_SECS', 'SHIELD_MAX_WINDOW_SECS', 'towardCeiling', 'secs'),
      f(2, 'only_under_siege', 0, 1, 'keep', 'flag'),
    ],
  },
  {
    id: 3, name: 'Wall', kind: 'defense', callbacks: ['before_transfer'],
    targets: 'none', dataBytes: 0, forgeable: true, spec: '04 3.3',
    fields: [f(0, 'max_wallet_bps', 'WALL_MIN_MAX_WALLET_BPS', 10_000, 'towardFloor', 'bps')],
  },
  {
    id: 4, name: 'Spy', kind: 'pool', callbacks: ['pool_before_swap', 'pool_after_swap'],
    targets: 'one rival mint', dataBytes: 0, forgeable: true, spec: '04 3.4',
    fields: [
      f(0, 'mode', 1, 2, 'keep', 'mode'),
      f(1, 'window_secs', 'MIN_TWAP_SECS', 'SPY_MAX_WINDOW_SECS', 'towardFloor', 'secs'),
      f(2, 'trigger_bps', 'SPY_MIN_TRIGGER_BPS', 'SPY_MAX_TRIGGER_BPS', 'towardFloor', 'bps'),
      f(3, 'effect_bps', 0, 'SPY_MAX_EFFECT_BPS', 'towardCeiling', 'bps'),
    ],
  },
  {
    id: 5, name: 'Treaty', kind: 'relation', callbacks: ['pool_before_swap'],
    targets: 'the partner mint', dataBytes: 0, forgeable: false, spec: '04 3.5',
    fields: [
      f(0, 'low_to_high_bps', 0, 'TREATY_MAX_BPS', 'none', 'bps'),
      f(1, 'high_to_low_bps', 0, 'TREATY_MAX_BPS', 'none', 'bps'),
      f(2, 'returns_captured', 0, 1, 'none', 'flag'),
    ],
  },
  {
    id: 6, name: 'Tribute', kind: 'relation', callbacks: ['pool_before_swap'],
    targets: 'the partner mint; role Pay or Receive', dataBytes: 0, forgeable: false, spec: '04 3.6',
    fields: [f(0, 'bps', 0, 'TRIBUTE_MAX_BPS', 'none', 'bps')],
  },
  {
    id: 7, name: 'Half-Life', kind: 'fee', callbacks: ['before_transfer'],
    targets: 'none', dataBytes: 5, forgeable: true, spec: '04 3.7',
    fields: [
      f(0, 'max_fee_ppm', 0, 'HL_MAX_FEE_PPM', 'towardCeiling', 'ppm'),
      f(1, 'half_life_secs', 'HL_MIN_HALF_LIFE_SECS', 'HL_MAX_HALF_LIFE_SECS', 'towardCeiling', 'secs'),
      f(2, 'zero_after_halvings', 1, 'HL_MAX_HALVINGS', 'towardCeiling', 'count'),
    ],
  },
  {
    id: 8, name: 'Transfer Fee', kind: 'fee', callbacks: ['before_transfer'],
    targets: 'the collector', dataBytes: 0, forgeable: true, spec: '04 3.8',
    fields: [
      f(0, 'fee_bps', 0, 'TF_MAX_FEE_BPS', 'towardCeiling', 'bps'),
      f(1, 'max_wallet_bps', 0, 10_000, 'floorWhenBothOn', 'bps'),
    ],
  },
  {
    id: 9, name: 'War orders', kind: 'war', callbacks: [],
    targets: 'none', dataBytes: 0, forgeable: false, spec: '04 3.9',
    fields: [
      f(0, 'siege_threshold', 'WAR_MIN_SIEGE_THRESHOLD', 'WAR_MAX_SIEGE_THRESHOLD', 'none', 'units'),
      f(1, 'siege_spend_bps', 0, 'SIEGE_MAX_SPEND_BPS', 'none', 'bps'),
      f(2, 'siege_twap_secs', 'MIN_TWAP_SECS', 'WAR_MAX_TWAP_SECS', 'none', 'secs'),
      f(3, 'counter_drop_bps', 'WAR_MIN_COUNTER_DROP_BPS', 'WAR_MAX_COUNTER_DROP_BPS', 'none', 'bps'),
      f(4, 'counter_short_secs', 'MIN_TWAP_SECS', 'WAR_MAX_TWAP_SECS', 'none', 'secs'),
      f(5, 'counter_long_secs', 'MIN_TWAP_SECS', 'WAR_MAX_TWAP_SECS', 'none', 'secs'),
      f(6, 'counter_interval_secs', 'COUNTER_STRIKE_MIN_INTERVAL_SECS', 'WAR_MAX_INTERVAL_SECS', 'none', 'secs'),
      f(7, 'counter_spend_bps', 0, 'COUNTER_STRIKE_MAX_SPEND_BPS', 'none', 'bps'),
      f(8, 'raze_enabled', 0, 1, 'none', 'flag'),
      f(9, 'bounty_rate', 0, 'WAR_MAX_BOUNTY_RATE', 'none', 'lamports'),
      f(10, 'crank_bounty_bps', 0, 'MAX_CRANK_BOUNTY_BPS', 'none', 'bps'),
    ],
  },
];

/** Every template the items program runs: 04's nine, then the arsenal (08, 09, 10). */
export const TEMPLATES: readonly TemplateDef[] = [...BASE_TEMPLATES, ...ARSENAL];

export function templateById(id: number): TemplateDef | undefined {
  return TEMPLATES.find((t) => t.id === id);
}

/** Formats a field value as the site prints it (04 2.7: bps as a percent, seconds as a duration). */
export function formatField(format: FieldFormat, v: number): string {
  switch (format) {
    case 'bps': return `${trimNum(v / 100)}%`;
    case 'ppm': return `${trimNum(v / 10_000)}%`;
    case 'secs': return duration(v);
    case 'flag': return v ? 'on' : 'off';
    case 'mode': return String(v);
    case 'lamports': return `${trimNum(v / 1e9)} SOL`;
    case 'units': return String(v);
    case 'count': return String(v);
  }
}

function trimNum(n: number): string {
  return Number.isInteger(n) ? String(n) : n.toFixed(4).replace(/0+$/, '').replace(/\.$/, '');
}

export function duration(secs: number): string {
  if (secs <= 0) return '0 s';
  const units: [number, string][] = [[86_400, 'd'], [3_600, 'h'], [60, 'min'], [1, 's']];
  const parts: string[] = [];
  let left = secs;
  for (const [n, u] of units) {
    const k = Math.floor(left / n);
    if (k > 0) { parts.push(`${k} ${u}`); left -= k * n; }
    if (parts.length === 2) break;
  }
  return parts.join(' ');
}

export interface SentenceContext {
  /** Name of the equipped target (symbol or short address), when the template has targets. */
  target?: string | null;
  /** Tribute: the role of this side. Treaty: whether this mint is the lower key of the pair. */
  role?: 'pay' | 'receive' | 'none';
  isLowKey?: boolean;
}

/** The fixed site sentence of 04 section 3, filled with the item's own fields. */
export function itemSentence(templateId: number, params: readonly number[], ctx: SentenceContext = {}): string {
  const t = templateById(templateId);
  if (!t) return 'Unknown template';
  const v = (i: number): string => {
    const fd = t.fields[i];
    const raw = params[i] ?? 0;
    return fd ? formatField(fd.format, raw) : String(raw);
  };
  const target = ctx.target ?? 'its target';
  switch (templateId) {
    case 1:
      return `Buyers who sold ${target} pay ${v(0)} less creator and holder fee, pay a ${v(1)} toll, and earn ${v(2)} raid points per unit of SOL raided.`;
    case 2:
      return `Holders who came from ${target} pay ${v(0)} extra when they sell within ${v(1)}${params[2] ? ' while we are under siege' : ''}.`;
    case 3:
      return `While we are under siege, no wallet may hold more than ${v(0)} of supply.`;
    case 4:
      return params[0] === 2
        ? `When ${target} falls ${v(2)} over ${v(1)}, buyers pay ${v(3)} less creator and holder fee.`
        : `When ${target} rises ${v(2)} over ${v(1)}, sellers pay ${v(3)} extra.`;
    case 5: {
      const mine = ctx.isLowKey === false ? v(1) : v(0);
      const theirs = ctx.isLowKey === false ? v(0) : v(1);
      return `Every buy of us sends ${mine} to ${target}'s holders, and every buy of ${target} sends ${theirs} to ours${params[2] ? '; captured bags are returned while it holds' : ''}.`;
    }
    case 6:
      return ctx.role === 'receive'
        ? `${target} pays us ${v(0)} of every buy of it as tribute.`
        : `Every buy of us sends ${v(0)} to ${target}'s holders as tribute.`;
    case 7:
      return `Selling or sending costs ${v(0)} of the tokens moved, halving every ${v(1)} held, and nothing after ${v(2)} halvings; buys are free; fees are burned.`;
    case 8:
      return `Every transfer pays ${v(0)} to ${ctx.target ?? 'its collector'}${params[1] ? `; no wallet may hold more than ${v(1)} of supply` : ''}.`;
    case 9:
      return `Siege a rival once ${v(0)} units of its holders' SOL have raided us, spending up to ${v(1)} of the chest; counter-strike when our price falls ${v(3)} between ${v(4)} and ${v(5)}, at most every ${v(6)}, spending up to ${v(7)}; bounties pay ${v(9)} per raid point${params[8] ? '; captured bags may be razed' : ''}.`;
    default:
      return arsenalSentence(templateId, v, target, params) ?? t.name;
  }
}
