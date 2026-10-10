// Changed by Hookwars: new file (economy panel). The shapes /v1/economy/* answers with
// (app/apps/api/src/economy.ts), and their empty forms for demo mode: demo mode shows the panel's
// empty states, never invented rows.

export const PERIODS = ['24h', '7d', '30d', 'season', 'all'] as const;
export type Period = (typeof PERIODS)[number];
export const PERIOD_LABEL: Record<Period, string> = { '24h': '24 hours', '7d': '7 days', '30d': '30 days', season: 'This season', all: 'All time' };

export const FEE_SOURCES = ['dex', 'launchLp', 'itemRun', 'sale', 'licence', 'lease', 'bookFill', 'recipe'] as const;
export type FeeSource = (typeof FEE_SOURCES)[number];
export const SOURCE_LABEL: Record<FeeSource, string> = {
  dex: 'DEX share', launchLp: 'Launch LP', itemRun: 'Item runs', sale: 'Item sales', licence: 'Licences', lease: 'Lease fees', bookFill: 'Order book', recipe: 'Recipes',
};
export const SOURCE_WHAT: Record<FeeSource, string> = {
  dex: 'The protocol share of swap fees on pools quoted in bridged SOL',
  launchLp: 'Recorded by no program yet',
  itemRun: 'The item protocol share taken first at each settle',
  sale: 'The market fee on every item sale',
  licence: 'The protocol share of each licence price',
  lease: 'Lease fees pay the lessor in full; no protocol share is taken',
  bookFill: 'Taker and maker fees on material and class fills',
  recipe: 'The protocol share of recipe fees',
};

export interface Window { period: Period; from: number; to: number; bucketSecs: number; seasonNumber: number | null }

export interface Revenue {
  window: Window;
  sources: { source: FeeSource; lamports: string | null; events: number }[];
  totalLamports: string | null;
  series: { t: number; bySource: Partial<Record<FeeSource, string>> }[];
  otherMints: { mint: string; source: FeeSource; amount: string; events: number }[];
  configTotals: { book: string | null; craft: string | null };
}
export interface Settlement {
  window: Window;
  quote: { holderRoyalty: string | null; author: string | null; rent: string | null; bounty: string | null; destinations: string | null; settles: number };
  dexShare: string | null;
  tokenSide: { mint: string; settles: number; holderRoyalty: string; author: string; rent: string; bounty: string; destinations: string; burned: string; protocol: string }[];
}
export interface Builders {
  window: Window;
  templates: { templateId: number; name: string | null; authorLamports: string; payments: number }[];
  holders: { wallet: string; royaltyLamports: string; licenceLamports: string }[];
  items: { item: string; itemMint: string | null; templateId: number | null; name: string | null; owner: string | null; settledLamports: string; settles: number }[];
}
export interface Market {
  window: Window;
  sales: { count: number; volumeLamports: string | null; feeLamports: string | null; resaleLamports: string | null };
  classes: { templateId: number; name: string | null; listings: number; floorLamports: string | null; lastSaleLamports: string | null; lastSaleTs: number | null; sales: number }[];
  activeListings: number;
  licences: { live: number; bought: number; incomeLamports: string | null; protocolLamports: string | null; authorLamports: string | null; holderLamports: string | null };
  leases: { active: number; started: number; feesLamports: string | null; rentLamports: string | null };
  commissions: { open: number; openBountyLamports: string | null; paid: number; paidLamports: string | null; refunded: number };
}
export interface Craft {
  window: Window;
  materials: { id: number; name: string; mint: string; cap: string; emittedThisSeason: string; emittedTotal: string; burnedTotal: string; dropped: string | null; burnedByRecipes: string | null; net: string | null }[];
  dropsBySource: { source: number; materialId: number; amount: string; drops: number }[];
  recipes: { crafts: number; repairs: number; feesLamports: string | null };
  books: { materialId: number; name: string | null; market: string; bestBid: string | null; bestAsk: string | null; spread: string | null; bidDepth: string; askDepth: string; bidOrders: number; askOrders: number; fills: number; volumeLamports: string | null; lastPrice: string | null; lastTs: number | null }[];
  classFills: { count: number; volumeLamports: string | null };
  wear: { tracked: number; dormant: number; repairs: number };
  chain: boolean;
}
export interface Cranks {
  window: Window;
  byKind: { kind: string; runs: number; bountyLamports: string | null }[];
  top: { cranker: string; runs: number; bountyLamports: string }[];
  agents: { passport: string; name: string | null; byKind: Record<string, string>; records: number }[];
}
export interface War {
  window: Window;
  inflows: { funded: string | null; fundings: number; fromCompanions: string | null; treatyShared: string | null; coalitionContributed: string | null };
  outflows: { siegeSpent: string | null; sieges: number; counterStrikeSpent: string | null; razeProceeds: string | null; razes: number; coalitionSiegeSpent: string | null; bountiesPaid: string | null };
  boss: { funded: string | null; claimed: string | null; sealed: number };
  prize: { toWinner: string | null; toTreasury: string | null; paid: number };
  chests: { mint: string; fundedLamports: string; siegeSpentLamports: string; razeLamports: string }[];
  series: { t: number; funded: string; spent: string }[];
}

