// Changed by Hookwars: new file (app pass v3). The hook economy's client side (docs/spec/11, 12
// section 7, 13 section 6): the optional account suffixes the armory, items and market take, the
// new armory, items, market and social instructions, the agents directive, commit and post, and
// the craft and book programs. Everything is built from the generated IDLs (`idlIx`); the suffix
// shapes are `crates/hookwars-common` `agents_record`, `market`, `eco_cpi` (`init_wear_metas`,
// `social_metas`) and the items program's `settle.rs`.
import { createHash } from 'node:crypto';
import { accessPolicyAddress, adminActionHash, gated, queueAdmin } from './access.ts';
import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SystemProgram, type AccountMeta, type TransactionInstruction } from '@solana/web3.js';
import { FIXED_ADDRESSES } from '@hookwars/shared';
import { coderOf, idlIx } from './from-idl.ts';
import {
  AGENTS_ID, ARMORY_ID, BOOK_ID, CRAFT_ID, ITEMS_ID, MARKET_ID, SOCIAL_ID, TOKEN_ID,
  agentsCallerAddress, agentsConfigAddress, authorCounterAddress, bookConfigAddress, bookEscrowAddress, bookMarketAddress,
  claimCounterAddress, classBidAddress, commitmentAddress, compositeAddress, craftCallerAddress, craftConfigAddress, craftMinterAddress,
  directiveAddress, dropRuleAddress, eventAuthorityOf, holdingAddr, itemAddress, itemMintAddress, leaseAddress, leaseEscrowAddress,
  marketCallerAddress, materialAddress, materialMintAddress, memoConfigAddress, policyAddress, presetAddress, profileAddress,
  recipeAddress, skillsAddress, slotStateAddress, socialCallerAddress, templateAddress, wearAddress,
  equipStateAddress, loyaltyPotAddress,
} from './addresses.ts';
import type { EquipChange } from './instructions.ts';
import * as up from '../addresses.ts';

const TOKEN_EVENT_AUTHORITY = new PublicKey(FIXED_ADDRESSES.tokenEventAuthority);
const QUOTE = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
const tok = { tokenProgram: TOKEN_ID, tokenEventAuthority: TOKEN_EVENT_AUTHORITY };
const ro = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: false });
const rw = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: true });

// ---------------------------------------------------------------- suffixes --

/** `agents_record::suffix_metas`: `[AGENTS_ID, ["agents-caller"] under the caller, agents event
 * authority, passport (mut), actor]` (12 section 2). Attributes the action to an agent passport. */
export function recordSuffix(callerProgram: PublicKey, passport: PublicKey, actor: PublicKey): AccountMeta[] {
  return [ro(AGENTS_ID), ro(agentsCallerAddress(callerProgram)), ro(eventAuthorityOf(AGENTS_ID)), rw(passport), ro(actor)];
}

/** `eco_cpi::social_metas`: `[SOCIAL_ID, skills, profile (mut), social event authority, the caller's
 * ["social-caller"]]` (13 E-6). A wallet without a profile is a no-op on chain. */
export function socialSuffix(callerProgram: PublicKey, wallet: PublicKey): AccountMeta[] {
  return [ro(SOCIAL_ID), ro(skillsAddress()), rw(profileAddress(wallet)), ro(eventAuthorityOf(SOCIAL_ID)), ro(socialCallerAddress(callerProgram))];
}

/** `eco_cpi::init_wear_metas`: the craft head, the item's `Wear` (mut) and the system program (13 E-3);
 * required on item creation when the template has `charges_on_create` above 0. */
export function initWearSuffix(callerProgram: PublicKey, item: PublicKey): AccountMeta[] {
  return [...craftHead(callerProgram), rw(wearAddress(item)), ro(SystemProgram.programId)];
}

function craftHead(callerProgram: PublicKey): AccountMeta[] {
  return [ro(CRAFT_ID), ro(craftConfigAddress()), ro(eventAuthorityOf(CRAFT_ID)), ro(craftCallerAddress(callerProgram))];
}

/** The lease suffix of `settle_equip`: `[MARKET_ID, ["lease", item], lessor's token holding,
 * lessor's quote holding]` (12 section 3 I-7). */
