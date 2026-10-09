/**
 * Instruction builders for the Hookwars programs. An Anchor instruction is
 * `sha256("global:<name>")[..8]` then the Borsh arguments. The argument schemas follow the spec
 * sections named beside each; account lists are passed in the order the spec section gives, and
 * every builder is listed in INTEGRATION.md section 3 to be checked against the generated IDL.
 */
import { PublicKey, SystemProgram, TransactionInstruction, type AccountMeta } from '@solana/web3.js';
import { discriminator, encode, type Field, type Ty } from './codec.ts';
import { EQUIP_CONFIG, PARAM_FIELDS, SCORE_WEIGHTS, SLOT_BOUNDS } from './accounts.ts';
import {
  ARMORY_EVENT_AUTHORITY, ARMORY_ID, ITEMS_EVENT_AUTHORITY, ITEMS_ID, LAUNCH_ID, SWAP_ID, TOKEN_ID, WAR_CONFIG, WAR_EVENT_AUTHORITY, WAR_ID,
  equipStateAddress, forgeCounterAddress, holdingAddr, poolCutsAddress, proposalAddress, questMarkAddress, rollAddress, royaltyOwner,
  slotStateAddress, voteAddress, warChestAddress, warStateAddress,
} from './addresses.ts';

const params = (): Ty => ({ array: ['u32', PARAM_FIELDS] });

/** Argument schemas by program and instruction (spec sections in the comments). */
export const ARGS: Record<string, Record<string, Field[]>> = {
  armory: {
    create_item: [['templateId', 'u16'], ['params', params()], ['royaltyBps', 'u16']], // 02 4.2
    claim_royalty: [['amount', 'u64']], // 02 5.2
    propose: [['slot', 'u8'], ['item', { option: 'pubkey' }], ['config', EQUIP_CONFIG]], // 02 6.2, 04 2.3
    vote: [['support', 'bool'], ['amount', 'u64']], // 02 6.3
    finalize: [], // 02 6.4
    execute: [], // 02 6.4
    cancel: [], // 02 6.5
    close_proposal: [], close_vote: [], // 02 6.6
    check_performance: [['slot', 'u8']], // 02 6.7
    forge: [], // 02 9.1
  },
  items: {
    settle_equip: [['slot', 'u8']], // 04 2.5
    init_raid_ledger: [], // 04 2.9
  },
  war: {
    init_war: [], record_funding: [], // 05 4, 5
    siege: [], counter_strike: [], raze: [], return_captured: [], // 05 6.2 to 6.5
    share_treaty_inflow: [], accrue_treaty_time: [], // 05 6.6, 6.7
    claim_bounty: [], // 05 7
    roll: [['nonce', 'u64']], reveal: [['nonce', 'u64']], cancel_roll: [['nonce', 'u64']], // 05 8.2
    claim_quest: [['questId', 'u8'], ['period', 'u32']], // 05 9
    open_season: [], // 05 10.2
    submit_candidate: [['season', 'u32']], finalize_season: [['season', 'u32']], // 05 10.3
    split_protocol_fees: [], // 05 10.4
    propose_season: [['number', 'u32'], ['startsAt', 'i64'], ['weights', SCORE_WEIGHTS], ['penalizeBesieged', 'bool']], // 05 11
    apply_config: [], close_quest_mark: [],
  },
  swap: {
    // 03 3.3
    swap_route: [['amountIn', 'u64'], ['minAmountOut', 'u64'], ['hops', { vec: { struct: [['direction', 'u8'], ['accounts', 'u8'], ['inHookAccounts', 'u8'], ['outHookAccounts', 'u8']] } }]],
  },
  launch: {
    // 03 4.3
    prepare_launch: [['name', 'string'], ['symbol', 'string'], ['uri', 'string'], ['slots', { vec: { struct: [
      ['kind', 'u8'], ['equipRule', 'u8'], ['bounds', SLOT_BOUNDS], ['noticeSecs', 'u32'], ['dataLen', 'u8'], ['launchItem', { option: 'pubkey' }], ['ruleData', { vec: 'u8' }],
    ] } }]],
    refresh_pool_registry: [],
  },
  token: {
    // 01 section 5 (M1 code)
    touch: [['slotIndex', 'u8'], ['payload', { vec: 'u8' }]],
  },
};