export const CRANK_LABEL: Record<string, string> = {
  settle: 'Settle', siege: 'Siege', counterStrike: 'Counter-strike', raze: 'Raze', coalitionSiege: 'Coalition siege', coalitionRaze: 'Coalition raze',
  treatyShare: 'Treaty share', prize: 'Season prize', bookExpiry: 'Order expiry', buyback: 'Buyback', feeClaim: 'Fee claim', holderShare: 'Holder share',
};
/** `hookwars_common::eco_cpi::drop_source`. */
export const DROP_SOURCE: Record<number, string> = { 0: 'Settles', 1: 'Raid reveals', 2: 'Season finish', 3: 'Quest claims' };

/** Empty answers for demo mode: every list empty, every figure null. */
export function emptyEconomy(section: string, period: Period): unknown {
  const now = Math.floor(Date.now() / 1000);
  const window: Window = { period, from: now, to: now, bucketSecs: 86_400, seasonNumber: null };
  switch (section) {
    case 'revenue': return { window, sources: FEE_SOURCES.map((source) => ({ source, lamports: null, events: 0 })), totalLamports: null, series: [], otherMints: [], configTotals: { book: null, craft: null } } satisfies Revenue;
    case 'settlement': return { window, quote: { holderRoyalty: null, author: null, rent: null, bounty: null, destinations: null, settles: 0 }, dexShare: null, tokenSide: [] } satisfies Settlement;
    case 'builders': return { window, templates: [], holders: [], items: [] } satisfies Builders;
    case 'market': return { window, sales: { count: 0, volumeLamports: null, feeLamports: null, resaleLamports: null }, classes: [], activeListings: 0, licences: { live: 0, bought: 0, incomeLamports: null, protocolLamports: null, authorLamports: null, holderLamports: null }, leases: { active: 0, started: 0, feesLamports: null, rentLamports: null }, commissions: { open: 0, openBountyLamports: null, paid: 0, paidLamports: null, refunded: 0 } } satisfies Market;
    case 'craft': return { window, materials: [], dropsBySource: [], recipes: { crafts: 0, repairs: 0, feesLamports: null }, books: [], classFills: { count: 0, volumeLamports: null }, wear: { tracked: 0, dormant: 0, repairs: 0 }, chain: false } satisfies Craft;
    case 'cranks': return { window, byKind: Object.keys(CRANK_LABEL).map((kind) => ({ kind, runs: 0, bountyLamports: null })), top: [], agents: [] } satisfies Cranks;
    case 'war': return { window, inflows: { funded: null, fundings: 0, fromCompanions: null, treatyShared: null, coalitionContributed: null }, outflows: { siegeSpent: null, sieges: 0, counterStrikeSpent: null, razeProceeds: null, razes: 0, coalitionSiegeSpent: null, bountiesPaid: null }, boss: { funded: null, claimed: null, sealed: 0 }, prize: { toWinner: null, toTreasury: null, paid: 0 }, chests: [], series: [] } satisfies War;
    default: return undefined;
  }
}
