/** Mock reads for the site, on only when MOCK_DATA=1 (lib/api.ts). Six invented tokens, their slots,
 * items, a map, a feed and a season, so every page can be seen filled before the programs are on a
 * cluster. Every figure here is made up for display and is labelled so in the nav. Never on in
 * production. */
import explorerDemo from './explorer-demo.json';
import { emptyEconomy, PERIODS, type Period } from './economy';
import type { BattleEvent, General, ItemManifest, ItemSummary, Page, PrizeVaultInfo, ProposalInfo, SeasonInfo, SlotInfo, TreatyInfo, WarInfo, WarMap } from '@hookwars/shared';

export const MOCK = process.env.MOCK_DATA === '1';

const now = Math.floor(Date.now() / 1000);
const h = 3600;

type Tok = { mint: string; name: string; symbol: string; creator: string; chest: string; chestBalance: string; underSiege: boolean };
const TOKENS: Tok[] = [
  { mint: 'Ash7k2pQmV4rJx9nLbT3wYcE6sHdF8gRaN5uKiZoP1Wm', name: 'Ashfall', symbol: 'ASH', creator: 'CrtA9bK2mN4pQ7rS1tU3vW5xY8zA2bC4dE6fG8hJ1kL3', chest: 'ChsA1bC2dE3fG4hJ5kL6mN7pQ8rS9tU1vW2xY3zA4bC5', chestBalance: '41200000000', underSiege: false },
  { mint: 'Brn3dE5fG7hJ9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1f', name: 'Brine', symbol: 'BRINE', creator: 'CrtB8cL3nP5qR8sT2uV4wX6yZ9aB3cD5eF7gH9jK2lM4', chest: 'ChsB2cD3eF4gH5jK6lM7nP8qR9sT1uV2wX3yZ4aB5cD6', chestBalance: '18750000000', underSiege: true },
  { mint: 'Cnd5fG7hJ9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1fG3h', name: 'Cinder', symbol: 'CNDR', creator: 'CrtC7dM4oQ6rS9tU3vW5xY7zA1bC4dE6fG8hJ1kL3mN5', chest: 'ChsC3dE4fG5hJ6kL7mN8pQ9rS1tU2vW3xY4zA5bC6dE7', chestBalance: '9300000000', underSiege: false },
  { mint: 'Dsk7hJ9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1fG3hJ5k', name: 'Dusk', symbol: 'DUSK', creator: 'CrtD6eN5pR7sT1uV4wX6yZ8aB2cD5eF7gH9jK2lM4nP6', chest: 'ChsD4eF5gH6jK7lM8nP9qR1sT2uV3wX4yZ5aB6cD7eF8', chestBalance: '2100000000', underSiege: false },
  { mint: 'Emb9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1fG3hJ5kL7m', name: 'Ember', symbol: 'EMBR', creator: 'CrtE5fP6qS8tU2vW5xY7zA9bC3dE6fG8hJ1kL3mN5pQ7', chest: 'ChsE5fG6hJ7kL8mN9pQ1rS2tU3vW4xY5zA6bC7dE8fG9', chestBalance: '67000000000', underSiege: false },
  { mint: 'Fln2mN4pQ6rS8tU1vW3xY5zA7bC9dE1fG3hJ5kL7mN9p', name: 'Flint', symbol: 'FLNT', creator: 'CrtF4gQ7rT9uV3wX6yZ8aB1cD4eF7gH9jK2lM4nP6qR8', chest: 'ChsF6gH7jK8lM9nP1qR2sT3uV4wX5yZ6aB7cD8eF9gH1', chestBalance: '350000000', underSiege: false },
];
const by = Object.fromEntries(TOKENS.map((t) => [t.mint, t]));
const sig = (n: number) => `${n.toString(36).padStart(4, '0')}MockSig${'x'.repeat(40)}`.slice(0, 64);
const RAIDERS = ['RdrA2bC4dE6fG8hJ1kL3mN5pQ7rS9tU2vW4xY6zA8bC1', 'RdrB3cD5eF7gH9jK2lM4nP6qR8sT1uV3wX5yZ7aB9cD2', 'RdrC4dE6fG8hJ1kL3mN5pQ7rS9tU2vW4xY6zA8bC1dE3', 'RdrD5eF7gH9jK2lM4nP6qR8sT1uV3wX5yZ7aB9cD2eF4'];