export const PROGRAMS: Record<string, PublicKey> = { armory: ARMORY_ID, items: ITEMS_ID, war: WAR_ID, swap: SWAP_ID, launch: LAUNCH_ID, token: TOKEN_ID };

/** Builds `program::name(args)` with the given accounts. */
export function hookwarsIx(program: string, name: string, args: Record<string, unknown>, keys: AccountMeta[]): TransactionInstruction {
  const fields = ARGS[program]?.[name];
  const pid = PROGRAMS[program];
  if (!fields || !pid) throw new Error(`unknown instruction ${program}::${name}`);
  const data = Buffer.concat([discriminator('global', name), encode({ struct: fields }, args)]);
  return new TransactionInstruction({ programId: pid, keys, data });
}

const w = (pubkey: PublicKey, isSigner = false): AccountMeta => ({ pubkey, isSigner, isWritable: true });
const r = (pubkey: PublicKey, isSigner = false): AccountMeta => ({ pubkey, isSigner, isWritable: false });
const eventTail = (authority: PublicKey, program: PublicKey): AccountMeta[] => [r(authority), r(program)];

/** 02 6.3: `vote(support, amount)`; tokens lock in place (R11). */
export function vote(voter: PublicKey, mint: PublicKey, slot: number, nonce: bigint, support: boolean, amount: bigint): TransactionInstruction {
  const proposal = proposalAddress(mint, slot, nonce);
  return hookwarsIx('armory', 'vote', { support, amount }, [
    w(voter, true), r(mint), w(proposal), w(voteAddress(proposal, voter)), w(holdingAddr(mint, voter)),
    r(TOKEN_ID), r(SystemProgram.programId), ...eventTail(ARMORY_EVENT_AUTHORITY, ARMORY_ID),
  ]);
}

/** 02 6.2: `propose(slot, item)` with the equip's targets and role (04 2.3). */
export function propose(proposer: PublicKey, mint: PublicKey, slot: number, nextNonce: bigint, item: PublicKey | null, targets: PublicKey[], role: number): TransactionInstruction {
  return hookwarsIx('armory', 'propose', { slot, item, config: { targets, role } }, [
    w(proposer, true), r(mint), w(slotStateAddress(mint, slot)), w(proposalAddress(mint, slot, nextNonce)),
    r(SystemProgram.programId), ...eventTail(ARMORY_EVENT_AUTHORITY, ARMORY_ID),
  ]);
}

/** 04 2.5: `settle_equip(slot)`, permissionless with a bounty; `extra` are the destination accounts the item's template needs. */
export function settleEquip(cranker: PublicKey, mint: PublicKey, slot: number, item: PublicKey, extra: AccountMeta[] = []): TransactionInstruction {
  const equip = equipStateAddress(mint, slot);
  return hookwarsIx('items', 'settle_equip', { slot }, [
    w(cranker, true), r(mint), w(equip), w(holdingAddr(mint, equip)), r(poolCutsAddress(mint)), r(item), w(royaltyOwner(item)),
    r(TOKEN_ID), ...extra, ...eventTail(ITEMS_EVENT_AUTHORITY, ITEMS_ID),
  ]);
}

/** 02 5.2: `claim_royalty(amount)` by the item's current holder. */
export function claimRoyalty(owner: PublicKey, item: PublicKey, itemMint: PublicKey, cutMint: PublicKey, amount: bigint): TransactionInstruction {
  const ro = royaltyOwner(item);
  return hookwarsIx('armory', 'claim_royalty', { amount }, [
    r(owner, true), r(item), r(itemMint), r(holdingAddr(itemMint, owner)), r(cutMint), r(ro), w(holdingAddr(cutMint, ro)),
    w(holdingAddr(cutMint, owner)), r(TOKEN_ID), ...eventTail(ARMORY_EVENT_AUTHORITY, ARMORY_ID),
  ]);
}

