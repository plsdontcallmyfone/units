/**
 * Every agents, market, social and arsenal-payout prepare, built against a mocked chain holding
 * accounts encoded with the IDL codecs (nothing deployed): each returns one instruction of the right
 * program with the IDL's discriminator, exactly the IDL's account list (plus the forwarded armory
 * accounts for `equip_badge`), and the accounts the route read from the chain in the named places.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { PublicKey, type AccountInfo, type TransactionInstruction } from '@solana/web3.js';
import { FIXED_ADDRESSES } from '@hookwars/shared';
import { hookwars } from '@hookwars/sdk';
import { PREPARES } from './prepares.ts';

const K = (n: number) => new PublicKey(Buffer.alloc(32, n));
const S = (k: PublicKey) => k.toBase58();
const zeroOf = (program: string, name: string) => hookwars.coderOf(program).decodeAccount<Record<string, unknown>>(name, Buffer.concat([hookwars.coderOf(program).accountDisc(name), Buffer.alloc(4000)]));
const enc = (program: string, name: string, over: Record<string, unknown>) => hookwars.coderOf(program).encodeAccount(name, { ...zeroOf(program, name), ...over });

const owner = K(3), mint = K(2), itemMint = K(10), seller = K(11), author = K(12), treasury = K(13), lessor = K(14);
const commission = K(20), tokenMint = K(21), creator = K(22), subItem = K(23), submitter = K(24);
const passport = K(30), agentKey = K(31), badgeMint = K(32), soulbound = K(33), feeCollector = K(34), bond = K(35);
const buyer = K(40), referrer = K(41), equipped = K(42), pool = K(43), to = K(44);
const item = hookwars.itemAddress(itemMint);

const itemData = (templateId: number) => hookwars.itemCodec().encode({
  version: 1, bump: 255, itemMint, templateId, params: Array(hookwars.PARAM_FIELDS).fill(0),
  manifest: { kind: 0, tokenFlags: 0, poolFlags: 0, maxCutBuyBps: 0, maxCutSellBps: 0, maxCutTransferBps: 0, maxDiscountBps: 0, mayRefuse: false, mayBurn: false, dataBytes: 0, readsOtherPools: 0 },
  author, royaltyBps: 0, level: 1, source: 0, equippedCount: 0, royaltyOwnerBump: 255, createdAt: 0n, hasWear: false, accessMode: 0, exclusive: false, reserved: Buffer.alloc(29),
});
const launch = Buffer.alloc(400); pool.toBuffer().copy(launch, 74);

const accounts = new Map<string, Buffer>([
  [S(item), itemData(1)], [S(equipped), itemData(24)],
  [S(hookwars.listingAddress(itemMint)), enc('market', 'Listing', { seller, item, itemMint, priceLamports: 5n })],
  [S(hookwars.marketConfigAddress()), enc('market', 'MarketConfig', { treasury, collections: 7 })],
  [S(hookwars.leaseAddress(item)), enc('market', 'Lease', { lessor, item, itemMint, tokenMint })],
  [S(commission), enc('market', 'Commission', { creator, tokenMint })],
  [S(hookwars.submissionAddress(commission, subItem)), enc('market', 'Submission', { commission, item: subItem, submitter })],
  [S(hookwars.socialConfigAddress()), enc('social', 'SocialConfig', { guilds: 4 })],
  [S(hookwars.guildAddress(4)), enc('social', 'Guild', { id: 4, actions: 2n })],
  [S(hookwars.guildActionAddress(4, 1n)), enc('social', 'GuildAction', { guildId: 4, nonce: 1n, kind: { name: 'SpendSol', to, lamports: 9n } })],
  [S(hookwars.agentsConfigAddress()), enc('agents', 'AgentsConfig', { feeCollector, soulboundItem: soulbound })],
  [S(passport), enc('agents', 'Passport', { operator: owner, agentKey, badgeMint, badgeIssued: false })],
  [S(bond), enc('agents', 'Bond', { passport, proposalA: K(50), proposalB: K(51), mintA: K(52), mintB: K(53), treatyItem: K(54) })],
  [S(hookwars.referredAddress(mint, buyer)), enc('items', 'Referred', { referrer })],
  [S(hookwars.equipStateAddress(mint, 1)), enc('items', 'EquipState', { item: equipped })],
  [S(hookwars.launchAddr(mint)), launch],
]);
const info = (k: PublicKey): AccountInfo<Buffer> | null => {
  const data = accounts.get(S(k));
  return data ? { data, owner: hookwars.ARMORY_ID, lamports: 1, executable: false, rentEpoch: 0 } : null;
};
const conn = { getAccountInfo: async (k: PublicKey) => info(k), getMultipleAccountsInfo: async (ks: PublicKey[]) => ks.map(info) } as never;

const PROGRAM: Record<string, PublicKey> = { market: hookwars.MARKET_ID, social: hookwars.SOCIAL_ID, agents: hookwars.AGENTS_ID, items: hookwars.ITEMS_ID };

/** Builds one route and checks program, discriminator and account count; returns the accounts by IDL name. */
async function built(route: string, program: string, ixName: string, body: Record<string, unknown>, extra = 0): Promise<Record<string, string>> {
  const ixs = await PREPARES[`${route}/prepare`]!.build({ owner: S(owner), ...body }, conn);
  expect(ixs).toHaveLength(1);
  const ix = ixs[0] as TransactionInstruction;
  expect(S(ix.programId)).toBe(S(PROGRAM[program]!));
  const coder = hookwars.coderOf(program);
  expect([...ix.data.subarray(0, 8)]).toEqual(coder.instruction(ixName).discriminator);
  const names = coder.accountsOf(ixName).map((a) => a.name);
  expect(ix.keys.length).toBe(names.length + extra);
  return Object.fromEntries(names.map((n, i) => [n, S(ix.keys[i]!.pubkey)]));
}