const manifest = (kind: ItemManifest['kind'], over: Partial<ItemManifest> = {}): ItemManifest => ({ kind, tokenFlags: 0, poolFlags: 0, maxCutBuyBps: 0, maxCutSellBps: 0, maxCutTransferBps: 0, maxDiscountBps: 0, mayRefuse: false, mayBurn: false, dataBytes: 0, readsOtherPools: 0, marks: false, ...over });
const item = (id: string, templateId: number, templateName: string, kind: ItemManifest['kind'], params: number[], paramsText: string, level: number, source: ItemSummary['source'], owner: string, on: ItemSummary['equippedOn']): ItemSummary => ({
  item: id, itemMint: id.replace(/^Itm/, 'Imt'), templateId, templateName, kind, params, paramsText, manifest: manifest(kind, { maxCutBuyBps: kind === 'pool' ? 300 : 0 }), level, source, author: owner, owner, royaltyBps: 500, runs: String(120 * level), royalties: [], equippedOn: on, closed: false,
});
const ITEMS: ItemSummary[] = [
  item('ItmA1bC2dE3fG4hJ5kL6mN7pQ8rS9tU1vW2xY3zA4bC5d', 1, 'Raid', 'pool', [150, 40, 3], 'Raids at a 1.5% discount, 0.4% toll, 3 points per unit', 2, 'authored', RAIDERS[0]!, [{ mint: TOKENS[0]!.mint, symbol: 'ASH', slot: 1, targets: [TOKENS[1]!.mint], role: 'none' }]),
  item('ItmB2cD3eF4gH5jK6lM7nP8qR9sT1uV2wX3yZ4aB5cD6e', 2, 'Shield', 'pool', [200, 900, 1], 'Cuts sells 2% for 15 minutes, only under siege', 1, 'loot', RAIDERS[1]!, [{ mint: TOKENS[1]!.mint, symbol: 'BRINE', slot: 1, targets: [], role: 'none' }]),
  item('ItmC3dE4fG5hJ6kL7mN8pQ9rS1tU2vW3xY4zA5bC6dE7f', 9, 'War orders', 'war', [500000000, 2000, 1800, 300, 600, 3600, 7200, 1500, 1, 10, 50], 'Sieges above 0.5 SOL of raids, spends 20% of the chest', 1, 'authored', TOKENS[0]!.creator, [{ mint: TOKENS[0]!.mint, symbol: 'ASH', slot: 3, targets: [], role: 'none' }]),
  item('ItmD4eF5gH6jK7lM8nP9qR1sT2uV3wX4yZ5aB6cD7eF8g', 5, 'Treaty', 'relation', [100, 100, 1], 'Shares 1% each way, returns captured', 1, 'authored', TOKENS[2]!.creator, [{ mint: TOKENS[2]!.mint, symbol: 'CNDR', slot: 2, targets: [TOKENS[4]!.mint], role: 'pay' }, { mint: TOKENS[4]!.mint, symbol: 'EMBR', slot: 2, targets: [TOKENS[2]!.mint], role: 'receive' }]),
  item('ItmE5fG6hJ7kL8mN9pQ1rS2tU3vW4xY5zA6bC7dE8fG9h', 7, 'Half-Life', 'fee', [50000, 600, 1], 'Fee starts at 5%, halves every 10 minutes', 3, 'forged', RAIDERS[2]!, []),
  item('ItmF6gH7jK8lM9nP1qR2sT3uV4wX5yZ6aB7cD8eF9gH1j', 4, 'Spy', 'pool', [1, 300, 200, 50], 'Watches a rival and cuts 0.5% when it moves 2%', 1, 'loot', RAIDERS[3]!, []),
];