export function rentSuffix(item: PublicKey, lessor: PublicKey, tokenMint: PublicKey, quoteMint: PublicKey = QUOTE): AccountMeta[] {
  return [ro(MARKET_ID), ro(leaseAddress(item)), rw(holdingAddr(tokenMint, lessor)), rw(holdingAddr(quoteMint, lessor))];
}

/** The fee suffix of `settle_equip`: `[Template, the armory admin's token holding, the template
 * registrant's token holding, its quote holding]` (13 E-2, R34); required once `item_protocol_bps` is set. */
export function feeSuffix(templateId: number, admin: PublicKey, registrant: PublicKey, tokenMint: PublicKey, quoteMint: PublicKey = QUOTE): AccountMeta[] {
  return [ro(templateAddress(templateId)), rw(holdingAddr(tokenMint, admin)), rw(holdingAddr(tokenMint, registrant)), rw(holdingAddr(quoteMint, registrant))];
}

/** Craft's drop sources (`eco_cpi::drop_source`). */
export const DROP_SOURCE = { settleCrank: 0, raidReveal: 1, seasonFinish: 2, questClaim: 3 } as const;

/** The craft suffix of `settle_equip` (12 accounts, 13 E-3 and E-4): the craft head under items,
 * the item's `Wear`, the settle drop rule, its material and material mint, craft's minter, the
 * recipient (the item's holder), its material holding and its holding of the item mint. */
export function settleCraftSuffix(o: { item: PublicKey; itemMint: PublicKey; recipient: PublicKey; materialId: number }): AccountMeta[] {
  const materialMint = materialMintAddress(o.materialId);
  return [
    ...craftHead(ITEMS_ID), rw(wearAddress(o.item)), rw(dropRuleAddress(DROP_SOURCE.settleCrank)), rw(materialAddress(o.materialId)), rw(materialMint),
    ro(craftMinterAddress()), ro(o.recipient), rw(holdingAddr(materialMint, o.recipient)), ro(holdingAddr(o.itemMint, o.recipient)),
  ];
}

/** The economy suffixes in the order every program splits them from the end (13 section 2):
 * `[..., rent, craft, fee, social, agents]`. Leave out the ones that do not apply. */
export interface Suffixes { rent?: AccountMeta[]; craft?: AccountMeta[]; fee?: AccountMeta[]; social?: AccountMeta[]; agents?: AccountMeta[] }
export function suffixes(s: Suffixes): AccountMeta[] {
  return [...(s.rent ?? []), ...(s.craft ?? []), ...(s.fee ?? []), ...(s.social ?? []), ...(s.agents ?? [])];
}

// ---------------------------------------------------------------- armory (12, 13) --

/** What item creation may append: the author's `AuthorCounter` (only when it exists), the
 * init-wear suffix (when the template wears), the social and agents suffixes. */
export interface CreateExtras { counter?: boolean; wear?: boolean; social?: boolean; record?: { passport: PublicKey; actor: PublicKey } }

function createTail(author: PublicKey, item: PublicKey, x: CreateExtras): AccountMeta[] {
  return [
    ...(x.counter ? [rw(authorCounterAddress(author))] : []),
    ...(x.wear ? initWearSuffix(ARMORY_ID, item) : []),
    ...suffixes({ social: x.social ? socialSuffix(ARMORY_ID, author) : undefined, agents: x.record ? recordSuffix(ARMORY_ID, x.record.passport, x.record.actor) : undefined }),
  ];
}

/** `create_item(template_id, params, royalty_bps)` with the economy tail: counter, init-wear,
 * social, agents (12 I-5, 13 E-3 and E-6). `itemsMinted` is `ArmoryConfig.items_minted` now. */
export function createItemEco(author: PublicKey, templateId: number, params: number[], royaltyBps: number, itemsMinted: bigint, extras: CreateExtras = {}, recipient: PublicKey = author): TransactionInstruction {
  const itemMint = itemMintAddress(itemsMinted);
  const item = itemAddress(itemMint);
  return idlIx('armory', 'create_item', {
    author, template: templateAddress(templateId), itemMint, item, recipientHolding: holdingAddr(itemMint, recipient),
  }, { templateId, params, royaltyBps }, createTail(author, item, extras));
}

