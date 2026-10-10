// Changed by Hookwars: new file (app pass v3). Every economy builder and every armory builder
// builds from the regenerated IDLs (no "missing account"), with the IDL's discriminator, and the
// suffix shapes match crates/hookwars-common (`agents_record`, `market`, `eco_cpi`).
import { describe, expect, it } from 'vitest';
import { PublicKey, type TransactionInstruction } from '@solana/web3.js';
import { NO_RULES } from '@hookwars/shared';
import * as h from './index.ts';
import { companionCreatorAddress } from '../addresses.ts';
import { launchRulesFromInput } from '../index.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const disc = (program: string, name: string, ix: TransactionInstruction) => expect([...ix.data.subarray(0, 8)]).toEqual(h.coderOf(program).instruction(name).discriminator);
const count = (program: string, name: string) => h.coderOf(program).accountsOf(name).length;

describe('armory builders on the regenerated IDL', () => {
  it('each builds with every named account', () => {
    const [a, m, i] = [K(1), K(2), K(3)];
    const cases: [string, TransactionInstruction][] = [
      ['create_item', h.createItem(a, 1, Array(11).fill(0), 0, 0n)],
      ['vote', h.vote(a, m, 0, 0n, true, 1n)],
      ['propose', h.propose(a, m, 0, 0n, i, [], 0)],
      ['finalize', h.finalize(m, 0, 0n)],
      ['execute', h.execute(a, m, 0, 0n, {})],
      ['cancel', h.cancelProposal(a, m, 0, 0n)],
      ['close_vote', h.closeVote(a, m, 0, 0n)],
      ['close_proposal', h.closeProposal(a, m, 0, 0n)],
      ['claim_royalty', h.claimRoyalty(a, i, K(4), K(5), 1n)],
      ['forge', h.forge(a, { item: K(6), itemMint: K(7) }, { item: K(8), itemMint: K(9) }, 1, 0n)],
      ['set_template_economy', h.setTemplateEconomy(a, 1, 500, 0, 1, 3)],
      ['set_item_protocol_bps', h.setItemProtocolBps(a, 100)],
      ['init_counters', h.initCounters(a, K(4))],
      ['register_preset', h.registerPreset(a, 1, [1, 2], 'p')],
    ];
    for (const [name, ix] of cases) { disc('armory', name, ix); expect(ix.keys.length).toBeGreaterThanOrEqual(count('armory', name)); }
  });
  it('close_proposal passes the bond mark always and the bond when given', () => {
    const p = h.proposalAddress(K(2), 0, 0n);
    const names = h.coderOf('armory').accountsOf('close_proposal').map((x) => x.name);
    const ix = h.closeProposal(K(1), K(2), 0, 0n, K(9));
    expect(ix.keys[names.indexOf('bond_mark')]!.pubkey.equals(h.bondMarkAddress(p))).toBe(true);
    expect(ix.keys[names.indexOf('bond')]!.pubkey.equals(K(9))).toBe(true);
  });
  it('fuse encodes its targets as (start, count) pairs', () => {
    const ix = h.fuse(K(1), [{ item: K(2), itemMint: K(3), templateId: 1, start: 0, count: 2 }, { item: K(4), itemMint: K(5), templateId: 2, start: 2, count: 1 }], 250, 9n);
    disc('armory', 'fuse', ix);
    expect([...ix.data.subarray(8, 18)]).toEqual([2, 0, 0, 0, 0, 2, 2, 1, 250, 0]);
    expect(ix.keys.length).toBe(count('armory', 'fuse') + 2 + 6);
  });
});