const slot = (n: number, kind: SlotInfo['kind'], rule: SlotInfo['equipRule'], it: ItemSummary | null, over: Partial<SlotInfo> = {}): SlotInfo => ({
  slot: n, kind, equipRule: rule, bounds: { maxCutBps: kind === 'war' ? 0 : 300, mayRefuse: kind === 'defense', mayWriteData: kind === 'pool', mayAnswerTouch: true },
  noticeSecs: rule === 'vote' ? 1800 : null, dataRange: { offset: n * 8, len: kind === 'pool' ? 8 : 0 }, dataEpoch: 1, item: it, targets: it?.equippedOn[0]?.targets ?? [], role: 'none', locked: null, launchItem: null, performance: null, openProposal: null, ...over,
});
const SLOTS: Record<string, SlotInfo[]> = {
  [TOKENS[0]!.mint]: [slot(0, 'locked', 'locked', null, { locked: { program: 'CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG', label: 'kit' } }), slot(1, 'pool', 'vote', ITEMS[0]!), slot(2, 'relation', 'vote', null), slot(3, 'war', 'vote', ITEMS[2]!)],
  [TOKENS[1]!.mint]: [slot(0, 'pool', 'vote', ITEMS[1]!), slot(1, 'defense', 'performance', null), slot(2, 'war', 'vote', null)],
  [TOKENS[2]!.mint]: [slot(0, 'fee', 'locked', null), slot(1, 'pool', 'vote', null), slot(2, 'relation', 'vote', ITEMS[3]!)],
  [TOKENS[3]!.mint]: [slot(0, 'pool', 'vote', null), slot(1, 'war', 'vote', null)],
  [TOKENS[4]!.mint]: [slot(0, 'locked', 'locked', null, { locked: { program: 'CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG', label: 'kit' } }), slot(1, 'relation', 'vote', ITEMS[3]!), slot(2, 'war', 'vote', null)],
  [TOKENS[5]!.mint]: [slot(0, 'pool', 'vote', null)],
};

const PROPOSALS: Record<string, ProposalInfo[]> = {
  [TOKENS[0]!.mint]: [{ proposal: 'PrpA1bC2dE3fG4hJ5kL6mN7pQ8rS9tU1vW2xY3zA4bC5d', mint: TOKENS[0]!.mint, slot: 2, nonce: 1, proposer: RAIDERS[0]!, item: ITEMS[3]!, voteEnd: now + 5 * h, executableAt: now + 6 * h, status: 'open', votesFor: '1250000000000', votesAgainst: '310000000000', eligibleNow: '4000000000000', quorumBps: 1000, needsSettleFirst: false, myVote: null }],
  [TOKENS[1]!.mint]: [{ proposal: 'PrpB2cD3eF4gH5jK6lM7nP8qR9sT1uV2wX3yZ4aB5cD6e', mint: TOKENS[1]!.mint, slot: 2, nonce: 1, proposer: TOKENS[1]!.creator, item: ITEMS[2]!, voteEnd: now - 2 * h, executableAt: now - h, status: 'passed', votesFor: '900000000000', votesAgainst: '120000000000', eligibleNow: null, quorumBps: 1000, needsSettleFirst: false, myVote: null }],
};