describe('market prepares', () => {
  it('list, delist and buy read the listing, the config and the item author', async () => {
    const l = await built('market/list', 'market', 'list', { itemMint: S(itemMint), priceLamports: '1000', expiresAt: 0 });
    expect(l.item).toBe(S(item));
    const d = await built('market/delist', 'market', 'delist', { itemMint: S(itemMint) });
    expect(d.seller).toBe(S(seller));
    const b = await built('market/buy', 'market', 'buy', { itemMint: S(itemMint) });
    expect([b.seller, b.author, b.treasury]).toEqual([S(seller), S(author), S(treasury)]);
  });
  it('a collection takes the next id from the config and refuses a bad template list', async () => {
    // Each template's account follows, in order (the program checks one per id; devnet drill).
    const c = await built('market/collections', 'market', 'create_collection', { name: 'Raiders', templateIds: [1, 9] }, 2);
    expect(c.collection).toBe(S(hookwars.collectionAddress(7)));
    const [ix] = await PREPARES['market/collections/prepare']!.build({ owner: S(owner), name: 'Raiders', templateIds: [1, 9] }, conn);
    expect(ix!.keys.slice(-2).map((k) => S(k.pubkey))).toEqual([S(hookwars.templateAddress(1)), S(hookwars.templateAddress(9))]);
    await expect(PREPARES['market/collections/prepare']!.build({ owner: S(owner), name: 'x', templateIds: [0] }, conn)).rejects.toThrow(/templateIds/);
  });
  it('lease offer, accept, withdraw and end name the lessor from the lease', async () => {
    await built('market/lease/offer', 'market', 'offer_lease', { itemMint: S(itemMint), tokenMint: S(tokenMint), slot: 1, rentBps: 100, feeLamports: '1', termSecs: 3600 });
    const a = await built('market/lease/accept', 'market', 'accept_lease', { itemMint: S(itemMint) });
    expect(a.lessor).toBe(S(lessor));
    expect((await built('market/lease/withdraw', 'market', 'withdraw_offer', { itemMint: S(itemMint) })).lessor).toBe(S(lessor));
    expect((await built('market/lease/end', 'market', 'end_lease', { itemMint: S(itemMint) }, 1)).lessor).toBe(S(lessor));
    // Review 3 M-5: with the slot no longer holding the item, the token mint alone follows the named accounts.
    const [end] = await PREPARES['market/lease/end/prepare']!.build({ owner: S(owner), itemMint: S(itemMint) }, conn);
    expect(end!.keys.slice(hookwars.coderOf('market').accountsOf('end_lease').length).map((k) => S(k.pubkey))).toEqual([S(tokenMint)]);
  });
  it('commission open, submit, pay and refund', async () => {
    const o = await built('commissions/open', 'market', 'open_commission', { tokenMint: S(tokenMint), nonce: '1', slot: 1, briefUri: 'https://x', bountyLamports: '100', windowSecs: 60 });
    expect(o.commission).toBe(S(hookwars.commissionAddress(tokenMint, 1n)));
    expect((await built('commissions/submit', 'market', 'submit', { commission: S(commission), itemMint: S(itemMint) })).item).toBe(S(item));
    expect((await built('commissions/pay', 'market', 'pay_commission', { commission: S(commission), item: S(subItem) })).submitter).toBe(S(submitter));
    expect((await built('commissions/refund', 'market', 'refund_commission', { commission: S(commission) })).creator).toBe(S(creator));
    await expect(PREPARES['commissions/pay/prepare']!.build({ owner: S(owner), commission: S(commission), item: S(K(99)) }, conn)).rejects.toThrow(/not submitted/);
  });
});