/** The composite template id (`hookwars_common::template_id::COMPOSITE`). */
export const COMPOSITE_TEMPLATE_ID = 41;

export interface ModuleInput { templateId: number; params: number[]; targetStart: number; targetCount: number; dataBytes: number; readsModule: number }

function compositeAccounts(author: PublicKey, itemsMinted: bigint) {
  const itemMint = itemMintAddress(itemsMinted);
  const item = itemAddress(itemMint);
  return { item, accounts: { author, template: templateAddress(COMPOSITE_TEMPLATE_ID), itemMint, item, composite: compositeAddress(item), recipientHolding: holdingAddr(itemMint, author) } };
}

/** `create_composite(modules, royalty_bps)`: remaining = each module's template, then the tail. */
export function createComposite(author: PublicKey, modules: ModuleInput[], royaltyBps: number, itemsMinted: bigint, extras: CreateExtras = {}): TransactionInstruction {
  const { item, accounts } = compositeAccounts(author, itemsMinted);
  return idlIx('armory', 'create_composite', accounts, { modules, royaltyBps }, [...modules.map((m) => ro(templateAddress(m.templateId))), ...createTail(author, item, extras)]);
}

/** One component of a `fuse`: an item the author holds, its template and its target range. */
export interface FuseComponent { item: PublicKey; itemMint: PublicKey; templateId: number; start: number; count: number }

/** `fuse(targets, royalty_bps)` (08 wave F, 13 section 4): burns 2 or more held items into one
 * composite. Remaining: the components' templates, then `(item, item mint (mut), holder's holding
 * (mut))` each, then the tail. One-way. */
export function fuse(author: PublicKey, components: FuseComponent[], royaltyBps: number, itemsMinted: bigint, extras: CreateExtras = {}): TransactionInstruction {
  const { item, accounts } = compositeAccounts(author, itemsMinted);
  return idlIx('armory', 'fuse', accounts, { targets: components.map((c) => ({ start: c.start, count: c.count })), royaltyBps }, [
    ...components.map((c) => ro(templateAddress(c.templateId))),
    ...components.flatMap((c) => [ro(c.item), rw(c.itemMint), rw(holdingAddr(c.itemMint, author))]),
    ...createTail(author, item, extras),
  ]);
}

/** `register_preset(id, template_ids, name)` (admin; gated by the admin queue since pass 4a: send `queueFor` of it first). */
export function registerPreset(admin: PublicKey, id: number, templateIds: number[], name: string): TransactionInstruction {
  return gated(admin, 'register_preset', {}, { id, templateIds, name }).apply;
}

/** `mint_composite(preset_id, modules, royalty_bps)`: the preset first in the remaining accounts,
 * then what `create_composite` takes. */
export function mintComposite(author: PublicKey, presetId: number, modules: ModuleInput[], royaltyBps: number, itemsMinted: bigint, extras: CreateExtras = {}): TransactionInstruction {
  const { item, accounts } = compositeAccounts(author, itemsMinted);
  return idlIx('armory', 'mint_composite', accounts, { presetId, modules, royaltyBps }, [
    ro(presetAddress(presetId)), ...modules.map((m) => ro(templateAddress(m.templateId))), ...createTail(author, item, extras),
  ]);
}

/** `set_template_economy(template_id, author_bps, default_access, allowed_access, charges_on_create)` (admin, 13 E-7). */
export function setTemplateEconomy(admin: PublicKey, templateId: number, authorBps: number, defaultAccess: number, allowedAccess: number, chargesOnCreate: number): TransactionInstruction {
  return gated(admin, 'set_template_economy', { template: templateAddress(templateId) }, { templateId, authorBps, defaultAccess, allowedAccess, chargesOnCreate }).apply;
}

/** `set_item_protocol_bps(item_protocol_bps)` (admin, 13 E-2; gated, `AdminQueued` accounts). */
export function setItemProtocolBps(admin: PublicKey, itemProtocolBps: number): TransactionInstruction {
  return gated(admin, 'set_item_protocol_bps', {}, { itemProtocolBps }).apply;
}