const ev = (n: number, kind: BattleEvent['kind'], t: Tok, other: Tok | null, actor: string | null, amount: string | null, agoSecs: number, detail: Record<string, string> = {}): BattleEvent => ({ kind, ts: now - agoSecs, signature: sig(n), ordinal: 0, mint: t.mint, otherMint: other?.mint ?? null, actor, amount, detail });
const FEED: BattleEvent[] = [
  ev(1, 'raid', TOKENS[0]!, TOKENS[1]!, RAIDERS[0]!, '2400000000', 240),
  ev(2, 'siege', TOKENS[0]!, TOKENS[1]!, null, '3750000000', 900, { until: String(now + 6 * h) }),
  ev(3, 'raid', TOKENS[0]!, TOKENS[1]!, RAIDERS[1]!, '1100000000', 1500),
  ev(4, 'treaty_on', TOKENS[2]!, TOKENS[4]!, TOKENS[2]!.creator, null, 2 * h),
  ev(5, 'forge', TOKENS[4]!, null, RAIDERS[2]!, null, 3 * h, { template: 'Half-Life', level: '3' }),
  ev(6, 'raid', TOKENS[4]!, TOKENS[3]!, RAIDERS[2]!, '5200000000', 4 * h),
  ev(7, 'loot', TOKENS[1]!, null, RAIDERS[1]!, null, 5 * h, { template: 'Shield' }),
  ev(8, 'proposal', TOKENS[0]!, null, RAIDERS[0]!, null, 7 * h, { slot: '2' }),
  ev(9, 'bounty', TOKENS[0]!, null, RAIDERS[3]!, '20000000', 9 * h),
  ev(10, 'raid', TOKENS[2]!, TOKENS[5]!, RAIDERS[3]!, '600000000', 12 * h),
  ev(11, 'equip', TOKENS[1]!, null, TOKENS[1]!.creator, null, 20 * h, { slot: '0', template: 'Shield' }),
  ev(12, 'raid', TOKENS[0]!, TOKENS[1]!, RAIDERS[0]!, '3300000000', 26 * h),
];

const MAP: WarMap = {
  nodes: TOKENS.map((t) => ({ mint: t.mint, symbol: t.symbol, image: null, chest: t.chestBalance, underSiege: t.underSiege })),
  edges: [
    { kind: 'raid', from: TOKENS[0]!.mint, to: TOKENS[1]!.mint, weight: '6800000000', since: now - 26 * h },
    { kind: 'siege', from: TOKENS[0]!.mint, to: TOKENS[1]!.mint, weight: '3750000000', since: now - 900 },
    { kind: 'treaty', from: TOKENS[2]!.mint, to: TOKENS[4]!.mint, weight: null, since: now - 2 * h },
    { kind: 'raid', from: TOKENS[4]!.mint, to: TOKENS[3]!.mint, weight: '5200000000', since: now - 4 * h },
    { kind: 'raid', from: TOKENS[2]!.mint, to: TOKENS[5]!.mint, weight: '600000000', since: now - 12 * h },
  ],
};

const war = (t: Tok): WarInfo => ({
  mint: t.mint, chest: t.chest, chestBalance: t.chestBalance, fundedTotal: String(Number(t.chestBalance) * 2), spent: { siege: '3750000000', counter: '0', bounties: '20000000', cranks: '5000000' }, razedProceeds: null,
  treatyInbox: null, orders: null, underSiege: t.underSiege ? { byChest: TOKENS[0]!.chest, byMint: TOKENS[0]!.mint, until: now + 6 * h } : null, captured: [], inbound: [], siege: [],
  counter: { ready: false, shortTwap: null, longTwap: null, nextAt: null, blocked: null }, season: null,
});
const TREATIES: Record<string, TreatyInfo[]> = {};

const GENERALS: General[] = RAIDERS.map((owner, i) => ({ owner, raidPoints: [420, 310, 180, 60][i]!, raidVolume: ['8100000000', '5200000000', '3300000000', '600000000'][i]!, rank: i + 1 }));

const SEASON: SeasonInfo = {
  number: 1, startsAt: now - 6 * 86400, endsAt: now + 8 * 86400, challengeEndsAt: null,
  weights: { raidVolumeWon: '3', sieges: '50', siegeSpend: '1', timesBesieged: '20', counterStrikes: '30', treatySecs: '1' }, penalizeBesieged: true,
  table: [[TOKENS[0], '1840'], [TOKENS[4], '1210'], [TOKENS[2], '640'], [TOKENS[1], '230'], [TOKENS[3], '90'], [TOKENS[5], '12']].map(([t, score]) => ({ mint: (t as Tok).mint, symbol: (t as Tok).symbol, score: score as string, counters: {} })),
  leader: { mint: TOKENS[0]!.mint, score: '1840' }, finalized: false, winner: null, prizePaid: null,
};
const PRIZE: PrizeVaultInfo = { vault: 'PrzV1aB2cD3eF4gH5jK6lM7nP8qR9sT1uV2wX3yZ4aB5c', lamports: '12400000000', quoteHolding: null, lastWinner: null, shareBps: 1000 };

