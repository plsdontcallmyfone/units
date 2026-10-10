/**
 * Instruction builders for the units programs.
 *
 * - The token program, the armory and the war program are built from their generated IDLs
 *   (`idlIx`): exact discriminators, argument encoding and account order, PDAs derived from the
 *   IDL's seeds. Remaining accounts follow the programs' own Rust clients
 *   (`programs/hookwars_war/src/client.rs`: slices first, then the token program and its event
 *   authority, then the accounts of inner instructions).
 * - Slot launches, `swap_route`, `settle_equip` and `init_raid_ledger` are in `slot-launch.ts`, from
 *   the launchpad's, the DEX's and the items program's IDLs.
 * Changed by Hookwars: siege and counter_strike no longer pass an observations account (the ring is in the pool).
 */
import { PublicKey, TransactionInstruction, type AccountMeta } from '@solana/web3.js';
import { idlIx } from './from-idl.ts';
import {
  ITEMS_EVENT_AUTHORITY, ITEMS_ID, LAUNCH_ID, SWAP_ID, TOKEN_ID, TOKEN_ITEMS_SIGNER, WAR_ID,
  equipStateAddress, forgeCounterAddress, holdingAddr, itemAddress, itemMintAddress, launchAddr, lootTableAddress,
  poolCutsAddress, proposalAddress, questMarkAddress, raidLedgerAddress, rollAddress, royaltyOwner, seasonAddress, slotAuthority, slotStateAddress,
  templateAddress, tokenHookSigner, treatyInboxAddress, voteAddress, warChestAddress, warStateAddress,
} from './addresses.ts';
import { FIXED_ADDRESSES } from '@hookwars/shared';

const TOKEN_EVENT_AUTHORITY = new PublicKey(FIXED_ADDRESSES.tokenEventAuthority);
const BRIDGED_SOL_MINT = new PublicKey(FIXED_ADDRESSES.bridgedSolMint);
const ro = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: false });
/** The token program and its event authority, which war steps that `touch` append after the slice. */
const TOKEN_TAIL: AccountMeta[] = [ro(TOKEN_ID), ro(TOKEN_EVENT_AUTHORITY)];

/** Every account of `ixs` and their programs, none signing (war `client::accounts_of`). */
export function accountsOf(ixs: TransactionInstruction[]): AccountMeta[] {
  const out: AccountMeta[] = [];
  for (const ix of ixs) {
    for (const k of ix.keys) out.push({ pubkey: k.pubkey, isSigner: false, isWritable: k.isWritable });
    out.push(ro(ix.programId));
  }
  return out;
}

// ---------------------------------------------------------------- token (IDL) --

/** `touch(slot, payload)` (01 section 5, R23) of `holding` through the item program of slot `slot`;
 * `extras` are that slot's registry accounts (token `client::touch`). */
export function touch(caller: PublicKey, mint: PublicKey, holding: PublicKey, itemProgram: PublicKey, slot: number, payload: Buffer, extras: AccountMeta[]): TransactionInstruction {
  return idlIx('token', 'touch', { caller, mint, holding, itemProgram, hookSigner: tokenHookSigner(itemProgram) }, { slot, payload }, extras);
}

// ---------------------------------------------------------------- armory (IDL) --

/** `create_item(template_id, params, royalty_bps)`; `itemsMinted` is `ArmoryConfig.items_minted` now. */
export function createItem(author: PublicKey, templateId: number, params: number[], royaltyBps: number, itemsMinted: bigint, recipient: PublicKey = author): TransactionInstruction {
  const itemMint = itemMintAddress(itemsMinted);
  return idlIx('armory', 'create_item', {
    author, template: templateAddress(templateId), itemMint, item: itemAddress(itemMint), recipientHolding: holdingAddr(itemMint, recipient),
  }, { templateId, params, royaltyBps });
}