/** The `queue_admin` instruction for a gated armory instruction built above (no bound keys). */
export function queueFor(apply: TransactionInstruction): TransactionInstruction {
  return queueAdmin(apply.keys[0]!.pubkey, adminActionHash(apply.data));
}

/** `init_counters(wallet)`: permissionless and idempotent (12 I-5). */
export function initCounters(payer: PublicKey, wallet: PublicKey): TransactionInstruction {
  return idlIx('armory', 'init_counters', { payer }, { wallet });
}

/** `claim_royalty(amount)` with the claimant's `ClaimCounter` (only when it exists) and the agents suffix. */
export function claimRoyaltyEco(claimant: PublicKey, item: PublicKey, itemMint: PublicKey, cutMint: PublicKey, amount: bigint, extras: { counter?: boolean; record?: { passport: PublicKey; actor: PublicKey } } = {}): TransactionInstruction {
  const owner = PublicKey.findProgramAddressSync([Buffer.from('royalty'), item.toBuffer()], ARMORY_ID)[0];
  return idlIx('armory', 'claim_royalty', {
    claimant, item, itemHolding: holdingAddr(itemMint, claimant), cutMint, royaltyHolding: holdingAddr(cutMint, owner), destination: holdingAddr(cutMint, claimant),
  }, { amount }, [...(extras.counter ? [rw(claimCounterAddress(claimant))] : []), ...(extras.record ? recordSuffix(ARMORY_ID, extras.record.passport, extras.record.actor) : [])]);
}

/**
 * The remaining accounts of market `end_lease` that revert the slot (12 section 3): `["market-caller"]`
 * first (the market signs it), then the armory's `revert_for_lease_end` accounts in order:
 * `config`, `slot_state`, the `EquipCtx` (old item the leased one, new item the slot's launch item)
 * and the armory's event authority and program. `payer` signs the transaction.
 */
export function revertForLeaseEndMetas(payer: PublicKey, mint: PublicKey, slot: number, leasedItem: PublicKey, change: EquipChange): AccountMeta[] {
  const keys = coderOf('armory').metas('revert_for_lease_end', {
    marketCaller: marketCallerAddress(), slotState: slotStateAddress(mint, slot), payer, tokenMint: mint,
    slotAuthority: PublicKey.findProgramAddressSync([Buffer.from('slots'), mint.toBuffer()], ARMORY_ID)[0],
    equipState: equipStateAddress(mint, slot), ...change,
  }, { slot, item: leasedItem });
  return keys.map((k, i) => ({ pubkey: k.pubkey, isSigner: i !== 0 && k.pubkey.equals(payer), isWritable: k.isWritable }));
}

/**
 * Market `end_lease` with the slot revert (12 section 3). `revert` is required since review 3 M-5:
 * the revert accounts, or the lease's token mint alone when the slot no longer holds the item.
 */
export function marketEndLease(caller: PublicKey, item: PublicKey, itemMint: PublicKey, lessor: PublicKey, revert: AccountMeta[] = []): TransactionInstruction {
  const leaseEscrow = leaseEscrowAddress(itemMint);
  return idlIx('market', 'end_lease', { caller, lease: leaseAddress(item), lessor, itemMint, leaseEscrow, leaseEscrowHolding: holdingAddr(itemMint, leaseEscrow), lessorHolding: holdingAddr(itemMint, lessor), ...tok }, {}, revert);
}

/** Market `buy(max_price)` with the social suffix (the seller's `ITEMS_SOLD`, 13 E-6). */
export function marketBuyEco(buyer: PublicKey, seller: PublicKey, author: PublicKey, treasury: PublicKey, item: PublicKey, itemMint: PublicKey, maxPrice: bigint, social = true): TransactionInstruction {
  const escrow = PublicKey.findProgramAddressSync([Buffer.from('escrow'), itemMint.toBuffer()], MARKET_ID)[0];
  return idlIx('market', 'buy', { buyer, seller, author, treasury, item, itemMint, escrow, escrowHolding: holdingAddr(itemMint, escrow), buyerHolding: holdingAddr(itemMint, buyer), ...tok }, { maxPrice }, social ? socialSuffix(MARKET_ID, seller) : []);
}