const launches: Page<Record<string, unknown>> = {
  items: TOKENS.map((t, i) => {
    const equipped = (SLOTS[t.mint] ?? []).find((s) => s.item)?.item ?? null;
    return { launch: `Lnc${t.mint.slice(3)}`, mint: t.mint, creator: t.creator, name: t.name, symbol: t.symbol, supply: '1000000000000000', decimals: 6, creator_fee_bps: 100, lp_fee_bps: 30, slot: 509000000 - i * 40000, image: `/agency/coins/${String(i + 1).padStart(3, '0')}.webp`, item: equipped?.paramsText ?? null, itemKind: equipped?.templateName ?? null };
  }),
  next: null,
};

/** 96 candles of 15 minutes per token: open, high, low, close in SOL, volume in SOL, chest balance
 * in SOL, raid volume in SOL. A fixed pseudo-random walk so the chart renders the same everywhere. */
export type Candle = { ts: number; open: number; high: number; low: number; close: number; volume: number; chest: number; raids: number };
export type MarketInfo = { priceSol: number; priceChange24h: number; volume1h: string; feesToChest: string; holders: number; series: Candle[] };
function market(t: Tok, i: number): MarketInfo {
  let seed = 7 + i * 13;
  const rnd = () => { seed = (seed * 9301 + 49297) % 233280; return seed / 233280; };
  let price = [0.00042, 0.00019, 0.00011, 0.00004, 0.00088, 0.000009][i]!;
  let chest = Number(t.chestBalance) / 1e9 * 0.7;
  const series: Candle[] = [];
  const step = 900;
  for (let k = 95; k >= 0; k--) {
    const open = price;
    const drift = (rnd() - 0.49) * 0.05;
    const close = Math.max(open * (1 + drift), 1e-7);
    const high = Math.max(open, close) * (1 + rnd() * 0.02);
    const low = Math.min(open, close) * (1 - rnd() * 0.02);
    const raids = rnd() > 0.85 ? Number((rnd() * 2.5).toFixed(2)) : 0;
    const volume = Number((rnd() * 4 + raids * 3 + Math.abs(drift) * 60).toFixed(2));
    chest += raids * 0.02 + rnd() * 0.08;
    series.push({ ts: now - k * step, open: Number(open.toPrecision(4)), high: Number(high.toPrecision(4)), low: Number(low.toPrecision(4)), close: Number(close.toPrecision(4)), volume, chest: Number(chest.toFixed(2)), raids });
    price = close;
  }
  const first = series[0]!.open, last = series[95]!.close;
  return { priceSol: last, priceChange24h: Number((((last - first) / first) * 100).toFixed(1)), volume1h: String(Math.round(series.slice(-4).reduce((a, c) => a + c.volume, 0) * 1e9)), feesToChest: String(Math.round(Number(t.chestBalance) * 0.4)), holders: [2557, 1180, 640, 212, 3904, 58][i]!, series };
}