/** `vote(support, amount)`: tokens lock in place (R11). */
export function vote(voter: PublicKey, mint: PublicKey, slot: number, nonce: bigint, support: boolean, amount: bigint): TransactionInstruction {
  return idlIx('armory', 'vote', {
    voter, proposal: proposalAddress(mint, slot, nonce), holding: holdingAddr(mint, voter), tokenMint: mint,
  }, { support, amount });
}

/** `propose(slot, item, equip_config)`. The template accounts are given when proposing an item. */
export function propose(proposer: PublicKey, mint: PublicKey, slot: number, nextNonce: bigint, item: PublicKey | null, targets: PublicKey[], role: number, template?: { address: PublicKey; program: PublicKey; programData: PublicKey }): TransactionInstruction {
  return idlIx('armory', 'propose', {
    proposer, tokenMint: mint, slotState: slotStateAddress(mint, slot), proposal: proposalAddress(mint, slot, nextNonce),
    item, template: template?.address ?? null, templateProgram: template?.program ?? null, templateProgramdata: template?.programData ?? null,
  }, { slot, item, equipConfig: { targets, role } });
}

/** `finalize`: counts the vote after its period. `launch` accounts are given for a launch-pool token. */
export function finalize(mint: PublicKey, slot: number, nonce: bigint, launch?: { launch: PublicKey; poolBaseVault: PublicKey; launchHolding: PublicKey }): TransactionInstruction {
  return idlIx('armory', 'finalize', {
    proposal: proposalAddress(mint, slot, nonce), slotState: slotStateAddress(mint, slot), tokenMint: mint,
    launch: launch?.launch ?? null, poolBaseVault: launch?.poolBaseVault ?? null, launchHolding: launch?.launchHolding ?? null,
  });
}

/** Accounts of an equip change (`execute`, `check_performance`), named as the IDL names them. */
export interface EquipChange {
  oldItem?: PublicKey | null; oldEquipVault?: PublicKey | null; newItem?: PublicKey | null; newTemplate?: PublicKey | null;
  templateProgram?: PublicKey | null; templateProgramdata?: PublicKey | null; registry?: PublicKey | null; newEquipVault?: PublicKey | null;
  royaltyOwner?: PublicKey | null; royaltyHoldingToken?: PublicKey | null; quoteMint?: PublicKey | null; royaltyHoldingQuote?: PublicKey | null;
}

/** `execute`: applies a passed proposal after its notice, signed by the armory as `SlotAuthority`. */
export function execute(payer: PublicKey, mint: PublicKey, slot: number, nonce: bigint, change: EquipChange, remaining: AccountMeta[] = []): TransactionInstruction {
  return idlIx('armory', 'execute', {
    proposal: proposalAddress(mint, slot, nonce), slotState: slotStateAddress(mint, slot), payer, tokenMint: mint,
    slotAuthority: slotAuthority(mint),
    equipState: equipStateAddress(mint, slot), ...change,
  }, {}, remaining);
}

/** `cancel`: the proposer withdraws an open proposal. */
export function cancelProposal(proposer: PublicKey, mint: PublicKey, slot: number, nonce: bigint): TransactionInstruction {
  return idlIx('armory', 'cancel', { proposer, proposal: proposalAddress(mint, slot, nonce), slotState: slotStateAddress(mint, slot) });
}

/** `close_vote`: returns a resolved vote lock's rent to the voter. */
export function closeVote(voter: PublicKey, mint: PublicKey, slot: number, nonce: bigint): TransactionInstruction {
  const proposal = proposalAddress(mint, slot, nonce);
  return idlIx('armory', 'close_vote', {
    voter, proposal, voteLock: voteAddress(proposal, voter),
  });
}

/** `close_proposal`: returns a resolved proposal's rent to its proposer. */
export function closeProposal(proposer: PublicKey, mint: PublicKey, slot: number, nonce: bigint): TransactionInstruction {
  return idlIx('armory', 'close_proposal', { proposer, proposal: proposalAddress(mint, slot, nonce) });
}

