/**
 * The agents, market and social reads against a mocked chain (getProgramAccounts filtered by
 * discriminator) and a mocked indexer: league order, sales as the only price history, lineage from
 * forge events, and nothing invented when the indexer is absent.
 */
import { describe, expect, it } from 'vitest';
import { PublicKey, type GetProgramAccountsFilter } from '@solana/web3.js';
import bs58 from 'bs58';
import { hookwars } from '@hookwars/sdk';
import { agentsLeague, guild, lineageOf, marketItem } from './reads-expansion.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });
const record = (itemsAuthored: number, cranks: number) => ({ ...(zeroOf('agents', 'Passport').record as Record<string, unknown>), itemsAuthored, cranks });

type Acc = { program: PublicKey; key: PublicKey; data: Buffer };
function chain(list: Acc[]) {
  const byKey = new Map(list.map((a) => [a.key.toBase58(), a]));
  return {
    getAccountInfo: async (k: PublicKey) => { const a = byKey.get(k.toBase58()); return a ? { data: a.data, owner: a.program, lamports: 7, executable: false, rentEpoch: 0 } : null; },
    getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map((k) => { const a = byKey.get(k.toBase58()); return a ? { data: a.data, owner: a.program, lamports: 7, executable: false, rentEpoch: 0 } : null; }),
    getProgramAccounts: async (program: PublicKey, opts: { filters: GetProgramAccountsFilter[] }) => list
      .filter((a) => a.program.equals(program))
      .filter((a) => opts.filters.every((f) => 'memcmp' in f && Buffer.from(bs58.decode(f.memcmp.bytes)).equals(a.data.subarray(f.memcmp.offset, f.memcmp.offset + bs58.decode(f.memcmp.bytes).length))))
      .map((a) => ({ pubkey: a.key, account: { data: a.data, owner: a.program, lamports: 7, executable: false, rentEpoch: 0 } })),
  } as never;
}
const db = (rows: Record<string, (args: unknown[]) => unknown[]>) => ({
  query: async (sql: string, args: unknown[]) => {
    const k = Object.keys(rows).find((x) => sql.includes(x));
    return { rows: k ? rows[k]!(args) : [] };
  },
}) as never;

describe('agent league', () => {
  const conn = chain([
    { program: hookwars.AGENTS_ID, key: K(1), data: enc('agents', 'Passport', { name: 'a', record: record(1, 9) }) },
    { program: hookwars.AGENTS_ID, key: K(2), data: enc('agents', 'Passport', { name: 'b', record: record(5, 2) }) },
    { program: hookwars.AGENTS_ID, key: K(3), data: enc('agents', 'AgentsConfig', {}) },
  ]);
  it('ranks passports by the chosen counter and ignores other account kinds', async () => {
    const byItems = await agentsLeague(conn, null) as { sort: string; items: { name: string }[] };
    expect(byItems.sort).toBe('itemsAuthored');
    expect(byItems.items.map((i) => i.name)).toEqual(['b', 'a']);
    const byCranks = await agentsLeague(conn, 'cranks') as { items: { name: string }[] };
    expect(byCranks.items.map((i) => i.name)).toEqual(['a', 'b']);
    const unknown = await agentsLeague(conn, 'prize') as { sort: string };
    expect(unknown.sort).toBe('itemsAuthored');
  });
});

describe('market item', () => {
  const itemMint = K(10);
  const item = hookwars.itemAddress(itemMint);
  const conn = chain([{ program: hookwars.ARMORY_ID, key: item, data: hookwars.itemCodec().encode({ ...zeroOf('armory', 'Item'), itemMint, templateId: 1, level: 2 } as never) }]);
  it('price history is the sale events and nothing else; lineage walks forge events', async () => {
    const d = db({
      ev_market_sold: () => [{ signature: 's1', price: '100' }, { signature: 's2', price: '150' }],
      'from ev_armory_forged where item = $1': (a) => (a[0] === item.toBase58() ? [{ burned: [K(20).toBase58(), K(21).toBase58()], level: 2 }] : []),
      'burned @>': () => [{ item: K(30).toBase58() }],
    });
    const r = await marketItem(conn, d, itemMint.toBase58()) as { sales: unknown[]; listing: unknown; lineage: { level: number; parents: { item: string }[]; children: string[] } };
    expect(r.sales).toHaveLength(2);
    expect(r.listing).toBeNull();
    expect(r.lineage.level).toBe(2);
    expect(r.lineage.parents.map((p) => p.item)).toEqual([K(20).toBase58(), K(21).toBase58()]);
    expect(r.lineage.children).toEqual([K(30).toBase58()]);
  });
  it('without the indexer there is no history, not an estimate', async () => {
    const r = await marketItem(conn, null, itemMint.toBase58()) as { sales: unknown[]; lineage: { parents: unknown[]; level: number | null } };
    expect(r.sales).toEqual([]);
    expect(r.lineage).toMatchObject({ parents: [], level: null });
    expect(await lineageOf(null, item.toBase58())).toMatchObject({ children: [] });
  });
  it('an unknown mint reads as null', async () => {
    expect(await marketItem(conn, null, K(99).toBase58())).toBeNull();
  });
});

describe('guild', () => {
  it('reads the guild, its treasury and only its own actions', async () => {
    const conn = chain([
      { program: hookwars.SOCIAL_ID, key: hookwars.guildAddress(4), data: enc('social', 'Guild', { id: 4, name: 'Hall', threshold: 1 }) },
      { program: hookwars.SOCIAL_ID, key: K(5), data: enc('social', 'GuildAction', { guildId: 4, nonce: 0n }) },
      { program: hookwars.SOCIAL_ID, key: K(6), data: enc('social', 'GuildAction', { guildId: 5, nonce: 0n }) },
    ]);
    const g = await guild(conn, 4) as { name: string; treasuryLamports: string | null; actions: { action: string }[] };
    expect(g.name).toBe('Hall');
    expect(g.treasuryLamports).toBeNull();
    expect(g.actions.map((a) => a.action)).toEqual([K(5).toBase58()]);
    expect(await guild(conn, 9)).toBeNull();
  });
});