/** The mock answer for a backend path, or undefined when the mock has none (the real backend answers). */
export function mock(path: string): unknown {
  const [p, qs] = path.split('?');
  const q = new URLSearchParams(qs);
  let m: RegExpExecArray | null;
  if (p === '/v1/status') return { cluster: 'https://api.devnet.solana.com', rpcReachable: true, slot: null, database: true, programs: [], indexer: [] };
  if (p === '/v1/templates') return [];
  if (p === '/v1/launches') return launches;
  if (p === '/v1/feed') { const k = q.get('kind'); return { items: k ? FEED.filter((e) => e.kind === k) : FEED, next: null } satisfies Page<BattleEvent>; }
  if (p === '/v1/map') return MAP;
  if (p === '/v1/items') return { items: ITEMS, next: null } satisfies Page<ItemSummary>;
  if ((m = /^\/v1\/items\/(\w+)$/.exec(p!))) { const it = ITEMS.find((x) => x.item === m![1]); return it ? { ...it, history: FEED.filter((e) => e.actor === it.owner) } : undefined; }
  if (p === '/v1/seasons/current') return SEASON;
  // Changed by Hookwars (app pass v3): invented craft, book and wear rows for the demo.
  // Changed by Hookwars (economy panel): empty answers, so the demo shows the panel's empty states.
  if ((m = /^\/v1\/economy\/(\w+)$/.exec(p!))) return emptyEconomy(m[1]!, (PERIODS as readonly string[]).includes(q.get('period') ?? '') ? q.get('period') as Period : '7d');
  if (p === '/v1/craft') return CRAFT;
  if (p === '/v1/book') return BOOK;
  if ((m = /^\/v1\/items\/(\w+)\/wear$/.exec(p!))) return ITEMS.findIndex((x) => x.item === m![1]) === 0 ? { maxCharges: 40, used: 40, dormant: true, repairs: 1 } : null;
  if (p === '/v1/prize-vault') return PRIZE;
  if ((m = /^\/v1\/launches\/(\w+)\/market$/.exec(p!))) { const i = TOKENS.findIndex((t) => t.mint === m![1]); return i < 0 ? undefined : market(TOKENS[i]!, i); }
  if ((m = /^\/v1\/launches\/(\w+)\/(slots|proposals|war|treaties|generals)$/.exec(p!))) {
    const t = by[m[1]!]; if (!t) return undefined;
    return { slots: SLOTS[t.mint] ?? [], proposals: PROPOSALS[t.mint] ?? [], war: war(t), treaties: TREATIES[t.mint] ?? [], generals: GENERALS }[m[2]!];
  }
  // Changed by Hookwars (explorer v2): the explorer's demo is real LiteSVM transactions as the
  // explorer decodes them (lib/explorer-demo.json, from packages/sdk/scripts/explorer-fixtures.mjs).
  // Changed by Hookwars (app pass 5): the new reads answer with empty chain state in the demo (no
  // invented rows), so each page shows its empty state and its forms.
  if (/^\/v1\/access\/\w+$/.test(p!)) return { mode: 0, exclusive: false, policy: null, approvals: [], licenceOffer: null };
  if (p === '/v1/governance/queue') return { armoryAdmin: null, queued: [], marketTerms: [], bookParams: [] };
  if (p === '/v1/templates/submissions') return [];
  if (p === '/v1/war/coalitions') return { coalitions: [], bossPools: [] };
  if (p!.startsWith('/v1/explorer/')) return explorerMock(p!, q);
  return undefined;
}

type DemoEvent = { signature: string; ordinal: number; slot: number; program: string; name: string; story: unknown };
type DemoTx = { slot: number; blockTime: number; ok: boolean; programs: string[] };
const DEMO = explorerDemo as unknown as { programs: { name: string; title: string; address: string; instructions: string[]; events: string[] }[]; txs: Record<string, DemoTx>; byAddress: Record<string, DemoEvent[]> };
const demoSig = (sig: string) => { const t = DEMO.txs[sig]!; return { signature: sig, slot: t.slot, blockTime: t.blockTime, ok: t.ok, memo: null }; };