/** `claim_royalty(amount)` by the item's current holder, into the claimant's holding of the cut mint. */
export function claimRoyalty(claimant: PublicKey, item: PublicKey, itemMint: PublicKey, cutMint: PublicKey, amount: bigint): TransactionInstruction {
  const owner = royaltyOwner(item);
  return idlIx('armory', 'claim_royalty', {
    claimant, item, itemHolding: holdingAddr(itemMint, claimant), cutMint,
    royaltyHolding: holdingAddr(cutMint, owner), destination: holdingAddr(cutMint, claimant),
  }, { amount });
}

/** `forge`: burns two unequipped items of one template and mints one (02 9.1). Settle and claim
 * both items' royalties first (02 section 10). `itemsMinted` is `ArmoryConfig.items_minted` now. */
export function forge(forger: PublicKey, a: { item: PublicKey; itemMint: PublicKey }, b: { item: PublicKey; itemMint: PublicKey }, templateId: number, itemsMinted: bigint): TransactionInstruction {
  const itemMint = itemMintAddress(itemsMinted);
  return idlIx('armory', 'forge', {
    forger, forgeCounter: forgeCounterAddress(forger), itemA: a.item, itemB: b.item, mintA: a.itemMint, mintB: b.itemMint,
    holdingA: holdingAddr(a.itemMint, forger), holdingB: holdingAddr(b.itemMint, forger), template: templateAddress(templateId),
    itemMint, item: itemAddress(itemMint), recipientHolding: holdingAddr(itemMint, forger),
  });
}

// ---------------------------------------------------------------- war (IDL) --

/** The War orders a step reads: the item the War slot names and its template. */
export interface Orders { item: PublicKey; template: PublicKey }

/** `init_war`: permissionless, once per launch. */
export function initWar(payer: PublicKey, mint: PublicKey, launch: PublicKey = launchAddr(mint)): TransactionInstruction {
  return idlIx('war', 'init_war', {
    payer, mint, launch, warState: warStateAddress(mint), warChest: warChestAddress(mint), treatyInbox: treatyInboxAddress(mint), chestHolding: holdingAddr(BRIDGED_SOL_MINT, warChestAddress(mint)),
    inboxHolding: holdingAddr(BRIDGED_SOL_MINT, treatyInboxAddress(mint)), tokenEventAuthority: TOKEN_EVENT_AUTHORITY,
  });
}

/** `record_funding`: books what reached the chest since the last look. */
export function recordFunding(mint: PublicKey): TransactionInstruction {
  return idlIx('war', 'record_funding', { warState: warStateAddress(mint), chestHolding: holdingAddr(BRIDGED_SOL_MINT, warChestAddress(mint)) });
}

/** `siege`: `slice` is the rival mint's delivery slice; `inner` the instructions the step invokes. */
export function siege(cranker: PublicKey, mint: PublicKey, orders: Orders, rivalMint: PublicKey, rivalPool: PublicKey, _rivalHasWar: boolean, rivalKitConfig: PublicKey | null, slice: AccountMeta[], inner: TransactionInstruction[]): TransactionInstruction {
  return idlIx('war', 'siege', {
    cranker, mint, warState: warStateAddress(mint), warChest: warChestAddress(mint), ordersItem: orders.item, ordersTemplate: orders.template, raidLedger: raidLedgerAddress(mint), rivalMint,
    rivalLaunch: launchAddr(rivalMint), rivalPool,
    // Changed by Hookwars (explorer v2, IDL regenerated at 7b48360): the rival's war state is required now (client.rs passes it always).
    rivalWarState: warStateAddress(rivalMint), rivalKitConfig,
  }, { args: { first: slice.length, second: 0 } }, [...slice, ...accountsOf(inner)]);
}

/** `counter_strike`: `buySlice` for the delivery, `burnSlice` for the burn. */
export function counterStrike(cranker: PublicKey, mint: PublicKey, orders: Orders, pool: PublicKey, kitConfig: PublicKey | null, buySlice: AccountMeta[], burnSlice: AccountMeta[], inner: TransactionInstruction[]): TransactionInstruction {
  return idlIx('war', 'counter_strike', {
    cranker, mint, warState: warStateAddress(mint), warChest: warChestAddress(mint), ordersItem: orders.item, ordersTemplate: orders.template, launch: launchAddr(mint), pool, kitConfig,
  }, { args: { first: buySlice.length, second: burnSlice.length } }, [...buySlice, ...burnSlice, ...accountsOf(inner)]);
}