describe('suffixes match crates/hookwars-common', () => {
  it('social, init-wear, rent, fee and settle craft have the program shapes', () => {
    expect(h.socialSuffix(h.ARMORY_ID, K(1))).toHaveLength(5);
    expect(h.initWearSuffix(h.ARMORY_ID, K(1))).toHaveLength(6);
    expect(h.rentSuffix(K(1), K(2), K(3))).toHaveLength(4);
    expect(h.feeSuffix(7, K(1), K(2), K(3))).toHaveLength(4);
    expect(h.settleCraftSuffix({ item: K(1), itemMint: K(2), recipient: K(3), materialId: 1 })).toHaveLength(12);
    expect(h.socialSuffix(h.MARKET_ID, K(1))[4]!.pubkey.equals(h.socialCallerAddress(h.MARKET_ID))).toBe(true);
    expect(h.initWearSuffix(h.ARMORY_ID, K(1))[3]!.pubkey.equals(h.craftCallerAddress(h.ARMORY_ID))).toBe(true);
    expect(h.settleCraftSuffix({ item: K(1), itemMint: K(2), recipient: K(3), materialId: 1 })[3]!.pubkey.equals(h.craftCallerAddress(h.ITEMS_ID))).toBe(true);
  });
  it('orders them rent, craft, fee, social, agents', () => {
    const t = h.suffixes({ agents: h.recordSuffix(h.ITEMS_ID, K(9), K(8)), fee: h.feeSuffix(7, K(1), K(2), K(3)), rent: h.rentSuffix(K(1), K(2), K(3)), craft: h.settleCraftSuffix({ item: K(1), itemMint: K(2), recipient: K(3), materialId: 1 }) });
    expect([t[0]!.pubkey, t[4]!.pubkey, t[16]!.pubkey, t[20]!.pubkey].map((k) => k.toBase58())).toEqual([h.MARKET_ID, h.CRAFT_ID, h.templateAddress(7), h.AGENTS_ID].map((k) => k.toBase58()));
  });
});

describe('craft, book, companion and items builders', () => {
  it('each builds with the IDL discriminator', () => {
    const o = K(1);
    disc('craft', 'craft', h.craftItem(o, { recipeId: 1, templateId: 7, materialIds: [2], treasury: K(2), seasonPool: K(3), itemsMinted: 0n, wear: true }));
    disc('craft', 'repair', h.repairItem(o, { recipeId: 2, item: K(4), itemMint: K(5), materialIds: [2], treasury: K(2), seasonPool: K(3) }));
    disc('book', 'place', h.bookPlace(o, { baseMint: K(6), treasury: K(2), side: 0, price: 1n, size: 1n, postOnly: false, expiresAt: 0n, makers: [K(7)] }));
    disc('book', 'cancel', h.bookCancel(o, K(6), 1n));
    disc('book', 'crank', h.bookCrank(o, K(6), 1, [K(7)]));
    disc('book', 'create_market', h.bookCreateMarket(o, 2, 1n, 1n));
    disc('book', 'place_class_bid', h.bookPlaceClassBid(o, 1n, { templateId: 1, minLevel: 0, paramMin: Array(11).fill(0), paramMax: Array(11).fill(0) }, 1n, 0n));
    disc('book', 'cancel_class_bid', h.bookCancelClassBid(o, 1n));
    disc('book', 'match_class', h.bookMatchClass(o, { bid: K(8), bidder: K(9), item: K(10), itemMint: K(11), treasury: K(2) }));
    disc('items', 'reslot_loyalty', h.reslotLoyalty(K(6), 0, K(7), 1, K(8)));
    disc('market', 'end_lease', h.marketEndLease(o, K(7), K(8), K(9)));
    disc('market', 'buy', h.marketBuyEco(o, K(2), K(3), K(4), K(5), K(6), 1n));
    const step = h.prepareLaunch(companionCreatorAddress(K(6)), K(6), { name: 'n', symbol: 's', uri: '', creatorFeeBps: 0, rules: launchRulesFromInput(NO_RULES), slots: [] });
    const wrapped = h.companionLaunchSlots(o, K(6), step);
    disc('companion', 'launch_slots', wrapped);
    expect(wrapped.keys.slice(count('companion', 'launch_slots')).filter((k) => k.isSigner).map((k) => k.pubkey.toBase58())).toEqual([K(6).toBase58()]);
  });
  it('book makers follow the program: best first, expired skipped, at most match_max, then the evicted owner', () => {
    const o = (id: number, owner: PublicKey, price: bigint, size: bigint, expiresAt = 0n) => ({ id: BigInt(id), owner, price, size, expiresAt });
    const m = { asks: [o(1, K(1), 10n, 1n), o(2, K(2), 10n, 1n, 5n), o(3, K(3), 11n, 1n), o(4, K(4), 12n, 1n)], bids: [o(5, K(5), 9n, 1n)], minSize: 1n };
    expect(h.bookMakers(m, { slots: 1, matchMax: 2 }, 0, 12n, 5n, 10n).map((k) => k.toBase58())).toEqual([K(1), K(3), K(5)].map((k) => k.toBase58()));
    expect(h.bookMakers(m, { slots: 4, matchMax: 8 }, 1, 9n, 1n, 10n).map((k) => k.toBase58())).toEqual([K(5).toBase58()]);
    expect(h.bookCrankOwners(m, 8, 10n).map((k) => k.toBase58())).toEqual([K(2).toBase58()]);
  });
});