/** Items `reslot_loyalty(slot)` (08 arsenal 2 request 7): moves the Loyalty Pot to `slot` when its
 * old slot no longer holds a Loyalty Pot. Permissionless. */
export function reslotLoyalty(mint: PublicKey, oldSlot: number, oldItem: PublicKey, newSlot: number, newItem: PublicKey): TransactionInstruction {
  return idlIx('items', 'reslot_loyalty', {
    mint, pot: loyaltyPotAddress(mint), oldEquipState: equipStateAddress(mint, oldSlot), oldItem, oldComposite: compositeAddress(oldItem),
    newEquipState: equipStateAddress(mint, newSlot), newItem, newComposite: compositeAddress(newItem),
  }, { slot: newSlot });
}

// ---------------------------------------------------------------- agents (11 section 4) --

export interface DirectiveConstraintsInput { maxSpendPerAction: bigint; maxSpendPerDay: bigint; allowedTargets: PublicKey[]; allowedAccessModes: number; maxLicencePrice: bigint; frozen: boolean }

/** Pass 5 (review 3 L-6): hex `sha256(borsh(constraints))`, the directive memo's `c`, which
 * `set_directive` checks against the constraints it writes. */
export function directiveConstraintsHashHex(c: DirectiveConstraintsInput): string {
  const u64 = (x: bigint) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(x); return b; };
  const n = Buffer.alloc(4); n.writeUInt32LE(c.allowedTargets.length);
  const bytes = Buffer.concat([u64(c.maxSpendPerAction), u64(c.maxSpendPerDay), n, ...c.allowedTargets.map((t) => t.toBuffer()), Buffer.from([c.allowedAccessModes]), u64(c.maxLicencePrice), Buffer.from([c.frozen ? 1 : 0])]);
  return createHash('sha256').update(bytes).digest('hex');
}

/** `set_directive(seq, constraints)`; the directive memo signed by the operator goes in the same transaction. */
export function agentsSetDirective(operator: PublicKey, passport: PublicKey, seq: number, constraints: DirectiveConstraintsInput): TransactionInstruction {
  return idlIx('agents', 'set_directive', {
    operator, config: agentsConfigAddress(), memoConfig: memoConfigAddress(), passport, policy: policyAddress(passport),
    directive: directiveAddress(passport, seq), previous: seq === 0 ? null : directiveAddress(passport, seq - 1), instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
  }, { seq, constraints });
}

/** `commit(reference, hash)`: a binding record of an accepted offer (moves no money). */
export function agentsCommit(agent: PublicKey, passport: PublicKey, reference: Uint8Array, hash: Uint8Array): TransactionInstruction {
  return idlIx('agents', 'commit', { agent, passport, commitment: commitmentAddress(passport, reference) }, { reference: Buffer.from(reference), hash: Buffer.from(hash) });
}

/** `post(reference)`: pays the postage for a message. */
export function agentsPost(agent: PublicKey, passport: PublicKey, feeCollector: PublicKey, reference: Uint8Array): TransactionInstruction {
  return idlIx('agents', 'post', { agent, passport, config: agentsConfigAddress(), memoConfig: memoConfigAddress(), feeCollector }, { reference: Buffer.from(reference) });
}

// ---------------------------------------------------------------- social (11 section 3.3) --

/** `open_profile(wallet)`: anyone pays; the profile collects the wallet's counters. */
export function socialOpenProfile(payer: PublicKey, wallet: PublicKey): TransactionInstruction {
  return idlIx('social', 'open_profile', { payer, profile: profileAddress(wallet) }, { wallet });
}

// ---------------------------------------------------------------- craft (11 section 5) --

/** Craft's view of the social accounts a craft, repair or book order counts into. */
function socialAccounts(program: PublicKey, wallet: PublicKey) {
  return { skills: skillsAddress(), profile: profileAddress(wallet), socialCaller: socialCallerAddress(program), socialEventAuthority: eventAuthorityOf(SOCIAL_ID), socialProgram: SOCIAL_ID };
}

