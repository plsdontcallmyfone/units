// Changed by Hookwars: new file (gating, docs/spec/18-gating.md): template supply and the external gate.
/**
 * Template supply (part A): the `Supply` account of a tracked template and the admin builders.
 * The external gate (part C): addresses, builders and the accounts a Token-2022 transfer of a
 * gated mint resolves (the same list Token-2022 reads from the gate's extra-account-metas).
 */
import { PublicKey, TransactionInstruction, type AccountMeta } from '@solana/web3.js';
import { PROGRAM_IDS } from '@hookwars/shared';
import { ARMORY_ID, ITEMS_ID, holdingAddr, templateAddress } from './addresses.ts';
import { idlAccountCodec, idlIx } from './from-idl.ts';

export const GATE_ID = new PublicKey(PROGRAM_IDS.gate);
/** SPL Token-2022. */
export const TOKEN_2022_ID = new PublicKey('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');

const u16le = (n: number): Buffer => { const b = Buffer.alloc(2); b.writeUInt16LE(n); return b; };
const ro = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: false });
const rw = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: true });

// ---------------------------------------------------------------- supply (part A) --

/** `Template.supply_flags` bit 0: the template has a `Supply`. */
export const SUPPLY_TRACKED = 1;
/** `Supply.minter_rule`. */
export const MINTER_RULE = { authorOnly: 0, openUntilCap: 1 } as const;

/** `["supply", template_id]` under the armory. */
export function supplyAddress(templateId: number): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('supply'), u16le(templateId)], ARMORY_ID)[0];
}

export interface SupplyView {
  templateId: number;
  maxSupply: number;
  lootReserve: number;
  issued: number;
  drops: number;
  forged: number;
  burned: number;
  minterRule: number;
  minter: PublicKey;
}

/** Decodes a `Supply` account. */
export function decodeSupply(data: Buffer): SupplyView {
  return idlAccountCodec<SupplyView>('armory', 'Supply').decode(data);
}

/** Copies in circulation: made minus burned. */
export function circulating(s: SupplyView): number {
  return Math.max(0, s.issued + s.drops + s.forged - s.burned);
}

/** What authors may still issue (`null` when uncapped). */
export function authorRoom(s: SupplyView): number | null {
  return s.maxSupply === 0 ? null : Math.max(0, s.maxSupply - s.lootReserve - s.issued);
}

/** What drops may still make (`null` when uncapped). */
export function dropRoom(s: SupplyView): number | null {
  return s.maxSupply === 0 ? null : Math.max(0, s.maxSupply - s.issued - s.drops);
}

/** `hand_over_minter(new_minter)`: the current minter hands issuance to another wallet. */
export function handOverMinter(minter: PublicKey, templateId: number, newMinter: PublicKey): TransactionInstruction {
  return idlIx('armory', 'hand_over_minter', { minter, supply: supplyAddress(templateId) }, { newMinter });
}

/** `init_supply(...)` (admin; queued like the other template setters: send `queueFor` of it first). */
export function initSupply(admin: PublicKey, templateId: number, maxSupply: number, lootReserve: number, minterRule: number, minter: PublicKey, queued: PublicKey): TransactionInstruction {
  return idlIx('armory', 'init_supply', { admin, template: templateAddress(templateId), supply: supplyAddress(templateId), queued }, { templateId, maxSupply, lootReserve, minterRule, minter });
}

/** `set_supply_cap(...)` (admin; queued). */
export function setSupplyCap(admin: PublicKey, templateId: number, maxSupply: number, lootReserve: number, queued: PublicKey): TransactionInstruction {
  return idlIx('armory', 'set_supply_cap', { admin, supply: supplyAddress(templateId), queued }, { templateId, maxSupply, lootReserve });
}

// ---------------------------------------------------------------- the gate (part C) --

export const GATE_BINDING_KIND = { licence: 0, owned: 1 } as const;
/** `["items-signer"]` under the gate. */
export const GATE_ITEMS_SIGNER = PublicKey.findProgramAddressSync([Buffer.from('items-signer')], GATE_ID)[0];

export function gateConfigAddress(): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('gate-config')], GATE_ID)[0];
}
export function mintGateAddress(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('gate-mint'), mint.toBuffer()], GATE_ID)[0];
}
export function gateHolderAddress(mint: PublicKey, owner: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('gate-holder'), mint.toBuffer(), owner.toBuffer()], GATE_ID)[0];
}
export function gateVaultAddress(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('gate-vault'), mint.toBuffer()], GATE_ID)[0];
}
export function extraMetasAddress(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('extra-account-metas'), mint.toBuffer()], GATE_ID)[0];
}

