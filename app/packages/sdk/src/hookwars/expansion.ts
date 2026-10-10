// Changed by Hookwars: new file, instruction builders for the agents, market and social programs and
// the arsenal payouts (docs/spec/08, 09, 10), all through the generated IDLs (`idlIx`): exact
// discriminators, argument encoding and account order. Accounts whose seeds read another account's
// data (for example `lease.item`) are passed explicitly; the rest are derived from the IDL seeds.
import { PublicKey, type TransactionInstruction } from '@solana/web3.js';
import { FIXED_ADDRESSES } from '@hookwars/shared';
import { idlIx } from './from-idl.ts';
import { equipLaunch } from './slot-launch.ts';
import {
  ITEMS_ID, TOKEN_ID, agentsConfigAddress, collectionAddress, agentVaultAddress, armoryConfigAddress, badgeMintAddress, badgeMinterAddress, commissionAddress,
  commissionVaultAddress, equipStateAddress, firstBloodAddress, guildActionAddress, guildAddress, guildTreasuryAddress, holdingAddr,
  leaseAddress, leaseEscrowAddress, listingAddress, loyaltyClaimAddress, loyaltyPotAddress, marketEscrowAddress, passportAddress,
  AGENTS_ID, armoryCallerAddress, referralVaultOwner, referredAddress, submissionAddress, treatyInboxAddress, agentsSignerAddress, agentBadgeMintAddress, compositeAddress, launchAddr, templateAddress,
} from './addresses.ts';

const TOKEN_EVENT_AUTHORITY = new PublicKey(FIXED_ADDRESSES.tokenEventAuthority);
const QUOTE = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
const tok = { tokenProgram: TOKEN_ID, tokenEventAuthority: TOKEN_EVENT_AUTHORITY };

// ---------------------------------------------------------------- market (10 sections 3 to 7) --