function explorerMock(p: string, q: URLSearchParams): unknown {
  let m: RegExpExecArray | null;
  if (p === '/v1/explorer/programs') return DEMO.programs.map(({ name, title, address }) => ({ name, title, address }));
  if ((m = /^\/v1\/explorer\/tx\/(\w+)$/.exec(p))) return DEMO.txs[m[1]!];
  if ((m = /^\/v1\/explorer\/program\/(\w+)$/.exec(p))) {
    const pr = DEMO.programs.find((x) => x.address === m![1]);
    if (!pr) return undefined;
    const sigs = Object.entries(DEMO.txs).filter(([, t]) => t.programs.includes(pr.name)).map(([s]) => demoSig(s)).reverse();
    const counts: Record<string, { n: number; last: number }> = {};
    for (const list of Object.values(DEMO.byAddress)) for (const e of list) if (e.program === pr.name) { const c = (counts[e.name] ??= { n: 0, last: 0 }); c.n++; c.last = Math.max(c.last, e.slot); }
    return { address: pr.address, name: pr.name, title: pr.title, deployed: false, instructions: pr.instructions, events: pr.events, indexedCounts: Object.entries(counts).map(([name, c]) => ({ name, n: c.n, last_slot: String(c.last) })), recent: sigs };
  }
  if ((m = /^\/v1\/explorer\/address\/(\w+)$/.exec(p))) {
    const ev = DEMO.byAddress[m[1]!] ?? [];
    const seen = [...new Set(ev.map((e) => e.signature))];
    return { address: m[1], exists: false, lamports: null, owner: null, ownerName: null, executable: false, space: null, kind: 'account', program: null, decoded: { program: null, type: null, data: null }, extra: {}, indexed: { events: ev }, recent: seen.map(demoSig) };
  }
  if (p === '/v1/explorer/search') {
    const v = (q.get('q') ?? '').trim();
    if (DEMO.txs[v]) return { kind: 'transaction', value: v, href: `/tx/${v}` };
    const pr = DEMO.programs.find((x) => x.address === v);
    if (pr) return { kind: 'program', value: v, href: `/program/${v}` };
    if (/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(v)) return { kind: 'account', value: v, href: `/address/${v}` };
    if (/^(template\s*|#)\d+$/i.test(v)) return undefined;
    return { kind: 'invalid', value: v, href: null };
  }
  return undefined;
}

// Demo craft and book rows (MOCK_DATA=1 only; invented for display).
const DK = (n: number) => `Demo${String(n).padStart(2, '0')}11111111111111111111111111111111`.slice(0, 44);
const CRAFT = {
  config: { treasury: DK(1), seasonPool: DK(2) },
  materials: [
    { address: DK(3), id: 1, mint: DK(4), name: 'Iron', emissionCapPerSeason: '1000000', emittedThisSeason: '412000', emittedTotal: '412000', burnedTotal: '91000' },
    { address: DK(5), id: 2, mint: DK(6), name: 'Ember', emissionCapPerSeason: '250000', emittedThisSeason: '38000', emittedTotal: '38000', burnedTotal: '4000' },
  ],
  recipes: [
    { address: DK(7), id: 1, uses: '27', terms: { kind: 0, inputs: [{ materialId: 1, amount: '500' }, { materialId: 2, amount: '50' }], feeLamports: '20000000', templateId: 7, chargesRestored: 0, minLevel: 0, active: true } },
    { address: DK(8), id: 2, uses: '9', terms: { kind: 1, inputs: [{ materialId: 1, amount: '200' }], feeLamports: '5000000', templateId: 7, chargesRestored: 40, minLevel: 0, active: true } },
  ],
  presets: [{ address: DK(9), id: 1, name: 'Siege kit', templateIds: [1, 2] }],
};
const BOOK = {
  config: { params: { takerBps: 50, makerBps: 0, slots: 16 } },
  markets: [{ address: DK(10), baseMint: DK(4), materialId: 1, tickLamports: '1000', minSize: '10', fills: '64',
    bids: [{ id: '12', owner: DK(11), price: '41000', size: '300', expiresAt: '0' }, { id: '9', owner: DK(12), price: '39000', size: '1200', expiresAt: '0' }],
    asks: [{ id: '14', owner: DK(13), price: '44000', size: '500', expiresAt: '0' }] }],
  classBids: [{ address: DK(14), bidder: DK(15), nonce: '1', price: '250000000', expiresAt: '0', class: { templateId: 7, minLevel: 2 } }],
};