/** `(material, material mint, holding)` per recipe input, the first block of craft's remaining accounts. */
export function recipeInputMetas(owner: PublicKey, materialIds: number[]): AccountMeta[] {
  return materialIds.flatMap((id) => [rw(materialAddress(id)), rw(materialMintAddress(id)), rw(holdingAddr(materialMintAddress(id), owner))]);
}

/**
 * `craft(recipe_id, reference)`: burns the inputs, pays the fee and has the armory's `mint_crafted`
 * make the item. Remaining: the inputs, then `mint_crafted`'s accounts after the craft signer (and
 * the init-wear suffix when the template wears). `itemsMinted` is `ArmoryConfig.items_minted` now.
 */
export function craftItem(crafter: PublicKey, o: { recipeId: number; templateId: number; materialIds: number[]; treasury: PublicKey; seasonPool: PublicKey; itemsMinted: bigint; wear: boolean; reference?: Uint8Array }): TransactionInstruction {
  const itemMint = itemMintAddress(o.itemsMinted);
  const item = itemAddress(itemMint);
  const mintCrafted = coderOf('armory').metas('mint_crafted', {
    craftSigner: PublicKey.findProgramAddressSync([Buffer.from('craft-signer')], CRAFT_ID)[0], payer: crafter, owner: crafter,
    template: templateAddress(o.templateId), itemMint, item, recipientHolding: holdingAddr(itemMint, crafter),
  }, { templateId: o.templateId });
  const output = [...mintCrafted.slice(1).map((k) => ({ ...k, isSigner: k.pubkey.equals(crafter) })), ...(o.wear ? initWearSuffix(ARMORY_ID, item) : [])];
  return idlIx('craft', 'craft', {
    crafter, recipe: recipeAddress(o.recipeId), treasury: o.treasury, seasonPool: o.seasonPool, outputProgram: ARMORY_ID, ...socialAccounts(CRAFT_ID, crafter), ...tok,
  }, { recipeId: o.recipeId, reference: Buffer.from(o.reference ?? new Uint8Array(32)) }, [...recipeInputMetas(crafter, o.materialIds), ...output]);
}

/** `repair(recipe_id, reference)`: burns the inputs, pays the fee, restores charges and wakes a dormant item. */
export function repairItem(holder: PublicKey, o: { recipeId: number; item: PublicKey; itemMint: PublicKey; materialIds: number[]; treasury: PublicKey; seasonPool: PublicKey; reference?: Uint8Array }): TransactionInstruction {
  return idlIx('craft', 'repair', {
    holder, recipe: recipeAddress(o.recipeId), item: o.item, itemMint: o.itemMint, holderItemHolding: holdingAddr(o.itemMint, holder), wear: wearAddress(o.item),
    treasury: o.treasury, seasonPool: o.seasonPool, ...socialAccounts(CRAFT_ID, holder), ...tok,
  }, { recipeId: o.recipeId, reference: Buffer.from(o.reference ?? new Uint8Array(32)) }, recipeInputMetas(holder, o.materialIds));
}

// ---------------------------------------------------------------- book (11 section 6) --

/** Book sides (`side_`): 0 bid (buy the material with lamports), 1 ask. */
export const BOOK_SIDE = { bid: 0, ask: 1 } as const;

function bookAccounts(baseMint: PublicKey) {
  const market = bookMarketAddress(baseMint);
  const escrow = bookEscrowAddress(market);
  return { market, escrow, baseMint, escrowHolding: holdingAddr(baseMint, escrow) };
}

/** `place(side, price, size, post_only, expires_at, reference)`; `makers` are the wallets of the
 * orders it will fill, in book order (and the evicted owner when the side is full): each adds
 * `(wallet, base holding)` to the remaining accounts. */