/** `raze`: sells captured rival tokens, capped per interval. */
export function raze(cranker: PublicKey, mint: PublicKey, orders: Orders, rivalMint: PublicKey, rivalPool: PublicKey, slice: AccountMeta[], inner: TransactionInstruction[]): TransactionInstruction {
  return idlIx('war', 'raze', {
    cranker, mint, warState: warStateAddress(mint), warChest: warChestAddress(mint), ordersItem: orders.item, ordersTemplate: orders.template, rivalMint, rivalLaunch: launchAddr(rivalMint), rivalPool,
  }, { args: { first: slice.length, second: 0 } }, [...slice, ...accountsOf(inner)]);
}

/** `return_captured`: peace, under a treaty both tokens equip. */
export function returnCaptured(cranker: PublicKey, mint: PublicKey, rivalMint: PublicKey, treaty: { item: PublicKey; template: PublicKey }, slice: AccountMeta[], inner: TransactionInstruction[]): TransactionInstruction {
  return idlIx('war', 'return_captured', {
    cranker, mint, warState: warStateAddress(mint), warChest: warChestAddress(mint), rivalMint, treatyItem: treaty.item, treatyTemplate: treaty.template, rivalWarChest: warChestAddress(rivalMint),
  }, { args: { first: slice.length, second: 0 } }, [...slice, ...accountsOf(inner)]);
}

/** `share_treaty_inflow`: streams treaty payments to holders through the kit. */
export function shareTreatyInflow(cranker: PublicKey, mint: PublicKey, inner: TransactionInstruction[]): TransactionInstruction {
  return idlIx('war', 'share_treaty_inflow', { cranker, warState: warStateAddress(mint), treatyInbox: treatyInboxAddress(mint), inboxHolding: holdingAddr(BRIDGED_SOL_MINT, treatyInboxAddress(mint)) }, {}, accountsOf(inner));
}

/** `accrue_treaty_time`: `pairs` of (treaty item, partner mint). */
export function accrueTreatyTime(mint: PublicKey, season: number, pairs: [PublicKey, PublicKey][]): TransactionInstruction {
  return idlIx('war', 'accrue_treaty_time', { mint, warState: warStateAddress(mint), season: seasonAddress(season) }, {}, pairs.flatMap(([item, partner]) => [ro(item), ro(partner)]));
}

/** `claim_bounty(raid_slot)`: `raidExtras` are the Raid slot's extras; `inner` the bridge unwrap. */
export function claimBounty(owner: PublicKey, mint: PublicKey, orders: Orders, raidSlot: number, raidExtras: AccountMeta[] = [], inner: TransactionInstruction[] = []): TransactionInstruction {
  const chest = warChestAddress(mint);
  return idlIx('war', 'claim_bounty', {
    owner, mint, warState: warStateAddress(mint), warChest: chest, chestHolding: holdingAddr(BRIDGED_SOL_MINT, chest), ordersItem: orders.item, ordersTemplate: orders.template,
    holding: holdingAddr(mint, owner), itemsHookSigner: TOKEN_ITEMS_SIGNER,
  }, { raidSlot }, [...raidExtras, ...TOKEN_TAIL, ...accountsOf(inner)]);
}

/** `roll(nonce, raid_slot)`: spends a loot ticket and requests randomness from the configured oracle. */
export function roll(owner: PublicKey, mint: PublicKey, nonce: bigint, raidSlot: number, oracle: { program: PublicKey; account: PublicKey }, raidExtras: AccountMeta[] = []): TransactionInstruction {
  const holding = holdingAddr(mint, owner);
  return idlIx('war', 'roll', {
    owner, mint, holding, rollRequest: rollAddress(holding, nonce), itemsHookSigner: TOKEN_ITEMS_SIGNER,
    oracleProgram: oracle.program, oracleAccount: oracle.account,
  }, { nonce, raidSlot }, [...raidExtras, ...TOKEN_TAIL]);
}