/** `list(price_lamports, expires_at)`: the item goes into the market's escrow. */
export function marketList(seller: PublicKey, item: PublicKey, itemMint: PublicKey, priceLamports: bigint, expiresAt: bigint): TransactionInstruction {
  const escrow = marketEscrowAddress(itemMint);
  return idlIx('market', 'list', { seller, item, itemMint, sellerHolding: holdingAddr(itemMint, seller), escrow, escrowHolding: holdingAddr(itemMint, escrow), ...tok }, { priceLamports, expiresAt });
}
/** `delist`: the seller (or anyone after expiry, per the program) returns the item from escrow. */
export function marketDelist(caller: PublicKey, seller: PublicKey, itemMint: PublicKey): TransactionInstruction {
  const escrow = marketEscrowAddress(itemMint);
  return idlIx('market', 'delist', { caller, seller, listing: listingAddress(itemMint), itemMint, escrow, escrowHolding: holdingAddr(itemMint, escrow), sellerHolding: holdingAddr(itemMint, seller), ...tok });
}
/** `buy(max_price)`: pays protocol fee, author share and seller, moves the item to the buyer. */
export function marketBuy(buyer: PublicKey, seller: PublicKey, author: PublicKey, treasury: PublicKey, item: PublicKey, itemMint: PublicKey, maxPrice: bigint): TransactionInstruction {
  const escrow = marketEscrowAddress(itemMint);
  return idlIx('market', 'buy', { buyer, seller, author, treasury, item, itemMint, escrow, escrowHolding: holdingAddr(itemMint, escrow), buyerHolding: holdingAddr(itemMint, buyer), ...tok }, { maxPrice });
}
/** `create_collection(name, template_ids)`; `nextId` is `MarketConfig.collections`. */
export function marketCreateCollection(curator: PublicKey, nextId: number, name: string, templateIds: number[]): TransactionInstruction {
  // Each template's account, in order, follows (the program checks one per id; found on the devnet drill).
  return idlIx('market', 'create_collection', { curator, collection: collectionAddress(nextId) }, { name, templateIds }, templateIds.map((id) => ({ pubkey: templateAddress(id), isSigner: false, isWritable: false })));
}
/** `offer_lease(token_mint, slot, rent_bps, fee_lamports, term_secs)`. */
export function marketOfferLease(lessor: PublicKey, item: PublicKey, itemMint: PublicKey, tokenMint: PublicKey, slot: number, rentBps: number, feeLamports: bigint, termSecs: number): TransactionInstruction {
  const leaseEscrow = leaseEscrowAddress(itemMint);
  return idlIx('market', 'offer_lease', { lessor, item, itemMint, lessorHolding: holdingAddr(itemMint, lessor), tokenMint, leaseEscrow, leaseEscrowHolding: holdingAddr(itemMint, leaseEscrow), ...tok }, { tokenMint, slot, rentBps, feeLamports, termSecs });
}
/** `accept_lease`: the token side pays the fee and the lease starts. */
export function marketAcceptLease(payer: PublicKey, item: PublicKey, lessor: PublicKey): TransactionInstruction {
  return idlIx('market', 'accept_lease', { payer, lease: leaseAddress(item), lessor });
}
/** `withdraw_offer` (an offer not yet accepted) or `end_lease` (after its term). */
export function marketCloseLease(name: 'withdraw_offer' | 'end_lease', caller: PublicKey, item: PublicKey, itemMint: PublicKey, lessor: PublicKey): TransactionInstruction {
  const leaseEscrow = leaseEscrowAddress(itemMint);
  return idlIx('market', name, { caller, lease: leaseAddress(item), lessor, itemMint, leaseEscrow, leaseEscrowHolding: holdingAddr(itemMint, leaseEscrow), lessorHolding: holdingAddr(itemMint, lessor), ...tok });
}
/** `open_commission(nonce, slot, brief_uri, bounty_lamports, window_secs)`. */
export function marketOpenCommission(creator: PublicKey, tokenMint: PublicKey, nonce: bigint, slot: number, briefUri: string, bountyLamports: bigint, windowSecs: number): TransactionInstruction {
  return idlIx('market', 'open_commission', { creator, tokenMint }, { nonce, slot, briefUri, bountyLamports, windowSecs });
}
/** `submit`: offers an item the submitter holds for a commission. */
export function marketSubmit(submitter: PublicKey, commission: PublicKey, tokenMint: PublicKey, item: PublicKey, itemMint: PublicKey): TransactionInstruction {
  return idlIx('market', 'submit', { submitter, commission, tokenMint, item, itemMint, submitterHolding: holdingAddr(itemMint, submitter), submission: submissionAddress(commission, item) });
}
/** `pay_commission`: anyone pays the winner once its item holds the slot. */
export function marketPayCommission(commission: PublicKey, tokenMint: PublicKey, submissionItem: PublicKey, submitter: PublicKey): TransactionInstruction {
  return idlIx('market', 'pay_commission', { commission, submission: submissionAddress(commission, submissionItem), tokenMint, submitter, vault: commissionVaultAddress(commission) });
}
/** `refund_commission`: the bounty returns to the creator after the window with no winner. */
export function marketRefundCommission(commission: PublicKey, creator: PublicKey): TransactionInstruction {
  return idlIx('market', 'refund_commission', { commission, creator, vault: commissionVaultAddress(commission) });
}
export { commissionAddress };

// ---------------------------------------------------------------- social (10 sections 9, 10) --

/** `claim_badge(badge_id, recipient_key)`: anyone may claim for a recipient who meets the criterion. */
export function socialClaimBadge(claimant: PublicKey, badgeId: number, recipient: PublicKey): TransactionInstruction {
  const badgeMint = badgeMintAddress(badgeId);
  return idlIx('social', 'claim_badge', { claimant, recipient, badgeMint, recipientHolding: holdingAddr(badgeMint, recipient), minter: badgeMinterAddress(), ...tok }, { badgeId, recipientKey: recipient });
}
/** `create_guild(name)`; `nextId` is `SocialConfig.guilds`. */
export function socialCreateGuild(founder: PublicKey, nextId: number, name: string): TransactionInstruction {
  return idlIx('social', 'create_guild', { founder, guild: guildAddress(nextId) }, { name });
}
/** `deposit_sol(lamports)` into a guild's treasury. */
export function socialDeposit(from: PublicKey, guildId: number, lamports: bigint): TransactionInstruction {
  return idlIx('social', 'deposit_sol', { from, guild: guildAddress(guildId), treasury: guildTreasuryAddress(guildId) }, { lamports });
}
/** `propose_action(kind)`; `nonce` is `Guild.actions`. */
export function socialProposeAction(officer: PublicKey, guildId: number, nonce: bigint, kind: Record<string, unknown>): TransactionInstruction {
  return idlIx('social', 'propose_action', { officer, guild: guildAddress(guildId), action: guildActionAddress(guildId, nonce) }, { kind });
}
export function socialApproveAction(officer: PublicKey, guildId: number, nonce: bigint): TransactionInstruction {
  return idlIx('social', 'approve_action', { officer, guild: guildAddress(guildId), action: guildActionAddress(guildId, nonce) });
}
/** `execute_action`: after the guild timelock; `to` is the action's recipient. */
export function socialExecuteAction(guildId: number, nonce: bigint, to: PublicKey): TransactionInstruction {
  return idlIx('social', 'execute_action', { guild: guildAddress(guildId), action: guildActionAddress(guildId, nonce), treasury: guildTreasuryAddress(guildId), to, ...tok });
}

