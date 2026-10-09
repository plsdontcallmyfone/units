/** Mock reads for the site, on only when MOCK_DATA=1 (lib/api.ts). Six invented tokens, their slots,
 * items, a map, a feed and a season, so every page can be seen filled before the programs are on a
 * cluster. Every figure here is made up for display and is labelled so in the nav. Never on in
 * production. */
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

/** The mock answer for a backend path, or undefined when the mock has none (the real backend answers). */
export function mock(path: string): unknown {
  const [p, qs] = path.split('?');
  const q = new URLSearchParams(qs);
  let m: RegExpExecArray | null;
  if (p === '/v1/launches') return launches;
  if (p === '/v1/feed') { const k = q.get('kind'); return { items: k ? FEED.filter((e) => e.kind === k) : FEED, next: null } satisfies Page<BattleEvent>; }
  if (p === '/v1/map') return MAP;
  if (p === '/v1/items') return { items: ITEMS, next: null } satisfies Page<ItemSummary>;
  if ((m = /^\/v1\/items\/(\w+)$/.exec(p!))) { const it = ITEMS.find((x) => x.item === m![1]); return it ? { ...it, history: FEED.filter((e) => e.actor === it.owner) } : undefined; }
  if (p === '/v1/seasons/current') return SEASON;
  if (p === '/v1/prize-vault') return PRIZE;
  if ((m = /^\/v1\/launches\/(\w+)\/(slots|proposals|war|treaties|generals)$/.exec(p!))) {
    const t = by[m[1]!]; if (!t) return undefined;
    return { slots: SLOTS[t.mint] ?? [], proposals: PROPOSALS[t.mint] ?? [], war: war(t), treaties: TREATIES[t.mint] ?? [], generals: GENERALS }[m[2]!];
  }
  return undefined;
}