export function bookPlace(owner: PublicKey, o: { baseMint: PublicKey; treasury: PublicKey; side: number; price: bigint; size: bigint; postOnly: boolean; expiresAt: bigint; makers: PublicKey[]; reference?: Uint8Array }): TransactionInstruction {
  return idlIx('book', 'place', {
    owner, ...bookAccounts(o.baseMint), ownerHolding: holdingAddr(o.baseMint, owner), treasury: o.treasury, ...socialAccounts(BOOK_ID, owner), ...tok,
  }, { side: o.side, price: o.price, size: o.size, postOnly: o.postOnly, expiresAt: o.expiresAt, reference: Buffer.from(o.reference ?? new Uint8Array(32)) },
  o.makers.flatMap((w) => [rw(w), rw(holdingAddr(o.baseMint, w))]));
}

/** `cancel(id)`: the owner takes a resting order and its escrow back. */
export function bookCancel(owner: PublicKey, baseMint: PublicKey, id: bigint): TransactionInstruction {
  return idlIx('book', 'cancel', { owner, config: bookConfigAddress(), ...bookAccounts(baseMint), ownerHolding: holdingAddr(baseMint, owner), ...tok }, { id });
}

/** `crank(max)`: removes up to `max` expired orders; `owners` are their wallets in book order. */
export function bookCrank(cranker: PublicKey, baseMint: PublicKey, max: number, owners: PublicKey[]): TransactionInstruction {
  return idlIx('book', 'crank', { cranker, ...bookAccounts(baseMint), ...tok }, { max }, owners.flatMap((w) => [rw(w), rw(holdingAddr(baseMint, w))]));
}

/** `create_market(tick_lamports, min_size)` for a craft material. */
export function bookCreateMarket(creator: PublicKey, materialId: number, tickLamports: bigint, minSize: bigint): TransactionInstruction {
  const baseMint = materialMintAddress(materialId);
  return idlIx('book', 'create_market', { creator, material: materialAddress(materialId), ...bookAccounts(baseMint), skills: skillsAddress(), profile: profileAddress(creator), ...tok }, { tickLamports, minSize });
}

export interface ClassKeyInput { templateId: number; minLevel: number; paramMin: number[]; paramMax: number[] }

/** `place_class_bid(nonce, class, price, expires_at)`: a standing bid for any item of a class. */
export function bookPlaceClassBid(bidder: PublicKey, nonce: bigint, cls: ClassKeyInput, price: bigint, expiresAt: bigint): TransactionInstruction {
  return idlIx('book', 'place_class_bid', { bidder, bid: classBidAddress(bidder, nonce) }, { nonce, class: cls, price, expiresAt });
}

/** `cancel_class_bid`. */
export function bookCancelClassBid(bidder: PublicKey, nonce: bigint): TransactionInstruction {
  return idlIx('book', 'cancel_class_bid', { bidder, bid: classBidAddress(bidder, nonce) });
}

/** `match_class(reference)`: the holder of a fitting item sells it into a class bid. */
export function bookMatchClass(seller: PublicKey, o: { bid: PublicKey; bidder: PublicKey; item: PublicKey; itemMint: PublicKey; treasury: PublicKey; reference?: Uint8Array }): TransactionInstruction {
  return idlIx('book', 'match_class', {
    seller, bid: o.bid, bidder: o.bidder, item: o.item, itemMint: o.itemMint, sellerHolding: holdingAddr(o.itemMint, seller), bidderHolding: holdingAddr(o.itemMint, o.bidder),
    listing: PublicKey.findProgramAddressSync([Buffer.from('listing'), o.itemMint.toBuffer()], MARKET_ID)[0], lease: leaseAddress(o.item), treasury: o.treasury, ...tok,
  }, { reference: Buffer.from(o.reference ?? new Uint8Array(32)) });
}


/** An order as the book account holds it. */
export interface BookOrder { id: bigint; owner: PublicKey; price: bigint; size: bigint; expiresAt: bigint }
const expired = (o: BookOrder, now: bigint): boolean => o.expiresAt !== 0n && now >= o.expiresAt;

/**
 * The wallets whose `(wallet, base holding)` pairs a `place` needs, in the order the program takes
 * them (hookwars_book `place`): each opposite order it fills (best first, expired ones skipped,
 * at most `matchMax`), then the evicted owner when what is left rests on a full side.
 */