// ---------------------------------------------------------------- agents (09) --

export interface PassportArgsInput { name: string; avatarUri: string; bioUri: string; hireUri: string; kinds: number; creditAgentId: Buffer | null }
/** `register_passport(index, args)`: operator and agent key both sign (the operator's co-sign). */
export function agentsRegisterPassport(operator: PublicKey, agentKey: PublicKey, payer: PublicKey, feeCollector: PublicKey, index: number, args: PassportArgsInput): TransactionInstruction {
  const passport = passportAddress(operator, index);
  return idlIx('agents', 'register_passport', { operator, agentKey, payer, feeCollector, passport, badgeMint: agentBadgeMintAddress(passport, 0), token: TOKEN_ID }, { index, args });
}
export function agentsUpdateProfile(operator: PublicKey, passport: PublicKey, badgeMint: PublicKey, args: PassportArgsInput): TransactionInstruction {
  return idlIx('agents', 'update_profile', { operator, passport, badgeMint, signer: agentsSignerAddress(), token: TOKEN_ID }, { args });
}
export interface PolicyLimitsInput { perActionLamports: bigint; perDayLamports: bigint; tracked: { mint: PublicKey; perAction: bigint; perDay: bigint }[]; targets: PublicKey[] }
/** The tracked list as the program's `Vec<(Pubkey, u64, u64)>` (same bytes as the IDL's struct). */
const limitsArg = (l: PolicyLimitsInput) => ({ perActionLamports: l.perActionLamports, perDayLamports: l.perDayLamports, tracked: l.tracked, targets: l.targets });
export function agentsInitPolicy(operator: PublicKey, payer: PublicKey, passport: PublicKey, limits: PolicyLimitsInput): TransactionInstruction {
  return idlIx('agents', 'init_policy', { operator, payer, passport }, { limits: limitsArg(limits) });
}
export function agentsSetLimits(operator: PublicKey, passport: PublicKey, limits: PolicyLimitsInput): TransactionInstruction {
  return idlIx('agents', 'set_limits', { operator, passport }, { limits: limitsArg(limits) });
}
export function agentsFreezePolicy(operator: PublicKey, passport: PublicKey, frozen: boolean): TransactionInstruction {
  return idlIx('agents', 'freeze_policy', { operator, passport }, { frozen });
}
/** `withdraw(amount, mint)`: the operator takes SOL or a tracked token out of the agent vault. */
export function agentsWithdraw(operator: PublicKey, passport: PublicKey, amount: bigint, mint: PublicKey | null): TransactionInstruction {
  return idlIx('agents', 'withdraw', { operator, passport, vault: agentVaultAddress(passport), token: TOKEN_ID }, { amount, mint });
}
/** The badge's `["armory-caller", badge_mint]` under the agents program (09 section 21 note 1). */
export const agentsArmoryCaller = (badgeMint: PublicKey): PublicKey => PublicKey.findProgramAddressSync([Buffer.from('armory-caller'), badgeMint.toBuffer()], AGENTS_ID)[0];
/** `equip_badge`: forwards the armory's `equip_launch` of the shared Soulbound item (template 42)
 * into the badge's slot 0, signed by CPI as the agents caller. The armory's accounts follow as
 * remaining accounts, the caller not a transaction signer. */