export interface GateBinding {
  slot: number;
  item: PublicKey;
  itemMint: PublicKey;
  templateId: number;
  kind: number;
  proof: PublicKey;
  role: number;
  targets: PublicKey[];
  dataOffset: number;
  dataBytes: number;
  generation: number;
  extras: PublicKey[];
  boundAt: bigint;
}

export interface MintGateView {
  mint: PublicKey;
  authority: PublicKey;
  venue: PublicKey;
  strict: boolean;
  generation: number;
  bindings: GateBinding[];
  registeredAt: bigint;
}

/** Decodes a `MintGate`. */
export function decodeMintGate(data: Buffer): MintGateView {
  return idlAccountCodec<MintGateView>('gate', 'MintGate').decode(data);
}

/** `register_mint(venue, strict)`: signed by the mint's transfer-hook authority. */
export function gateRegisterMint(payer: PublicKey, authority: PublicKey, mint: PublicKey, venue: PublicKey, strict: boolean | null): TransactionInstruction {
  return idlIx('gate', 'register_mint', {
    payer, authority, config: gateConfigAddress(), mint, mintGate: mintGateAddress(mint), extraMetas: extraMetasAddress(mint),
  }, { venue, strict });
}

/** `set_venue(venue, strict)`: the mint's gate authority. */
export function gateSetVenue(authority: PublicKey, mint: PublicKey, venue: PublicKey, strict: boolean): TransactionInstruction {
  return idlIx('gate', 'set_venue', { authority, mintGate: mintGateAddress(mint) }, { venue, strict });
}

/** The licence a gated mint binds an item with (`["license", item, mint]` under the market). */
export function gateLicenceProof(item: PublicKey, mint: PublicKey, marketId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('license'), item.toBuffer(), mint.toBuffer()], marketId)[0];
}

/** The gate vault's holding of an item mint (the owned binding kind's proof). */
export function gateVaultHolding(itemMint: PublicKey, mint: PublicKey): PublicKey {
  return holdingAddr(itemMint, gateVaultAddress(mint));
}

/** `bind(slot, kind, targets, role)`: `extras` are the items program's further accounts for the
 * item (the composite's module list, module extras, `Wear`). */
export function gateBind(authority: PublicKey, mint: PublicKey, slot: number, kind: number, item: PublicKey, proof: PublicKey, targets: PublicKey[], extras: PublicKey[], role = 0): TransactionInstruction {
  return idlIx('gate', 'bind', {
    payer: authority, authority, config: gateConfigAddress(), mintGate: mintGateAddress(mint), extraMetas: extraMetasAddress(mint), item, proof,
  }, { slot, bindingKind: kind, targets, role }, extras.map(ro));
}

/** `unbind(slot)`: the gate authority any time, anyone once the binding lapsed. */
export function gateUnbind(signer: PublicKey, mint: PublicKey, slot: number, proof: PublicKey): TransactionInstruction {
  return idlIx('gate', 'unbind', { signer, mintGate: mintGateAddress(mint), extraMetas: extraMetasAddress(mint), proof }, { slot });
}

/** `open_holder(owner)`: anyone pays a wallet's holder memory for a gated mint. */
export function gateOpenHolder(payer: PublicKey, mint: PublicKey, owner: PublicKey): TransactionInstruction {
  return idlIx('gate', 'open_holder', { payer, mintGate: mintGateAddress(mint), holder: gateHolderAddress(mint, owner) }, { owner });
}

/** The hook's accounts a Token-2022 `TransferChecked` of a gated mint carries, in the order the
 * gate's extra-account-metas list gives them, then the gate program and the list itself. */
export function gateTransferAccounts(mint: PublicKey, gate: MintGateView, sourceOwner: PublicKey, destinationOwner: PublicKey): AccountMeta[] {
  const v: AccountMeta[] = [
    ro(mintGateAddress(mint)),
    ro(ITEMS_ID),
    ro(GATE_ITEMS_SIGNER),
    rw(gateHolderAddress(mint, sourceOwner)),
    rw(gateHolderAddress(mint, destinationOwner)),
  ];
  for (const b of gate.bindings) {
    v.push(ro(b.item), ro(b.proof), ...b.extras.map(ro));
  }
  v.push(ro(GATE_ID), ro(extraMetasAddress(mint)));
  return v;
}

/** Templates whose items can run on an external token: no token-side cut, a token-side callback,
 * the items program's own code (spec 18 section 3.3). The gate checks the item's manifest at bind;
 * this list is what the app offers. */
export const GATEABLE_TEMPLATES: readonly number[] = [17, 18, 19, 20, 22, 26, 39];
