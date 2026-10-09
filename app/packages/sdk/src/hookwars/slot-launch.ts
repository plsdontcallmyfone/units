/**
 * Slot launches and raids on the real programs (M3a, M3b): the launchpad's four steps
 * (`prepare_launch`, `equip_prepared` forwarding the armory's `equip_launch`,
 * `create_prepared_launch`, `refresh_pool_registry`), the war program's `init_war`, the items
 * program's `init_raid_ledger` and `settle_equip`, and the DEX's `swap_route`. Built from the
 * generated IDLs (`idlIx`); remaining accounts follow the programs' own Rust clients
 * (`bordrless_launch::client::slots`, `bordrless_swap::client::swap_route`,
 * `programs/tests/tests/e2e.rs`).
 */
import { PublicKey, TransactionInstruction, type AccountMeta, type Connection } from '@solana/web3.js';
import { activeSlots, decodeSlotMint } from './accounts.ts';
import { decodeHookAccountList, resolveHookAccounts } from '../hooks.ts';
import { idlIx } from './from-idl.ts';
import {
  ARMORY_ID, ITEMS_ID, LAUNCH_ID, SWAP_ID, TOKEN_ID,
  armoryCallerAddress, armoryConfigAddress, equipStateAddress, holdingAddr, itemRegistryAddress, launchAddr, poolCutsAddress,
  preparedLaunchAddress, raidLedgerAddress, royaltyOwner, slotAuthority, slotStateAddress, templateAddress, tokenHookSigner,
} from './addresses.ts';
import * as up from '../addresses.ts';
import { PROGRAM_IDS } from '@hookwars/shared';
import { launch as upLaunch, type CreateLaunchArgs } from '../instructions.ts';

const ro = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: false });
const rw = (pubkey: PublicKey, isSigner = false): AccountMeta => ({ pubkey, isSigner, isWritable: true });

/** `["hook-authority", program]` under the launchpad: its signer of a pool item's callbacks. */
export const launchItemSigner = (program: PublicKey): PublicKey =>
  PublicKey.findProgramAddressSync([Buffer.from('hook-authority'), program.toBuffer()], LAUNCH_ID)[0];
/** `["hook-authority", hook]` under the DEX: its signer of a pool hook's callbacks. */
export const dexHookSigner = (hook: PublicKey): PublicKey =>
  PublicKey.findProgramAddressSync([Buffer.from('hook-authority'), hook.toBuffer()], SWAP_ID)[0];
/** The launch pool's own registry (`["bordrless-hook-accounts", pool]` under the launchpad). */
export const launchRegistryAddress = (pool: PublicKey): PublicKey =>
  PublicKey.findProgramAddressSync([Buffer.from('bordrless-hook-accounts'), pool.toBuffer()], LAUNCH_ID)[0];

/** A slot of `PrepareLaunchArgs.slots` (`bordrless_token::instructions::SlotInit`). */
export interface SlotInitArgs {
  kind: number;
  equipRule: number;
  bounds: { maxCutBps: number; mayRefuse: boolean; mayWriteData: boolean; mayAnswerTouch: boolean; mayBurn: boolean };
  dataLen: number;
  lockedProgram: PublicKey | null;
  lockedFlags: number;
  lockedExtraCount: number;
}

export interface PrepareLaunchArgs {
  name: string; symbol: string; uri: string; creatorFeeBps: number;
  rules: CreateLaunchArgs['rules'];
  slots: SlotInitArgs[];
}

/** Step 1, `prepare_launch`: the slot mint and the `PreparedLaunch`. The mint is a fresh keypair that signs. */
export function prepareLaunch(creator: PublicKey, mint: PublicKey, args: PrepareLaunchArgs): TransactionInstruction {
  return idlIx('launch', 'prepare_launch', {
    creator, config: up.LAUNCH_CONFIG, mint, prepared: preparedLaunchAddress(mint),
    tokenProgram: TOKEN_ID, tokenEventAuthority: up.TOKEN_EVENT_AUTHORITY,
  }, { args });
}

/** One slot's launch entry (`hookwars_armory::LaunchEquip`). */
export interface LaunchEquip {
  slot: number;
  item: PublicKey | null;
  config: { targets: PublicKey[]; role: number };
  noticeSecs: number;
  rule: null | { metric: number; windowSecs: number; baseWindowSecs: number; op: number; ratioBps: number; holdSecs: number };
}