/** `reveal`: mints the drawn item through the armory; `armoryExtra` are `mint_loot`'s further accounts. */
export function reveal(revealer: PublicKey, owner: PublicKey, mint: PublicKey, nonce: bigint, season: number, oracleAccount: PublicKey, armoryExtra: AccountMeta[]): TransactionInstruction {
  return idlIx('war', 'reveal', {
    revealer, owner, rollRequest: rollAddress(holdingAddr(mint, owner), nonce), lootTable: lootTableAddress(season), oracleAccount,
  }, {}, armoryExtra);
}

/** `cancel_roll`: refunds an expired roll's ticket. */
export function cancelRoll(owner: PublicKey, mint: PublicKey, nonce: bigint): TransactionInstruction {
  return idlIx('war', 'cancel_roll', { owner, rollRequest: rollAddress(holdingAddr(mint, owner), nonce) });
}

/** `claim_quest(quest_id, period, raid_slot)`: 1 Raid, 2 Forge (05 section 9). */
export function claimQuest(owner: PublicKey, mint: PublicKey, season: number, questId: 1 | 2, period: number, raidSlot: number, raidExtras: AccountMeta[] = []): TransactionInstruction {
  const markMint = questId === 2 ? PublicKey.default : mint;
  return idlIx('war', 'claim_quest', {
    owner, season: seasonAddress(season), mint, holding: holdingAddr(mint, owner), questMark: questMarkAddress(season, markMint, owner),
    forgeCounter: questId === 2 ? forgeCounterAddress(owner) : null, itemsHookSigner: TOKEN_ITEMS_SIGNER,
  }, { questId, period, raidSlot }, [...raidExtras, ...TOKEN_TAIL]);
}

/** `close_quest_mark`: returns a past season's quest mark rent. */
export function closeQuestMark(owner: PublicKey, season: number, mark: PublicKey): TransactionInstruction {
  return idlIx('war', 'close_quest_mark', { owner, season: seasonAddress(season), questMark: mark });
}

/** `open_season`: opens season `current + 1` once its proposal's eta has passed. */
export function openSeason(current: number): TransactionInstruction {
  return idlIx('war', 'open_season', {
    next: seasonAddress(current + 1), previous: current > 0 ? seasonAddress(current) : null, lootTable: lootTableAddress(current + 1),
  });
}

/** `submit_candidate(number)`: proposes `mint` as the season's leader (O(1) check). */
export function submitCandidate(submitter: PublicKey, number: number, mint: PublicKey, withLedger: boolean): TransactionInstruction {
  return idlIx('war', 'submit_candidate', {
    submitter, season: seasonAddress(number), warState: warStateAddress(mint), raidLedger: withLedger ? raidLedgerAddress(mint) : null,
  }, { number });
}

/** `finalize_season(number)`: after the challenge window. */
export function finalizeSeason(number: number): TransactionInstruction {
  return idlIx('war', 'finalize_season', { season: seasonAddress(number) }, { number });
}

/** `split_protocol_fees`: the prize vault's share to the last winner's chest, the rest to the treasury. */
export function splitProtocolFees(cranker: PublicKey, treasury: PublicKey, winner: { mint: PublicKey; season: number } | null, inner: TransactionInstruction[] = []): TransactionInstruction {
  const vault = new PublicKey(FIXED_ADDRESSES.prizeVault);
  return idlIx('war', 'split_protocol_fees', {
    cranker, prizeHolding: holdingAddr(BRIDGED_SOL_MINT, vault), treasury,
    winnerChest: winner ? warChestAddress(winner.mint) : null, winnerSeason: winner ? seasonAddress(winner.season) : null,
  }, {}, accountsOf(inner));
}