/** 02 9.1: `forge(item_a, item_b)`; refuse client-side while either item is equipped or has royalties to settle or claim (02 section 10). */
export function forge(forger: PublicKey, a: { item: PublicKey; itemMint: PublicKey }, b: { item: PublicKey; itemMint: PublicKey }, newItemMint: PublicKey, newItem: PublicKey): TransactionInstruction {
  return hookwarsIx('armory', 'forge', {}, [
    w(forger, true), w(a.item), w(a.itemMint), w(holdingAddr(a.itemMint, forger)), w(b.item), w(b.itemMint), w(holdingAddr(b.itemMint, forger)),
    w(newItemMint), w(newItem), w(forgeCounterAddress(forger)), r(ITEMS_ID), r(TOKEN_ID), r(SystemProgram.programId),
    ...eventTail(ARMORY_EVENT_AUTHORITY, ARMORY_ID),
  ]);
}

/** 05 7: `claim_bounty`; `touchAccounts` are the token touch accounts with the Raid slot's slice. */
export function claimBounty(owner: PublicKey, mint: PublicKey, warOrdersItem: PublicKey, warOrdersTemplate: PublicKey, touchAccounts: AccountMeta[]): TransactionInstruction {
  return hookwarsIx('war', 'claim_bounty', {}, [
    w(owner, true), w(holdingAddr(mint, owner)), r(mint), w(warStateAddress(mint)), w(warChestAddress(mint)), r(warOrdersItem), r(warOrdersTemplate),
    r(WAR_CONFIG), ...touchAccounts, ...eventTail(WAR_EVENT_AUTHORITY, WAR_ID),
  ]);
}

/** 05 8.2: `roll(mint, nonce)`. */
export function roll(owner: PublicKey, mint: PublicKey, nonce: bigint, oracleAccounts: AccountMeta[], touchAccounts: AccountMeta[]): TransactionInstruction {
  const holding = holdingAddr(mint, owner);
  return hookwarsIx('war', 'roll', { nonce }, [
    w(owner, true), w(holding), r(mint), w(rollAddress(holding, nonce)), r(WAR_CONFIG), ...oracleAccounts, ...touchAccounts,
    r(SystemProgram.programId), ...eventTail(WAR_EVENT_AUTHORITY, WAR_ID),
  ]);
}

/** 05 9: `claim_quest(quest_id, period)`. For the Forge quest the mark's mint is the default key. */
export function claimQuest(owner: PublicKey, mint: PublicKey, season: number, questId: 1 | 2, period: number, touchAccounts: AccountMeta[]): TransactionInstruction {
  const markMint = questId === 2 ? PublicKey.default : mint;
  return hookwarsIx('war', 'claim_quest', { questId, period }, [
    w(owner, true), w(holdingAddr(mint, owner)), r(mint), w(questMarkAddress(season, markMint, owner)), r(forgeCounterAddress(owner)),
    r(WAR_CONFIG), ...touchAccounts, r(SystemProgram.programId), ...eventTail(WAR_EVENT_AUTHORITY, WAR_ID),
  ]);
}

/** 05 section 5: `init_war(mint)`, permissionless. */
export function initWar(payer: PublicKey, mint: PublicKey, launch: PublicKey): TransactionInstruction {
  return hookwarsIx('war', 'init_war', {}, [
    w(payer, true), r(mint), r(launch), w(warStateAddress(mint)), r(warChestAddress(mint)), r(WAR_CONFIG), r(TOKEN_ID),
    r(SystemProgram.programId), ...eventTail(WAR_EVENT_AUTHORITY, WAR_ID),
  ]);
}

/** 01 section 5: `touch(holding, slot_index, payload)` (M1). */
export function touch(authority: PublicKey, mint: PublicKey, owner: PublicKey, slotIndex: number, payload: Buffer, slice: AccountMeta[]): TransactionInstruction {
  return hookwarsIx('token', 'touch', { slotIndex, payload: [...payload] }, [r(authority, true), r(mint), w(holdingAddr(mint, owner)), ...slice]);
}