/** What `equip_launch` needs to know about the item it equips (read from its `Item` account). */
export interface EquipItemInfo { item: PublicKey; templateId: number; tokenCuts: boolean; poolCuts: boolean; composite: boolean }

/** The armory's `equip_launch` of one slot, signed by the launchpad as `["armory-caller", mint]`. */
export function equipLaunch(payer: PublicKey, mint: PublicKey, quoteMint: PublicKey, entry: LaunchEquip, info: EquipItemInfo | null): TransactionInstruction {
  const equipState = equipStateAddress(mint, entry.slot);
  const vault = holdingAddr(mint, equipState);
  const owner = info ? royaltyOwner(info.item) : null;
  const pays = info ? info.tokenCuts || info.poolCuts : false;
  return idlIx('armory', 'equip_launch', {
    launchCaller: armoryCallerAddress(mint), config: armoryConfigAddress(), slotState: slotStateAddress(mint, entry.slot),
    payer, tokenMint: mint, slotAuthority: slotAuthority(mint), equipState,
    newItem: info?.item ?? null,
    newTemplate: info ? templateAddress(info.templateId) : null,
    templateProgram: info ? ITEMS_ID : null,
    templateProgramdata: info ? PublicKey.findProgramAddressSync([ITEMS_ID.toBuffer()], new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111'))[0] : null,
    registry: info ? itemRegistryAddress(mint, info.item) : null,
    newEquipVault: info?.tokenCuts ? vault : null,
    royaltyOwner: pays ? owner : null,
    royaltyHoldingToken: pays && owner ? holdingAddr(mint, owner) : null,
    quoteMint: pays ? quoteMint : null,
    royaltyHoldingQuote: pays && owner ? holdingAddr(quoteMint, owner) : null,
    newComposite: info?.composite ? PublicKey.findProgramAddressSync([Buffer.from('composite'), info.item.toBuffer()], ARMORY_ID)[0] : null,
    itemsProgram: ITEMS_ID,
  }, { entry });
}

/** Step 2, `equip_prepared`: forwards one `equip_launch` for the creator who prepared the mint. */
export function equipPrepared(creator: PublicKey, mint: PublicKey, armoryIx: TransactionInstruction): TransactionInstruction {
  const caller = armoryCallerAddress(mint);
  const keys = armoryIx.keys.map((k) => (k.pubkey.equals(caller) ? { ...k, isSigner: false } : k));
  return idlIx('launch', 'equip_prepared', {
    creator, prepared: preparedLaunchAddress(mint), mint, armory: ARMORY_ID,
  }, { data: armoryIx.data }, keys);
}

/** Step 3, `create_prepared_launch`: upstream `create_launch`'s accounts, then the prepared launch,
 * the `PoolCuts` owner and holding, then `slices` (the mint's transfer slices, for the deposit). */
export function createPreparedLaunch(creator: PublicKey, mint: PublicKey, treasury: PublicKey, quoteMint: PublicKey, lpFeeBps: number, args: CreateLaunchArgs, slices: AccountMeta[]): TransactionInstruction {
  const base = upLaunch.createLaunch(creator, mint, treasury, quoteMint, lpFeeBps, args);
  const pc = poolCutsAddress(mint);
  const keys = [...base.keys, rw(preparedLaunchAddress(mint)), ro(pc), rw(holdingAddr(quoteMint, pc)), ...slices];
  const step = idlIx('launch', 'create_prepared_launch', {}, { args }).data;
  return new TransactionInstruction({ programId: LAUNCH_ID, keys, data: step });
}

/** Step 4, `refresh_pool_registry`: the launch pool's registry with every forwarded pool slot's item registry. */
export function refreshPoolRegistry(payer: PublicKey, mint: PublicKey, quoteMint: PublicKey, lpFeeBps: number, itemRegistries: PublicKey[]): TransactionInstruction {
  const pool = up.launchPoolAddress(mint, quoteMint, lpFeeBps);
  return idlIx('launch', 'refresh_pool_registry', {
    payer, launch: launchAddr(mint), mint, registry: launchRegistryAddress(pool),
  }, {}, itemRegistries.map(ro));
}

/** `init_raid_ledger` of a token with a Raid or Shield item (permissionless). */
export function initRaidLedger(payer: PublicKey, mint: PublicKey): TransactionInstruction {
  return idlIx('items', 'init_raid_ledger', { payer, mint, raidLedger: raidLedgerAddress(mint) });
}

/** The pool items' accounts of a launch's swap: per forwarded pool slot, the items program, the
 * launchpad's signer for it and the item's registry extras (`registryExtras` resolves them). */
export function poolItemAccounts(items: { program: PublicKey; extras: AccountMeta[] }[]): AccountMeta[] {
  return items.flatMap((i) => [ro(i.program), ro(launchItemSigner(i.program)), ...i.extras]);
}

/** The pool extras of a slot launch's swap after the launchpad's four: the `PoolCuts` holding, then the items. */
export function slotPoolExtras(mint: PublicKey, quoteMint: PublicKey, launchFour: AccountMeta[], items: AccountMeta[]): AccountMeta[] {
  return [...launchFour, rw(holdingAddr(quoteMint, poolCutsAddress(mint))), ...items];
}

/** One hop of `swap_route` (`bordrless_swap::client::RouteHop`). */
export interface RouteHop {
  pool: PublicKey; baseMint: PublicKey; quoteMint: PublicKey; traderBase: PublicKey; traderQuote: PublicKey;
  hookProgram: PublicKey | null; baseMintWritable: boolean; quoteMintWritable: boolean;
  direction: 0 | 1; inSlice: AccountMeta[]; outSlice: AccountMeta[]; poolExtras: AccountMeta[];
}

/** `swap_route`: each hop's nine fixed accounts, then its in slice, out slice and pool extras. */
export function swapRoute(trader: PublicKey, amountIn: bigint, minAmountOut: bigint, hops: RouteHop[], hookData: Buffer = Buffer.alloc(0)): TransactionInstruction {
  const remaining: AccountMeta[] = [];
  const hopArgs = hops.map((h) => {
    const group: AccountMeta[] = [
      rw(h.pool),
      h.baseMintWritable ? rw(h.baseMint) : ro(h.baseMint),
      h.quoteMintWritable ? rw(h.quoteMint) : ro(h.quoteMint),
      rw(up.vaultAddress(h.pool, h.baseMint)),
      rw(up.vaultAddress(h.pool, h.quoteMint)),
      rw(h.traderBase),
      rw(h.traderQuote),
      ro(h.hookProgram ?? SWAP_ID),
      ro(h.hookProgram ? dexHookSigner(h.hookProgram) : SWAP_ID),
    ];
    remaining.push(...group, ...h.inSlice, ...h.outSlice, ...h.poolExtras);
    return { direction: h.direction, accounts: group.length + h.inSlice.length + h.outSlice.length + h.poolExtras.length, inHookAccounts: h.inSlice.length, outHookAccounts: h.outSlice.length };
  });
  return idlIx('swap', 'swap_route', {
    trader, config: up.SWAP_CONFIG, tokenProgram: TOKEN_ID, tokenEventAuthority: up.TOKEN_EVENT_AUTHORITY,
  }, { args: { amountIn, minAmountOut, hops: hopArgs, hookData } }, remaining);
}

/** `settle_equip(slot)` (IDL): pays an item's royalty, the sender's bounty and each module's
 * destination; `dests` are `(token destination, pool destination)` per module. */
export function settleEquip(cranker: PublicKey, mint: PublicKey, slot: number, item: { key: PublicKey; tokenCuts: boolean; composite: boolean }, quoteMint: PublicKey, dests: [PublicKey, PublicKey][]): TransactionInstruction {
  const state = equipStateAddress(mint, slot);
  const owner = royaltyOwner(item.key);
  const cuts = poolCutsAddress(mint);
  return idlIx('items', 'settle_equip', {
    cranker, mint, item: item.key,
    composite: item.composite ? PublicKey.findProgramAddressSync([Buffer.from('composite'), item.key.toBuffer()], ARMORY_ID)[0] : ITEMS_ID,
    equipState: state, equipVault: item.tokenCuts ? holdingAddr(mint, state) : null,
    poolCuts: cuts, poolCutsHolding: holdingAddr(quoteMint, cuts), quoteMint,
    royaltyOwner: owner, royaltyToken: holdingAddr(mint, owner), royaltyQuote: holdingAddr(quoteMint, owner),
    crankerToken: holdingAddr(mint, cranker), crankerQuote: holdingAddr(quoteMint, cranker),
    armoryConfig: armoryConfigAddress(), tokenProgram: TOKEN_ID, tokenEventAuthority: up.TOKEN_EVENT_AUTHORITY,
  }, { slot }, dests.flatMap(([a, b]) => [rw(a), rw(b)]));
}

// ---------------------------------------------------------------- reading a launch's pool items --

/** `bordrless_launch::instructions::forwards`: a filled Pool or Relation slot with pool callbacks. */
export function forwards(slot: { kind: number; program: PublicKey; poolFlags: number }): boolean {
  const ITEM_POOL_BEFORE = 1;
  const ITEM_POOL_AFTER = 2;
  return !slot.program.equals(PublicKey.default)
    && (slot.kind === 4 || slot.kind === 3)
    && (slot.poolFlags & (ITEM_POOL_BEFORE | ITEM_POOL_AFTER)) !== 0;
}

/** The pool items' accounts of a launch's swap, read from its mint and each forwarded item's
 * registry (`[program, launchItemSigner(program), extras ...]` per forwarded slot, in slot order). */
export async function fetchPoolItems(connection: Connection, mint: PublicKey): Promise<{ accounts: AccountMeta[]; registries: PublicKey[] }> {
  const info = await connection.getAccountInfo(mint, 'confirmed');
  if (!info) throw new Error('no such mint');
  const m = decodeSlotMint(info.data);
  const slots = activeSlots(m).filter((s) => forwards(s as unknown as { kind: number; program: PublicKey; poolFlags: number }));
  const registries = slots.map((s) => itemRegistryAddress(mint, s.item));
  const infos = registries.length ? await connection.getMultipleAccountsInfo(registries, 'confirmed') : [];
  const accounts: AccountMeta[] = [];
  slots.forEach((s, i) => {
    const list = infos[i] ? decodeHookAccountList(infos[i]!.data) : null;
    if (!list) throw new Error(`the item in a pool slot has no registry; refresh it first`);
    const signer = tokenHookSigner(s.program);
    const extras = resolveHookAccounts(list, [signer, mint, mint, mint, mint]);
    accounts.push(ro(s.program), ro(launchItemSigner(s.program)), ...extras);
  });
  return { accounts, registries };
}

// ---------------------------------------------------------------- settlement destinations --

const T = { RAID: 1, SHIELD: 2, SPY: 4, TREATY: 5, TRIBUTE: 6, HALF_LIFE: 7, TRANSFER_FEE: 8, SIZE_TIERS: 10, SIDE_SKEW: 11, LAUNCH_DECAY: 12 } as const;
const warChestOf = (mint: PublicKey) => PublicKey.findProgramAddressSync([Buffer.from('war-chest'), mint.toBuffer()], new PublicKey(PROGRAM_IDS.war))[0];
const treatyInboxOf = (mint: PublicKey) => PublicKey.findProgramAddressSync([Buffer.from('treaty-inbox'), mint.toBuffer()], new PublicKey(PROGRAM_IDS.war))[0];

/** `settle_equip`'s destination pair of one module (`templates::token_destination`,
 * `templates::pool_destination`): the holding a token-side cut goes to (the items program's id when
 * it burns or has none) and the bridged-SOL holding a pool-side cut goes to. */
export function settleDestination(templateId: number, mint: PublicKey, quoteMint: PublicKey, targets: PublicKey[]): [PublicKey, PublicKey] {
  const token = templateId === T.TRANSFER_FEE && targets[0] ? holdingAddr(mint, targets[0]) : ITEMS_ID;
  let owner: PublicKey | null = null;
  if (([T.RAID, T.SHIELD, T.SPY, T.SIZE_TIERS, T.SIDE_SKEW, T.LAUNCH_DECAY] as number[]).includes(templateId)) owner = warChestOf(mint);
  else if ((templateId === T.TREATY || templateId === T.TRIBUTE) && targets[0]) owner = treatyInboxOf(targets[0]);
  return [token, owner ? holdingAddr(quoteMint, owner) : ITEMS_ID];
}
