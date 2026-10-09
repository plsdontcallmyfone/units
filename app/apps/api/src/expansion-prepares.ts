// Changed by Hookwars: new file, prepares for the agents, market and social programs and the
// arsenal payouts (docs/spec/08, 09, 10). Each builds one instruction from the generated IDLs
// (`@hookwars/sdk` expansion builders) after reading the accounts it needs from the chain, then
// goes through the same `finish` as every other prepare: simulate, v0 with the protocol table and
// the mint's own, refuse anything over a packet.
import type { Connection } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { big, int, keys, pk, PrepareError, type Body, type PrepareDef } from './prepares.ts';

const str = (b: Body, k: string, max: number, optional = false): string => {
  const v = b[k];
  if (v === undefined && optional) return '';
  if (typeof v !== 'string' || v.length > max) throw new PrepareError(400, 'BadRequest', `"${k}" must be text of at most ${max} characters.`);
  return v;
};
const bool = (b: Body, k: string): boolean => b[k] === true || b[k] === 'true';
const optPk = (b: Body, k: string): PublicKey | null => (b[k] === undefined || b[k] === null || b[k] === '' ? null : pk(b, k));

async function account<T>(conn: Connection, address: PublicKey, decode: (d: Buffer) => T, missing: string): Promise<T> {
  const info = await conn.getAccountInfo(address, 'confirmed');
  if (!info) throw new PrepareError(409, 'NotFound', missing);
  return decode(info.data);
}

/** The armory `Item` behind an item mint (its author and the item key). */
async function itemOf(conn: Connection, itemMint: PublicKey) {
  const item = hookwars.itemAddress(itemMint);
  const data = await account(conn, item, (d) => hookwars.itemCodec().decode(d) as { author: PublicKey; templateId: number }, 'No armory item has this mint.');
  return { item, ...data };
}

const one = (label: string, programs: PrepareDef['programs'], build: PrepareDef['build']): PrepareDef => ({ programs, label, payer: (b) => pk(b, 'owner'), build });