export function bookMakers(m: { bids: BookOrder[]; asks: BookOrder[]; minSize: bigint }, p: { slots: number; matchMax: number }, side: number, price: bigint, size: bigint, now: bigint): PublicKey[] {
  const out: PublicKey[] = [];
  const book = side === BOOK_SIDE.bid ? m.asks : m.bids;
  let remaining = size;
  let fills = 0;
  for (const o of book) {
    if (remaining === 0n || fills >= p.matchMax) break;
    if (expired(o, now)) continue;
    if (side === BOOK_SIDE.bid ? o.price > price : o.price < price) break;
    out.push(o.owner);
    remaining -= remaining < o.size ? remaining : o.size;
    fills++;
  }
  const own = side === BOOK_SIDE.bid ? m.bids : m.asks;
  if (remaining >= m.minSize && own.length >= p.slots && own.length > 0) out.push(own[own.length - 1]!.owner);
  return out;
}

/** The owners of the expired orders a `crank(max)` removes: bids first, then asks, in book order. */
export function bookCrankOwners(m: { bids: BookOrder[]; asks: BookOrder[] }, max: number, now: bigint): PublicKey[] {
  return [...m.bids, ...m.asks].filter((o) => expired(o, now)).slice(0, max).map((o) => o.owner);
}

/** Companion `launch_slots(data)` (bordrless_companion): one slot launch step (`prepare_launch`,
 * `equip_prepared`, `create_prepared_launch`) made with the companion's creator address as the
 * launch's creator; the step's accounts follow, only the mint keeping its signature. */
export function companionLaunchSlots(launcher: PublicKey, mint: PublicKey, step: TransactionInstruction): TransactionInstruction {
  const companion = up.companionAddress(mint);
  const creator = up.companionCreatorAddress(mint);
  const keys = step.keys.map((k) => ({ pubkey: k.pubkey, isSigner: k.isSigner && k.pubkey.equals(mint), isWritable: k.isWritable }));
  return idlIx('companion', 'launch_slots', { launcher, companion, creator, launchProgram: step.programId }, { data: step.data }, keys);
}

// ---------------------------------------------------------------- licences (11 section 2) --

/** `set_licence_offer(price, term, per, max_live, exclusive, active)` by the item's holder. */
export function setLicenceOffer(holder: PublicKey, item: PublicKey, itemMint: PublicKey, o: { priceLamports: bigint; termSecs: number; per: number; maxLive: number; exclusive: boolean; active: boolean }): TransactionInstruction {
  return idlIx('market', 'set_licence_offer', { holder, item, itemMint, holderHolding: holdingAddr(itemMint, holder) }, o);
}

/** `buy_license(max_price, reference)` (or `renew_license`): a token's right to equip the item for a term. */
export function buyLicense(payer: PublicKey, o: { item: PublicKey; itemMint: PublicKey; templateId: number; tokenMint: PublicKey; holder: PublicKey; author: PublicKey; treasury: PublicKey; maxPrice: bigint; renew?: boolean; reference?: Uint8Array }): TransactionInstruction {
  return idlIx('market', o.renew ? 'renew_license' : 'buy_license', {
    payer, item: o.item, itemMint: o.itemMint, template: templateAddress(o.templateId), tokenMint: o.tokenMint, holder: o.holder, holderHolding: holdingAddr(o.itemMint, o.holder), accessPolicy: accessPolicyAddress(o.item),
    author: o.author, treasury: o.treasury, skills: skillsAddress(), holderProfile: profileAddress(o.holder), socialCaller: socialCallerAddress(MARKET_ID), socialEventAuthority: eventAuthorityOf(SOCIAL_ID), socialProgram: SOCIAL_ID,
  }, { maxPrice: o.maxPrice, reference: Buffer.from(o.reference ?? new Uint8Array(32)) });
}

/** The licence offer address of an item (`["licence-offer", item]` under the market). */
export const licenceOfferAddress = (item: PublicKey): PublicKey => PublicKey.findProgramAddressSync([Buffer.from('licence-offer'), item.toBuffer()], MARKET_ID)[0];