export function agentsEquipBadge(payer: PublicKey, passport: PublicKey, badgeMint: PublicKey, soulboundItem: PublicKey): TransactionInstruction {
  const caller = agentsArmoryCaller(badgeMint);
  const inner = equipLaunch(payer, badgeMint, QUOTE, { slot: 0, item: soulboundItem, config: { targets: [], role: 0 }, noticeSecs: 0, rule: null },
    { item: soulboundItem, templateId: 42, tokenCuts: false, poolCuts: false, composite: false });
  const launchCaller = armoryCallerAddress(badgeMint);
  const keys = inner.keys.map((k) => (k.pubkey.equals(launchCaller) ? { pubkey: caller, isSigner: false, isWritable: false } : k));
  return idlIx('agents', 'equip_badge', { passport, badgeMint, caller }, {}, keys);
}
/** `issue_badge`: mints the one badge to the agent key's holding once slot 0 holds the Soulbound item. */
export function agentsIssueBadge(payer: PublicKey, passport: PublicKey, agentKey: PublicKey, badgeMint: PublicKey): TransactionInstruction {
  return idlIx('agents', 'issue_badge', { payer, passport, agentKey, badgeMint, badgeHolding: holdingAddr(badgeMint, agentKey), signer: agentsSignerAddress() });
}
/** `post_bond`: the agent key bonds two treaty proposals (one on each side). */
export function agentsPostBond(agentKey: PublicKey, passport: PublicKey, proposalA: PublicKey, proposalB: PublicKey, treatyItem: PublicKey): TransactionInstruction {
  return idlIx('agents', 'post_bond', { agentKey, passport, proposalA, proposalB, treatyItem });
}
/** `resolve_bond`: anyone, once both proposals are decided. */
export function agentsResolveBond(bond: PublicKey, passport: PublicKey, agentKey: PublicKey, proposalA: PublicKey, proposalB: PublicKey, mintA: PublicKey, mintB: PublicKey): TransactionInstruction {
  return idlIx('agents', 'resolve_bond', {
    config: agentsConfigAddress(), bond, passport, proposalA, proposalB, mintA, mintB, armoryConfig: armoryConfigAddress(), agentKey,
    inboxA: treatyInboxAddress(mintA), inboxB: treatyInboxAddress(mintB),
  });
}

// ---------------------------------------------------------------- items payouts (08 waves D and E) --

/** `set_referrer(referrer)`: the buyer names who referred them, once per mint. */
export function itemsSetReferrer(buyer: PublicKey, mint: PublicKey, referrer: PublicKey): TransactionInstruction {
  return idlIx('items', 'set_referrer', { buyer, mint, referred: referredAddress(mint, buyer) }, { referrer });
}
/** `settle_referral(slot)`: pays what a buyer's referrer is owed from the Referral vault. */
export function itemsSettleReferral(cranker: PublicKey, mint: PublicKey, buyer: PublicKey, slot: number, item: PublicKey, referrer: PublicKey): TransactionInstruction {
  const owner = referralVaultOwner(mint);
  return idlIx('items', 'settle_referral', {
    cranker, mint, referred: referredAddress(mint, buyer), equipState: equipStateAddress(mint, slot), item, armoryConfig: armoryConfigAddress(), referralOwner: owner,
    vault: holdingAddr(QUOTE, owner), referrerQuote: holdingAddr(QUOTE, referrer), crankerQuote: holdingAddr(QUOTE, cranker), quoteMint: QUOTE, tokenEventAuthority: TOKEN_EVENT_AUTHORITY,
  }, { slot });
}
export function itemsInitFirstBlood(payer: PublicKey, mint: PublicKey): TransactionInstruction {
  return idlIx('items', 'init_first_blood', { payer, mint, firstBlood: firstBloodAddress(mint) });
}
export function itemsInitLoyalty(payer: PublicKey, mint: PublicKey, slot: number): TransactionInstruction {
  return idlIx('items', 'init_loyalty', { payer, mint, pot: loyaltyPotAddress(mint) }, { slot });
}
/** `claim_loyalty`: a holder who held the whole last epoch claims a share of the pot. */
export function itemsClaimLoyalty(holder: PublicKey, mint: PublicKey, slot: number, item: PublicKey, composite: boolean, pool: PublicKey): TransactionInstruction {
  const pot = loyaltyPotAddress(mint);
  return idlIx('items', 'claim_loyalty', {
    holder, mint, pot, potVault: holdingAddr(QUOTE, pot), equipState: equipStateAddress(mint, slot), item, composite: composite ? compositeAddress(item) : ITEMS_ID,
    holderToken: holdingAddr(mint, holder), holderQuote: holdingAddr(QUOTE, holder), receipt: loyaltyClaimAddress(mint, holder), launch: launchAddr(mint),
    poolToken: holdingAddr(mint, pool), launchToken: holdingAddr(mint, launchAddr(mint)), quoteMint: QUOTE, tokenEventAuthority: TOKEN_EVENT_AUTHORITY,
  });
}