export const EXPANSION_PREPARES: Record<string, PrepareDef> = {
  // ---------------------------------------------------------------- market (10) --
  'market/list/prepare': one('List item', ['market', 'armory', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const itemMint = pk(b, 'itemMint');
    const { item } = await itemOf(conn, itemMint);
    return [hookwars.marketList(owner, item, itemMint, big(b, 'priceLamports'), BigInt(int(b, 'expiresAt', 0, 2 ** 40)))];
  }),
  'market/delist/prepare': one('Delist item', ['market', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const itemMint = pk(b, 'itemMint');
    const l = await account(conn, hookwars.listingAddress(itemMint), (d) => hookwars.listingCodec.decode(d), 'This item is not listed.');
    return [hookwars.marketDelist(owner, l.seller, itemMint)];
  }),
  'market/buy/prepare': one('Buy item', ['market', 'armory', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const itemMint = pk(b, 'itemMint');
    const l = await account(conn, hookwars.listingAddress(itemMint), (d) => hookwars.listingCodec.decode(d), 'This item is not listed.');
    const cfg = await account(conn, hookwars.marketConfigAddress(), (d) => hookwars.marketConfigCodec.decode(d), 'The market has no config on this cluster yet.');
    const { item, author } = await itemOf(conn, itemMint);
    const max = b.maxPrice === undefined ? l.priceLamports : big(b, 'maxPrice');
    return [hookwars.marketBuy(owner, l.seller, author, cfg.treasury, item, itemMint, max)];
  }),
  'market/collections/prepare': one('Create collection', ['market'], async (b, conn) => {
    const cfg = await account(conn, hookwars.marketConfigAddress(), (d) => hookwars.marketConfigCodec.decode(d), 'The market has no config on this cluster yet.');
    const ids = b.templateIds;
    if (!Array.isArray(ids) || ids.length === 0 || ids.length > 255 || ids.some((x) => !Number.isInteger(x) || x < 1 || x > 65_535)) throw new PrepareError(400, 'BadRequest', '"templateIds" must be a list of template ids.');
    return [hookwars.marketCreateCollection(pk(b, 'owner'), cfg.collections, str(b, 'name', 64), ids as number[])];
  }),
  'market/lease/offer/prepare': one('Offer a lease', ['market', 'armory', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const itemMint = pk(b, 'itemMint');
    const { item } = await itemOf(conn, itemMint);
    return [hookwars.marketOfferLease(owner, item, itemMint, pk(b, 'tokenMint'), int(b, 'slot', 0, 7), int(b, 'rentBps', 0, 10_000), big(b, 'feeLamports'), int(b, 'termSecs', 1, 4_294_967_295))];
  }),
  'market/lease/accept/prepare': one('Accept a lease', ['market'], async (b, conn) => {
    const itemMint = pk(b, 'itemMint'); const { item } = await itemOf(conn, itemMint);
    const lease = await account(conn, hookwars.leaseAddress(item), (d) => hookwars.leaseCodec.decode(d), 'There is no lease offer for this item.');
    return [hookwars.marketAcceptLease(pk(b, 'owner'), item, lease.lessor)];
  }),
  'market/lease/withdraw/prepare': one('Withdraw a lease offer', ['market', 'token'], async (b, conn) => leaseClose(conn, b, 'withdraw_offer')),
  'market/lease/end/prepare': one('End a lease', ['market', 'token'], async (b, conn) => leaseClose(conn, b, 'end_lease')),
  'commissions/open/prepare': one('Open a commission', ['market'], async (b) => [hookwars.marketOpenCommission(
    pk(b, 'owner'), pk(b, 'tokenMint'), big(b, 'nonce'), int(b, 'slot', 0, 7), str(b, 'briefUri', 200), big(b, 'bountyLamports'), int(b, 'windowSecs', 1, 4_294_967_295),
  )]),
  'commissions/submit/prepare': one('Submit to a commission', ['market', 'armory'], async (b, conn) => {
    const commission = pk(b, 'commission'); const itemMint = pk(b, 'itemMint');
    const c = await account(conn, commission, (d) => hookwars.commissionCodec.decode(d), 'No such commission.');
    const { item } = await itemOf(conn, itemMint);
    return [hookwars.marketSubmit(pk(b, 'owner'), commission, c.tokenMint, item, itemMint)];
  }),
  'commissions/pay/prepare': one('Pay a commission', ['market'], async (b, conn) => {
    const commission = pk(b, 'commission');
    const c = await account(conn, commission, (d) => hookwars.commissionCodec.decode(d), 'No such commission.');
    const item = pk(b, 'item');
    const sub = await account(conn, hookwars.submissionAddress(commission, item), (d) => hookwars.submissionCodec.decode(d), 'That item was not submitted to this commission.');
    return [hookwars.marketPayCommission(commission, c.tokenMint, item, sub.submitter)];
  }),
  'commissions/refund/prepare': one('Refund a commission', ['market'], async (b, conn) => {
    const commission = pk(b, 'commission');
    const c = await account(conn, commission, (d) => hookwars.commissionCodec.decode(d), 'No such commission.');
    return [hookwars.marketRefundCommission(commission, c.creator)];
  }),

  // ---------------------------------------------------------------- social (10) --
  'badges/claim/prepare': one('Claim a badge', ['social', 'token'], async (b) => [hookwars.socialClaimBadge(pk(b, 'owner'), int(b, 'badgeId', 0, 4_294_967_295), optPk(b, 'recipient') ?? pk(b, 'owner'))]),
  'guilds/create/prepare': one('Found a guild', ['social'], async (b, conn) => {
    const cfg = await account(conn, hookwars.socialConfigAddress(), (d) => hookwars.socialConfigCodec.decode(d), 'Social has no config on this cluster yet.');
    return [hookwars.socialCreateGuild(pk(b, 'owner'), cfg.guilds, str(b, 'name', 32))];
  }),
  'guilds/deposit/prepare': one('Deposit to a guild', ['social'], async (b) => [hookwars.socialDeposit(pk(b, 'owner'), int(b, 'guildId', 0, 4_294_967_295), big(b, 'lamports'))]),
  'guilds/propose/prepare': one('Propose a guild action', ['social'], async (b, conn) => {
    const id = int(b, 'guildId', 0, 4_294_967_295);
    const g = await account(conn, hookwars.guildAddress(id), (d) => hookwars.guildCodec.decode(d), 'No such guild.');
    const kind = b.kind as Record<string, unknown> | undefined;
    if (!kind || typeof kind.name !== 'string') throw new PrepareError(400, 'BadRequest', '"kind" must be an action such as { "name": "SpendSol", "to": "...", "lamports": "..." }.');
    const k: Record<string, unknown> = { name: kind.name };
    if (kind.name === 'SpendSol') { k.to = pk(kind, 'to'); k.lamports = big(kind, 'lamports'); }
    else if (kind.name === 'SpendToken') { k.mint = pk(kind, 'mint'); k.to = pk(kind, 'to'); k.amount = big(kind, 'amount'); }
    else if (kind.name === 'SetOfficers') { k.officers = keys(kind, 'officers', 32); k.threshold = int(kind, 'threshold', 1, 32); }
    else throw new PrepareError(400, 'BadRequest', 'Unknown guild action.');
    return [hookwars.socialProposeAction(pk(b, 'owner'), id, g.actions, k)];
  }),
  'guilds/approve/prepare': one('Approve a guild action', ['social'], async (b) => [hookwars.socialApproveAction(pk(b, 'owner'), int(b, 'guildId', 0, 4_294_967_295), big(b, 'nonce'))]),
  'guilds/execute/prepare': one('Execute a guild action', ['social', 'token'], async (b, conn) => {
    const id = int(b, 'guildId', 0, 4_294_967_295); const nonce = big(b, 'nonce');
    const a = await account(conn, hookwars.guildActionAddress(id, nonce), (d) => hookwars.guildActionCodec.decode(d), 'No such guild action.');
    const kind = a.kind as unknown as { name: string; to?: PublicKey };
    const to = kind.to ?? pk(b, 'owner');
    return [hookwars.socialExecuteAction(id, nonce, to)];
  }),

  // ---------------------------------------------------------------- agents (09) --
  'agents/register/prepare': one('Register an agent passport', ['agents', 'token'], async (b, conn) => {
    const owner = pk(b, 'owner'); const agentKey = optPk(b, 'agentKey') ?? owner;
    const cfg = await account(conn, hookwars.agentsConfigAddress(), (d) => hookwars.agentsConfigCodec.decode(d), 'Agents has no config on this cluster yet.');
    const idx = await conn.getAccountInfo(hookwars.operatorIndexAddress(owner), 'confirmed');
    const index = idx ? hookwars.operatorIndexCodec.decode(idx.data).next : 0;
    return [hookwars.agentsRegisterPassport(owner, agentKey, owner, cfg.feeCollector, index, profile(b))];
  }),
  'agents/profile/prepare': one('Update an agent profile', ['agents', 'token'], async (b, conn) => {
    const passport = pk(b, 'passport');
    const p = await account(conn, passport, (d) => hookwars.passportCodec.decode(d), 'No such passport.');
    return [hookwars.agentsUpdateProfile(pk(b, 'owner'), passport, p.badgeMint, profile(b))];
  }),
  'agents/policy/init/prepare': one('Set up an agent wallet', ['agents'], async (b) => [hookwars.agentsInitPolicy(pk(b, 'owner'), pk(b, 'owner'), pk(b, 'passport'), limits(b))]),
  'agents/policy/limits/prepare': one('Change agent wallet limits', ['agents'], async (b) => [hookwars.agentsSetLimits(pk(b, 'owner'), pk(b, 'passport'), limits(b))]),
  'agents/policy/freeze/prepare': one('Freeze or unfreeze an agent wallet', ['agents'], async (b) => [hookwars.agentsFreezePolicy(pk(b, 'owner'), pk(b, 'passport'), bool(b, 'frozen'))]),
  'agents/policy/withdraw/prepare': one('Withdraw from an agent wallet', ['agents', 'token'], async (b) => [hookwars.agentsWithdraw(pk(b, 'owner'), pk(b, 'passport'), big(b, 'amount'), optPk(b, 'mint'))]),
  'agents/bonds/post/prepare': one('Post a diplomat bond', ['agents', 'armory'], async (b) => [hookwars.agentsPostBond(pk(b, 'owner'), pk(b, 'passport'), pk(b, 'proposalA'), pk(b, 'proposalB'), pk(b, 'treatyItem'))]),
  'agents/bonds/resolve/prepare': one('Resolve a diplomat bond', ['agents', 'armory'], async (b, conn) => {
    const bond = pk(b, 'bond');
    const d = await account(conn, bond, (x) => hookwars.bondCodec.decode(x), 'No such bond.');
    const p = await account(conn, d.passport, (x) => hookwars.passportCodec.decode(x), 'The bond names no passport.');
    return [hookwars.agentsResolveBond(bond, d.passport, p.agentKey, d.proposalA, d.proposalB, d.mintA, d.mintB)];
  }),

  // ---------------------------------------------------------------- arsenal payouts (08) --
  'referral/set/prepare': one('Name a referrer', ['items'], async (b) => [hookwars.itemsSetReferrer(pk(b, 'owner'), pk(b, 'mint'), pk(b, 'referrer'))]),
  'referral/settle/prepare': one('Pay a referrer', ['items', 'armory', 'token'], async (b, conn) => {
    const mint = pk(b, 'mint'); const buyer = pk(b, 'buyer'); const slot = int(b, 'slot', 0, 7);
    const r = await account(conn, hookwars.referredAddress(mint, buyer), (d) => hookwars.referredCodec.decode(d), 'This buyer named no referrer.');
    const st = await account(conn, hookwars.equipStateAddress(mint, slot), (d) => hookwars.equipStateCodec.decode(d), 'Nothing is equipped in this slot.');
    return [hookwars.itemsSettleReferral(pk(b, 'owner'), mint, buyer, slot, st.item, r.referrer)];
  }),
  'loyalty/init/prepare': one('Open the loyalty pot', ['items'], async (b) => [hookwars.itemsInitLoyalty(pk(b, 'owner'), pk(b, 'mint'), int(b, 'slot', 0, 7))]),
  'first-blood/init/prepare': one('Open First Blood', ['items'], async (b) => [hookwars.itemsInitFirstBlood(pk(b, 'owner'), pk(b, 'mint'))]),
  'loyalty/claim/prepare': one('Claim from the loyalty pot', ['items', 'armory', 'token', 'launch'], async (b, conn) => {
    const mint = pk(b, 'mint'); const slot = int(b, 'slot', 0, 7);
    const st = await account(conn, hookwars.equipStateAddress(mint, slot), (d) => hookwars.equipStateCodec.decode(d), 'Nothing is equipped in this slot.');
    const item = await account(conn, st.item, (d) => hookwars.itemCodec().decode(d) as { templateId: number }, 'The equipped item no longer exists.');
    const launch = await conn.getAccountInfo(hookwars.launchAddr(mint), 'confirmed');
    if (!launch) throw new PrepareError(409, 'NoLaunch', 'This token has no launch.');
    const pool = new PublicKey(launch.data.subarray(74, 106));
    return [hookwars.itemsClaimLoyalty(pk(b, 'owner'), mint, slot, st.item, item.templateId === 41, pool)];
  }),
};