describe('social prepares', () => {
  it('badge claim defaults the recipient to the wallet', async () => {
    const c = await built('badges/claim', 'social', 'claim_badge', { badgeId: 2 });
    expect(c.recipient).toBe(S(owner));
  });
  it('guild create, deposit, propose, approve and execute', async () => {
    expect((await built('guilds/create', 'social', 'create_guild', { name: 'Hall' })).guild).toBe(S(hookwars.guildAddress(4)));
    await built('guilds/deposit', 'social', 'deposit_sol', { guildId: 4, lamports: '10' });
    const p = await built('guilds/propose', 'social', 'propose_action', { guildId: 4, kind: { name: 'SpendSol', to: S(to), lamports: '9' } });
    expect(p.action).toBe(S(hookwars.guildActionAddress(4, 2n)));
    await built('guilds/approve', 'social', 'approve_action', { guildId: 4, nonce: '1' });
    expect((await built('guilds/execute', 'social', 'execute_action', { guildId: 4, nonce: '1' })).to).toBe(S(to));
    await expect(PREPARES['guilds/propose/prepare']!.build({ owner: S(owner), guildId: 4, kind: { name: 'Mint' } }, conn)).rejects.toThrow(/Unknown guild action/);
  });
});

describe('agents prepares', () => {
  it('register takes the next index and derives the badge mint', async () => {
    const agentKey = K(77);
    const r = await built('agents/register', 'agents', 'register_passport', { name: 'scout', kinds: 3, agentKey: S(agentKey) });
    const p = hookwars.passportAddress(owner, 0);
    expect([r.passport, r.fee_collector, r.badge_mint, r.agent_key]).toEqual([S(p), S(feeCollector), S(hookwars.agentBadgeMintAddress(p, 0)), S(agentKey)]);
    // The agent key must be its own key: the program refuses the operator's (KeyIsOperator).
    await expect(PREPARES['agents/register/prepare']!.build({ owner: S(owner), name: 'scout', kinds: 3 }, conn)).rejects.toThrow(/agentKey/);
    await expect(PREPARES['agents/register/prepare']!.build({ owner: S(owner), name: 'scout', kinds: 3, agentKey: S(owner) }, conn)).rejects.toThrow(/agentKey/);
  });
  it('profile, policy and withdraw', async () => {
    await built('agents/profile', 'agents', 'update_profile', { passport: S(passport), name: 'scout', kinds: 1 });
    await built('agents/policy/init', 'agents', 'init_policy', { passport: S(passport), perActionLamports: '1', perDayLamports: '2', tracked: [{ mint: S(mint), perAction: '1', perDay: '2' }] });
    await built('agents/policy/limits', 'agents', 'set_limits', { passport: S(passport), perActionLamports: '1', perDayLamports: '2' });
    await built('agents/policy/freeze', 'agents', 'freeze_policy', { passport: S(passport), frozen: true });
    await built('agents/policy/withdraw', 'agents', 'withdraw', { passport: S(passport), amount: '5' });
  });
  it('bonds: post and resolve read the bond and its passport', async () => {
    await built('agents/bonds/post', 'agents', 'post_bond', { passport: S(passport), proposalA: S(K(50)), proposalB: S(K(51)), treatyItem: S(K(54)) });
    const r = await built('agents/bonds/resolve', 'agents', 'resolve_bond', { bond: S(bond) });
    expect(r.passport).toBe(S(passport));
  });
  it('badge: equip forwards the armory equip of the Soulbound item, issue mints to the agent key', async () => {
    const ixs = await PREPARES['agents/badge/equip/prepare']!.build({ owner: S(owner), passport: S(passport) }, conn);
    const ix = ixs[0]!;
    const caller = hookwars.agentsArmoryCaller(badgeMint);
    expect(ix.keys.some((k) => k.pubkey.equals(caller) && !k.isSigner)).toBe(true);
    expect(ix.keys.some((k) => k.pubkey.equals(soulbound))).toBe(true);
    expect(ix.keys.some((k) => k.pubkey.equals(hookwars.armoryCallerAddress(badgeMint)))).toBe(false);
    const i = await built('agents/badge/issue', 'agents', 'issue_badge', { passport: S(passport) });
    expect([i.agent_key, i.badge_mint, i.badge_holding]).toEqual([S(agentKey), S(badgeMint), S(hookwars.holdingAddr(badgeMint, agentKey))]);
  });
});

