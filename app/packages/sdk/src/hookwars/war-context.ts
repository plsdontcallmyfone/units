/**
 * What a war step needs to know about a token before it can be built: the War orders the War slot
 * names (05 6.0), the Raid slot's index and its touch extras (05 sections 7 to 9). Read from the
 * mint's slot table, the armory `Item` records and the items program's registries.
 */
import { PublicKey, type AccountMeta, type Connection } from '@solana/web3.js';
import { decodeHookAccountList, resolveHookAccounts } from '../hooks.ts';
import { activeSlots, decodeSlotMint, holdingCodec, itemCodec, type SlotData } from './accounts.ts';
import { TOKEN_ID, WAR_SIGNER, holdingAddr, templateAddress, tokenHookSigner } from './addresses.ts';
import { SLOT_KIND, slotRegistryAddress } from './slices.ts';
import type { Orders } from './instructions.ts';

/** Template ids of crates/hookwars-common `template_id`. */
export const TEMPLATE_ID = { RAID: 1, SHIELD: 2, WALL: 3, SPY: 4, TREATY: 5, TRIBUTE: 6, HALF_LIFE: 7, TRANSFER_FEE: 8, WAR_ORDERS: 9 } as const;

export interface WarContext {
  /** The War orders, or null when the War slot is empty or absent. */
  orders: Orders | null;
  /** The Raid slot index and the item in it, or null when no Raid item is equipped. */
  raid: { slot: number; item: PublicKey; program: PublicKey } | null;
}

/** Reads the slot table and the items in it. */
export async function fetchWarContext(connection: Connection, mint: PublicKey): Promise<WarContext> {
  const info = await connection.getAccountInfo(mint, 'confirmed');
  if (!info || !info.owner.equals(TOKEN_ID)) throw new Error('not a token mint');
  const m = decodeSlotMint(info.data);
  const slots = activeSlots(m).map((s, i) => ({ s, i })).filter(({ s }) => !s.item.equals(PublicKey.default));
  const items = slots.length ? await connection.getMultipleAccountsInfo(slots.map(({ s }) => s.item), 'confirmed') : [];
  let orders: Orders | null = null;
  let raid: WarContext['raid'] = null;
  slots.forEach(({ s, i }, k) => {
    const data = items[k]?.data;
    if (!data) return;
    let templateId: number;
    try { templateId = itemCodec().decode(data).templateId; } catch { return; }
    if (s.kind === SLOT_KIND.WAR && templateId === TEMPLATE_ID.WAR_ORDERS) orders = { item: s.item, template: templateAddress(templateId) };
    if (templateId === TEMPLATE_ID.RAID && raid === null) raid = { slot: i, item: s.item, program: s.program };
  });
  return { orders, raid };
}

/**
 * The extras a `touch` of `owner`'s holding through `slot` carries when the war program is the
 * caller (`["war-signer"]`), resolved as the token program resolves them for `touch`:
 * `[signer, mint, holding, holding, caller]` with the holding's owner on both sides.
 */
export async function fetchTouchExtras(connection: Connection, mint: PublicKey, slot: SlotData, owner: PublicKey, caller: PublicKey = WAR_SIGNER): Promise<AccountMeta[]> {
  if (slot.extraCount === 0) return [];
  const reg = await connection.getAccountInfo(slotRegistryAddress(slot, mint), 'confirmed');
  const list = reg ? decodeHookAccountList(reg.data) : null;
  if (!list) throw new Error('the slot registry is missing');
  const holding = holdingAddr(mint, owner);
  const extras = resolveHookAccounts(list, [tokenHookSigner(slot.program), mint, holding, holding, caller], owner, owner);
  if (extras.length !== slot.extraCount) throw new Error(`the registry lists ${extras.length} extras, the slot takes ${slot.extraCount}`);
  return extras;
}

/** The slot at `index` of `mint`. */
export async function fetchSlot(connection: Connection, mint: PublicKey, index: number): Promise<SlotData> {
  const info = await connection.getAccountInfo(mint, 'confirmed');
  if (!info) throw new Error('no such mint');
  const s = activeSlots(decodeSlotMint(info.data))[index];
  if (!s) throw new Error(`the mint has no slot ${index}`);
  return s;
}

/** A holder's current vote lock and balance, for the voting panel. */
export async function fetchHolding(connection: Connection, mint: PublicKey, owner: PublicKey) {
  const info = await connection.getAccountInfo(holdingAddr(mint, owner), 'confirmed');
  return info ? holdingCodec.decode(info.data) : null;
}
