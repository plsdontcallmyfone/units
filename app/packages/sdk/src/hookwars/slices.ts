/**
 * Slot-aware hook account resolution (docs/spec/06-app.md 4.3) for the token program as built in
 * M1 (`programs/bordrless_token/src/slots.rs`): a transfer, burn or mint carries one slice per slot
 * it calls, in slot order, each `[program, hook_signer, extras ...]` with exactly `extra_count`
 * extras. A slot is called when it is filled, not `War`, and subscribes to the operation
 * (`slot_flags`); a transfer out of a protocol vault (R16) and a mint call only the `Locked` slot.
 * The signer is `["hook-authority", program]` under the token program. An item's extras come from
 * its registry `["bordrless-hook-accounts", mint, item]` under the item's program (01 section 7),
 * resolved against `[signer, mint, source, destination, authority]` as upstream resolves them.
 */
import { PublicKey, type AccountMeta, type Connection } from '@solana/web3.js';
import { decodeHookAccountList, kitHookExtras, resolveHookAccounts } from '../hooks.ts';
import { KIT_PROGRAM } from '../addresses.ts';
import { activeSlots, decodeSlotMint, type SlotData, type SlotMintData } from './accounts.ts';
import { TOKEN_ID, tokenHookSigner } from './addresses.ts';

export type SlotOp = 'transfer' | 'protocolTransfer' | 'burn' | 'mint';

/** crates/bordrless-hook `slot_kind` and `slot_flags`. */
export const SLOT_KIND = { FEE: 0, REWARD: 1, DEFENSE: 2, RELATION: 3, POOL: 4, LOCKED: 5, WAR: 6 } as const;
export const SLOT_FLAGS = {
  BEFORE_TRANSFER: 1 << 0, AFTER_TRANSFER: 1 << 1, BEFORE_MINT: 1 << 2, AFTER_MINT: 1 << 3, BEFORE_BURN: 1 << 4, AFTER_BURN: 1 << 5,
  TRANSFER_RETURNS_DELTA: 1 << 6, WRITES_HOOK_DATA: 1 << 7, ANSWERS_TOUCH: 1 << 8,
} as const;
const TRANSFER = SLOT_FLAGS.BEFORE_TRANSFER | SLOT_FLAGS.AFTER_TRANSFER;
const BURN = SLOT_FLAGS.BEFORE_BURN | SLOT_FLAGS.AFTER_BURN;
const MINT = SLOT_FLAGS.BEFORE_MINT | SLOT_FLAGS.AFTER_MINT;

export function isFilled(slot: SlotData): boolean {
  return !slot.program.equals(PublicKey.default);
}

/** `slots::is_called` (M1). */
export function isCalled(slot: SlotData, op: SlotOp): boolean {
  if (!isFilled(slot) || slot.kind === SLOT_KIND.WAR) return false;
  const locked = slot.kind === SLOT_KIND.LOCKED;
  switch (op) {
    case 'transfer': return (slot.flags & TRANSFER) !== 0;
    case 'protocolTransfer': return locked && (slot.flags & TRANSFER) !== 0;
    case 'burn': return (slot.flags & BURN) !== 0;
    case 'mint': return locked && (slot.flags & MINT) !== 0;
  }
}

export interface SlotSlice { slot: number; program: PublicKey; signer: PublicKey; extras: AccountMeta[] }
export interface TokenHookSlices { mint: PublicKey; slices: SlotSlice[] }

export interface SliceArgs {
  mint: PublicKey; source: PublicKey; destination: PublicKey; authority: PublicKey;
  sourceOwner?: PublicKey; destinationOwner?: PublicKey;
  op?: SlotOp;
  /** A prepared launch's deposit, before the kit has written its registry: the kit slot's extras
   * are known without it (`bordrless_kit::client::hook_extras`), the reward vault with holder
   * rewards or `null` without (found on the devnet drill, 2026-10-10). */
  kitRewardVault?: PublicKey | null;
}

/** The registry of a slot's item: `["bordrless-hook-accounts", mint, item]` under the item's program;
 * a Locked slot keeps upstream's `["bordrless-hook-accounts", mint]` under its program. */
export function slotRegistryAddress(slot: SlotData, mint: PublicKey): PublicKey {
  const seeds = [Buffer.from('bordrless-hook-accounts'), mint.toBuffer()];
  if (slot.kind !== SLOT_KIND.LOCKED) seeds.push(slot.item.toBuffer());
  return PublicKey.findProgramAddressSync(seeds, slot.program)[0];
}

/** Resolves the slices from a decoded mint and its registries (pure; tests feed registries in). */
export function resolveSlices(mint: SlotMintData, args: SliceArgs, registries: Map<string, Buffer | null>): TokenHookSlices {
  const op = args.op ?? 'transfer';
  const slices: SlotSlice[] = [];
  activeSlots(mint).forEach((slot, i) => {
    if (!isCalled(slot, op)) return;
    const signer = tokenHookSigner(slot.program);
    let extras: AccountMeta[] = [];
    if (slot.extraCount > 0) {
      const reg = registries.get(slotRegistryAddress(slot, args.mint).toBase58());
      const list = reg ? decodeHookAccountList(reg) : null;
      if (!list && args.kitRewardVault !== undefined && slot.program.equals(KIT_PROGRAM)) {
        slices.push({ slot: i, program: slot.program, signer, extras: kitHookExtras(args.mint, args.kitRewardVault) });
        return;
      }
      if (!list) throw new Error(`slot ${i}: its registry is missing; the slot takes ${slot.extraCount} extras`);
      extras = resolveHookAccounts(list, [signer, args.mint, args.source, args.destination, args.authority], args.sourceOwner, args.destinationOwner);
      if (extras.length !== slot.extraCount) throw new Error(`slot ${i}: registry lists ${extras.length} extras, the slot takes ${slot.extraCount} (SlotAccountsMismatch)`);
    }
    slices.push({ slot: i, program: slot.program, signer, extras });
  });
  return { mint: args.mint, slices };
}

/** Reads the mint's slot table and each called slot's registry, then resolves (06 4.3). */
export async function fetchTokenHookSlices(connection: Connection, args: SliceArgs): Promise<TokenHookSlices | null> {
  const info = await connection.getAccountInfo(args.mint, 'confirmed');
  if (!info || !info.owner.equals(TOKEN_ID)) return null;
  const mint = decodeSlotMint(info.data);
  const op = args.op ?? 'transfer';
  const called = activeSlots(mint).filter((s) => isCalled(s, op) && s.extraCount > 0);
  const keys = called.map((s) => slotRegistryAddress(s, args.mint));
  const infos = keys.length ? await connection.getMultipleAccountsInfo(keys, 'confirmed') : [];
  const registries = new Map<string, Buffer | null>();
  keys.forEach((k, i) => registries.set(k.toBase58(), infos[i]?.data ?? null));
  return resolveSlices(mint, args, registries);
}

/** The remaining accounts of a token instruction: every slice flattened in slot order. */
export function sliceAccounts(s: TokenHookSlices): AccountMeta[] {
  return s.slices.flatMap((x) => [
    { pubkey: x.program, isSigner: false, isWritable: false },
    { pubkey: x.signer, isSigner: false, isWritable: false },
    ...x.extras,
  ]);
}