describe('arsenal payout prepares', () => {
  it('referral set and settle', async () => {
    await built('referral/set', 'items', 'set_referrer', { mint: S(mint), referrer: S(referrer) });
    const s = await built('referral/settle', 'items', 'settle_referral', { mint: S(mint), buyer: S(buyer), slot: 1 });
    expect(s.referrer_quote).toBe(S(hookwars.holdingAddr(new PublicKey(FIXED_ADDRESSES.bridgedSolMint), referrer)));
    await expect(PREPARES['referral/settle/prepare']!.build({ owner: S(owner), mint: S(mint), buyer: S(K(98)), slot: 1 }, conn)).rejects.toThrow(/named no referrer/);
  });
  it('loyalty init and claim, first blood init', async () => {
    await built('loyalty/init', 'items', 'init_loyalty', { mint: S(mint), slot: 1 });
    const c = await built('loyalty/claim', 'items', 'claim_loyalty', { mint: S(mint), slot: 1 });
    expect(c.item).toBe(S(equipped));
    expect(c.pool_token).toBe(S(hookwars.holdingAddr(mint, pool)));
    expect(c.composite).toBe(S(hookwars.ITEMS_ID)); // not a composite: the program id stands in
    await built('first-blood/init', 'items', 'init_first_blood', { mint: S(mint) });
  });
  it('every route the site posts to exists', () => {
    const files = (dir: string): string[] => readdirSync(dir).flatMap((f) => {
      const p = join(dir, f);
      return statSync(p).isDirectory() ? (f === 'node_modules' || f === '.next' ? [] : files(p)) : f.endsWith('.tsx') ? [p] : [];
    });
    const web = join(__dirname, '../../web');
    const routes = new Set<string>();
    for (const f of [...files(join(web, 'app')), ...files(join(web, 'components'))]) {
      for (const m of readFileSync(f, 'utf8').matchAll(/route=(?:"([^"]+)"|\{([^}]*)\})/g)) {
        if (m[1]) routes.add(m[1]);
        for (const q of (m[2] ?? '').matchAll(/'([a-z/-]+)'/g)) routes.add(q[1]!);
      }
    }
    expect(routes.size).toBeGreaterThan(40);
    for (const r of routes) expect(PREPARES[`${r}/prepare`], r).toBeDefined();
  });
});