async function leaseClose(conn: Connection, b: Body, name: 'withdraw_offer' | 'end_lease') {
  const itemMint = pk(b, 'itemMint'); const { item } = await itemOf(conn, itemMint);
  const lease = await account(conn, hookwars.leaseAddress(item), (d) => hookwars.leaseCodec.decode(d), 'There is no lease for this item.');
  return [hookwars.marketCloseLease(name, pk(b, 'owner'), item, itemMint, lease.lessor)];
}

function profile(b: Body): hookwars.PassportArgsInput {
  return {
    name: str(b, 'name', 32), avatarUri: str(b, 'avatarUri', 200, true), bioUri: str(b, 'bioUri', 200, true), hireUri: str(b, 'hireUri', 200, true),
    kinds: int(b, 'kinds', 0, 255), creditAgentId: null,
  };
}

function limits(b: Body): hookwars.PolicyLimitsInput {
  const tracked = b.tracked;
  if (tracked !== undefined && (!Array.isArray(tracked) || tracked.length > 8)) throw new PrepareError(400, 'BadRequest', '"tracked" must be a list of at most 8 mints with their limits.');
  return {
    perActionLamports: big(b, 'perActionLamports'), perDayLamports: big(b, 'perDayLamports'),
    tracked: (tracked as Body[] | undefined ?? []).map((t) => ({ mint: pk(t, 'mint'), perAction: big(t, 'perAction'), perDay: big(t, 'perDay') })),
    targets: keys(b, 'targets', 8),
  };
}
