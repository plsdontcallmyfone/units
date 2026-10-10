// Changed by Hookwars: D-4 (17-randomness). Switchboard On-Demand randomness for loot rolls: the
// program ids the war config may name, the randomness account the war program reads, the draw a
// revealed value makes (the same as `hookwars_war::instructions::loot::draw`) and `mint_loot`'s
// accounts for the drawn item.
import { PublicKey, type AccountMeta } from '@solana/web3.js';
import { idlIx } from './from-idl.ts';
import { LOOT_SIGNER, holdingAddr, itemAddress, itemMintAddress, templateAddress } from './addresses.ts';
import type { War } from './idl-types.gen.ts';

/** Switchboard On-Demand, mainnet (`switchboard-on-demand` 0.13.0 `ON_DEMAND_MAINNET_PID`). */
export const SWITCHBOARD_MAINNET = new PublicKey('SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv');
/** Switchboard On-Demand, devnet (`ON_DEMAND_DEVNET_PID`). */
export const SWITCHBOARD_DEVNET = new PublicKey('Aio4gaXjXzJNVLtzwtNVmSqGKpANtXhybbkhtAC94ji2');

/** Whether the war config's randomness program is Switchboard (read directly, no adapter). */
export const isSwitchboard = (program: PublicKey): boolean => program.equals(SWITCHBOARD_MAINNET) || program.equals(SWITCHBOARD_DEVNET);

/** The fields of Switchboard's `RandomnessAccountData` the war program reads (408 bytes). */
export interface SbRandomness { authority: PublicKey; seedSlot: bigint; revealSlot: bigint; value: Uint8Array }

export const SB_RANDOMNESS_DISCRIMINATOR = Uint8Array.from([10, 66, 229, 135, 220, 239, 217, 114]);
export const SB_RANDOMNESS_LEN = 408;

/** Decodes a Switchboard randomness account: `disc | authority | queue | seed_slothash | seed_slot | oracle | reveal_slot | value | reserved`. */
export function decodeSbRandomness(data: Uint8Array): SbRandomness {
  if (data.length < SB_RANDOMNESS_LEN || !SB_RANDOMNESS_DISCRIMINATOR.every((b, i) => data[i] === b)) {
    throw new Error('not a Switchboard randomness account');
  }
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  return {
    authority: new PublicKey(data.subarray(8, 40)),
    seedSlot: view.getBigUint64(104, true),
    revealSlot: view.getBigUint64(144, true),
    value: data.slice(152, 184),
  };
}

/** Encodes the same layout (fixtures and tests). */
export function encodeSbRandomness(r: SbRandomness): Uint8Array {
  const d = new Uint8Array(SB_RANDOMNESS_LEN);
  d.set(SB_RANDOMNESS_DISCRIMINATOR, 0);
  d.set(r.authority.toBytes(), 8);
  const view = new DataView(d.buffer);
  view.setBigUint64(104, r.seedSlot, true);
  view.setBigUint64(144, r.revealSlot, true);
  d.set(r.value, 152);
  return d;
}

/**
 * What a 32-byte value draws from a loot table's active entries: a template by weight from bytes
 * 0..8, then each field uniformly in its range from its own two bytes (8 + 2i). `null` when the
 * table has no weight.
 */
export function drawLoot(entries: readonly War.LootEntry[], value: Uint8Array): { templateId: number; params: number[] } | null {
  const total = entries.reduce((a, e) => a + BigInt(e.weight), 0n);
  if (total === 0n) return null;
  const view = new DataView(value.buffer, value.byteOffset, value.byteLength);
  let pick = view.getBigUint64(0, true) % total;
  const entry = entries.find((e) => {
    const w = BigInt(e.weight);
    if (pick < w) return true;
    pick -= w;
    return false;
  });
  if (!entry) return null;
  const params = entry.ranges.map((range, i) => {
    const r = BigInt(view.getUint16(8 + 2 * i, true));
    const span = BigInt(range.max - range.min) + 1n;
    return range.min + Number((r * span) >> 16n);
  });
  return { templateId: entry.templateId, params };
}

/**
 * `mint_loot`'s accounts after `loot_signer`, `payer` and `owner` (what `reveal` forwards as its
 * remaining accounts), for an item of `templateId` minted to `owner` as item number `itemsMinted`.
 */
export function mintLootExtras(payer: PublicKey, owner: PublicKey, templateId: number, params: number[], itemsMinted: bigint): AccountMeta[] {
  const itemMint = itemMintAddress(itemsMinted);
  const ix = idlIx('armory', 'mint_loot', {
    lootSigner: LOOT_SIGNER, payer, owner, template: templateAddress(templateId), itemMint, item: itemAddress(itemMint),
    recipientHolding: holdingAddr(itemMint, owner),
  }, { templateId, params });
  return ix.keys.slice(3).map((k) => ({ ...k, isSigner: false }));
}
