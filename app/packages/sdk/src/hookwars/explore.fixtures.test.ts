// Changed by Hookwars: new file (explorer v2). Decoder vectors from real LiteSVM transactions,
// recorded by programs/tests/tests/explorer_fixtures.rs (EXPLORER_FIXTURES=<dir>). Each figure
// asserted here is the one the Rust suites already assert for the same scenario (tests/items.rs,
// tests/slot_launch.rs). Note: in the test environment the launchpad's id runs `launch_stub` for the
// item callbacks, and the pool item is `pool_item_stub`; the explorer decodes neither, by design.
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { explainTransaction, type ExplainedTransaction } from './explore.ts';
import { sourceOf, type Fixture } from './fixtures/load.ts';

const dir = join(dirname(fileURLToPath(import.meta.url)), 'fixtures', 'explorer');
const all = Object.fromEntries(readdirSync(dir).filter((f) => f.endsWith('.json')).map((f) => {
  const fx = JSON.parse(readFileSync(join(dir, f), 'utf8')) as Fixture;
  return [fx.name, explainTransaction(sourceOf(fx))] as const;
}));
const ev = (t: ExplainedTransaction, name: string) => t.events.filter((e) => e.name === name);
const fact = (t: ExplainedTransaction, title: RegExp, key: string) => t.story.find((l) => title.test(l.title))?.facts.find(([k]) => k === key)?.[1];

describe('recorded transactions', () => {
  it('has every scenario', () => {
    expect(Object.keys(all).sort()).toEqual(['raid_before', 'raid_delivery', 'raid_mark', 'settle_half_life', 'slot_buy_pool_item', 'slot_sell_pool_item', 'slot_sell_wrong_side', 'transfer_half_life_cut', 'transfer_refused']);
  });

  it('decodes every units instruction it has an interface for', () => {
    const missed: string[] = [];
    const walk = (name: string, xs: ExplainedTransaction['instructions']): void => xs.forEach((x) => { if (x.undecoded) missed.push(`${name} ${x.program ?? x.programId} ${x.undecoded}`); walk(name, x.inner); });
    for (const [name, t] of Object.entries(all)) walk(name, t.instructions);
    // Only the two test stand-ins (see the header) stay undecoded.
    for (const m of missed) expect(m).toMatch(/ (launch unknown instruction discriminator|BGeLKiiaGRyz7WTWh1Zf9CHgo7v92rmikCP3BARzv4HT unknown program)$/);
  });

  it('a Half-Life transfer: the slot item ran, its cut is on the transfer and in its own event', () => {
    const t = all.transfer_half_life_cut!;
    const tr = ev(t, 'Transferred')[0]!;
    expect((tr.data.slotCuts as { slot: number; cut: string }[]).map((c) => [c.slot, c.cut])).toEqual([[0, '20000']]);
    expect(ev(t, 'ItemCut')[0]!.data).toMatchObject({ slot: 0, side: 0, amount: '20000' });
    // The item's cut is logged while it runs, before the token's own event.
    expect(t.events.map((e) => e.name)).toEqual(['HoldingCreated', 'ItemCut', 'Transferred']);
    expect(t.story.find((l) => l.title === 'Transfer with 1 slot cut')).toBeDefined();
    expect(fact(t, /item cut on a transfer/, 'Amount')).toBe('20000');
  });

  it('the settle: royalty 10%, the bounty, the rest burned (tests/items.rs half_life)', () => {
    const t = all.settle_half_life!;
    expect(ev(t, 'EquipSettled')[0]!.data).toMatchObject({ royaltyToken: '2000', burned: '17910', bountyToken: '90' });
    expect(ev(t, 'Burned')[0]!.data.amount).toBe('17910');
    expect(fact(t, /^Settlement of slot 0$/, 'Item royalty (token)')).toBe('2000');
  });

  it('a refusal names the innermost program and its error, and marks the events rolled back', () => {
    const t = all.transfer_refused!;
    expect(t.ok).toBe(false);
    expect(t.failure).toMatchObject({ program: 'items', code: 6011, name: 'WalletTooLarge' });
    expect(t.story.every((l) => l.title.startsWith('Rolled back: '))).toBe(true);
    const s = all.slot_sell_wrong_side!;
    expect(s.failure).toMatchObject({ program: 'launch', name: 'ItemCutWrongSide' });
  });

  it('a raid: the pool item cuts the raid buy, the delivery marks it', () => {
    expect(ev(all.raid_before!, 'ItemCut')[0]!.data).toMatchObject({ slot: 1, side: 1, amount: '200000' });
    expect(ev(all.raid_mark!, 'RaidMarked')).toHaveLength(0);
    const m = ev(all.raid_delivery!, 'RaidMarked')[0]!;
    expect(m.data).toMatchObject({ volume: '20000000', points: 60, lootTicket: true });
    expect(fact(all.raid_delivery!, /^Raid marked$/, 'Volume')).toBe('0.02 SOL');
  });

  it('a slot-launch buy and sell: pool item cuts, the swap, in execution order', () => {
    const b = all.slot_buy_pool_item!;
    const cuts = ev(b, 'PoolItemCuts')[0]!;
    expect(cuts.data).toMatchObject({ side: 0, poolCutsDelta: '1000000' });
    expect(ev(b, 'Swapped')[0]!.data).toMatchObject({ direction: 1, amountIn: '100000000' });
    expect(b.events.findIndex((e) => e.name === 'PoolItemCuts')).toBeLessThan(b.events.findIndex((e) => e.name === 'Swapped'));
    expect(fact(b, /^Pool items on this swap$/, 'Side')).toBe('input, before the swap');
    const s = all.slot_sell_pool_item!;
    expect(ev(s, 'PoolItemCuts')[0]!.data).toMatchObject({ side: 1, poolCutsDelta: '500000' });
    expect(ev(s, 'Swapped')[0]!.data.direction).toBe(0);
    expect(fact(s, /^Swap: sell$/, 'Delivered')).toBe(String(ev(s, 'Swapped')[0]!.data.deliveredOut));
  });
});
