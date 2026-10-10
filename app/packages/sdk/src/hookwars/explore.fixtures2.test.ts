// Changed by Hookwars: new file (app pass v3). Decoder vectors for war, market, craft, book and memo
// transactions, recorded from LiteSVM by programs/tests/tests/explorer_fixtures_2.rs
// (EXPLORER_FIXTURES=<dir>) in the scenarios the Rust suites already prove.
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { explainTransaction, type ExplainedTransaction } from './explore.ts';
import { sourceOf, type Fixture } from './fixtures/load.ts';

const dir = join(dirname(fileURLToPath(import.meta.url)), 'fixtures', 'explorer2');
const all = Object.fromEntries(readdirSync(dir).filter((f) => f.endsWith('.json')).map((f) => {
  const fx = JSON.parse(readFileSync(join(dir, f), 'utf8')) as Fixture;
  return [fx.name, explainTransaction(sourceOf(fx))] as const;
}));
const names = (t: ExplainedTransaction) => t.events.map((e) => `${e.program}:${e.name}`);

const missed = (t: ExplainedTransaction): string[] => {
  const out: string[] = [];
  const walk = (xs: ExplainedTransaction['instructions']): void => xs.forEach((x) => { if (x.undecoded) out.push(`${x.program ?? x.programId} ${x.undecoded}`); walk(x.inner); });
  walk(t.instructions);
  return out;
};

describe('recorded economy transactions', () => {
  it('has every scenario, all decoded', () => {
    expect(Object.keys(all).sort()).toEqual(['book_fill', 'book_place_ask', 'craft_item', 'market_buy', 'market_list', 'memo_directive', 'war_siege']);
    for (const t of Object.values(all)) expect(missed(t)).toEqual([]);
  });
  it('a siege: the chest buys the rival through the launch pool and the war step reports it', () => {
    const t = all.war_siege!;
    expect(names(t)).toContain('swap:Swapped');
    expect(names(t)[names(t).length - 1]).toBe('war:SiegeExecuted');
    expect(t.story.some((l) => l.kind === 'war')).toBe(true);
  });
  it('a market listing escrows the item; the sale moves it and reports the sale', () => {
    expect(names(all.market_list!)).toEqual(['token:HoldingCreated', 'token:Transferred', 'market:Listed']);
    expect(names(all.market_buy!)).toEqual(['token:HoldingCreated', 'token:Transferred', 'market:Sold']);
  });
  it('a craft burns the inputs, pays the fee and the armory mints a crafted item', () => {
    const n = names(all.craft_item!);
    expect(n[0]).toBe('token:Burned');
    expect(n).toEqual(expect.arrayContaining(['craft:ProtocolFee', 'armory:ItemCreated', 'armory:ItemCrafted', 'craft:Crafted']));
    expect(n.indexOf('armory:ItemCrafted')).toBeLessThan(n.indexOf('craft:Crafted'));
  });
  it('the book: an ask rests, a bid fills against it', () => {
    expect(names(all.book_place_ask!)).toEqual(['token:Transferred', 'book:Placed']);
    expect(names(all.book_fill!)).toContain('book:Filled');
    expect(all.book_fill!.story.some((l) => l.kind === 'book')).toBe(true);
  });
  it('an operator directive: the memo and DirectiveSet in one transaction', () => {
    const t = all.memo_directive!;
    expect(names(t)).toEqual(['agents:DirectiveSet']);
    expect(t.story.map((l) => l.kind)).toEqual(['agents', 'memo']);
  });
});
