// Changed by Hookwars: new file, the app side of protocol pass 4a (docs/spec/14-pass-4a.md section 7):
// the armory's admin queue, access modes (set_access, approve, revoke_approval, enforce_access), the
// access proof suffix on the equip paths, Hook Lab submissions, and the launchpad refresh tail.
import { createHash } from 'node:crypto';
import { PublicKey, type AccountMeta, type TransactionInstruction } from '@solana/web3.js';
import { ARMORY_ID, AGENTS_ID, LAUNCH_ID, MARKET_ID, SOCIAL_ID, equipStateAddress, holdingAddr, itemAddress, launchAddr, slotAuthority, slotStateAddress, templateAddress } from './addresses.ts';
import type { EquipChange } from './instructions.ts';
import { coderOf, idlIx } from './from-idl.ts';
import { encode } from './codec.ts';
import { idlArgs } from './from-idl.ts';

const s = (t: string) => Buffer.from(t);
const pda = (seeds: Buffer[], program: PublicKey) => PublicKey.findProgramAddressSync(seeds, program)[0];
const ro = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: false });
const rw = (pubkey: PublicKey): AccountMeta => ({ pubkey, isSigner: false, isWritable: true });

/** Access modes (`hookwars_common::access`). */
export const ACCESS = { OPEN: 0, GATED: 1, LICENSED: 2, LEASED: 3, EXCLUSIVE: 4 } as const;

export const accessPolicyAddress = (item: PublicKey) => pda([s('access'), item.toBuffer()], ARMORY_ID);
export const approvalAddress = (item: PublicKey, tokenMint: PublicKey) => pda([s('approval'), item.toBuffer(), tokenMint.toBuffer()], ARMORY_ID);
export const queuedAddress = (actionHash: Uint8Array) => pda([s('queued'), Buffer.from(actionHash)], ARMORY_ID);
export const templateSubmissionAddress = (program: PublicKey) => pda([s('submission-tpl'), program.toBuffer()], ARMORY_ID);
/** The market's `License` at `["license", item, token_mint]`. */
export const licenseAddress = (item: PublicKey, tokenMint: PublicKey) => pda([s('license'), item.toBuffer(), tokenMint.toBuffer()], MARKET_ID);
/** The market's `Lease` at `["lease", item]`. */
const leaseOf = (item: PublicKey) => pda([s('lease'), item.toBuffer()], MARKET_ID);

/** The bytes a gated armory instruction carries (discriminator and arguments). */
export function armoryIxData(name: string, args: Record<string, unknown>): Buffer {
  const ix = coderOf('armory').instruction(name);
  return Buffer.concat([Buffer.from(ix.discriminator), encode({ struct: idlArgs('armory', name) }, args)]);
}

/** `sha256(instruction data || bound keys)`: the admin queue's action hash (spec 14 section 1). */
export function adminActionHash(data: Uint8Array, bound: PublicKey[] = []): Buffer {
  const h = createHash('sha256').update(data);
  for (const k of bound) h.update(k.toBuffer());
  return h.digest();
}

/** `queue_admin(action_hash)`: opens the queue entry, ready after the armory's timelock. */
export function queueAdmin(admin: PublicKey, actionHash: Uint8Array): TransactionInstruction {
  return idlIx('armory', 'queue_admin', { admin, queued: queuedAddress(actionHash) }, { actionHash: Array.from(actionHash) });
}

/** `cancel_admin`: closes a queue entry. */
export function cancelAdmin(admin: PublicKey, actionHash: Uint8Array): TransactionInstruction {
  return idlIx('armory', 'cancel_admin', { admin, queued: queuedAddress(actionHash) });
}

/**
 * A gated admin instruction: the `queue_admin` to send first and the instruction itself (with its
 * trailing `queued` account) to send once the timelock has passed. `bound` is the template program
 * for `register_template` and `register_external_template`, empty otherwise.
 */
export function gated(admin: PublicKey, name: string, accounts: Record<string, PublicKey | null | undefined>, args: Record<string, unknown>, bound: PublicKey[] = []): { actionHash: Buffer; queue: TransactionInstruction; apply: TransactionInstruction } {
  const actionHash = adminActionHash(armoryIxData(name, args), bound);
  return { actionHash, queue: queueAdmin(admin, actionHash), apply: idlIx('armory', name, { admin, ...accounts, queued: queuedAddress(actionHash) }, args) };
}

/** `set_access_params(params)` (gated). */
export const setAccessParams = (admin: PublicKey, params: Record<string, unknown>) => gated(admin, 'set_access_params', {}, { params });

/** Suffixes `set_access` may take: the agent's passport and directive, and the Builder level read. */
export interface SetAccessSuffixes { agent?: { passport: PublicKey; directive: PublicKey }; level?: { wallet: PublicKey } }

/** `set_access(mode, exclusive, licence_terms)` by the item's holder (or its agent's key). */
export function setAccess(signer: PublicKey, o: { itemMint: PublicKey; templateId: number; holder: PublicKey; mode: number; exclusive: boolean; licenceTerms: { priceLamports: bigint; termSecs: number; per: number; maxLive: number } | null }, sfx: SetAccessSuffixes = {}): TransactionInstruction {
  const item = itemAddress(o.itemMint);
  const rest: AccountMeta[] = [];
  if (sfx.level) rest.push(ro(SOCIAL_ID), ro(pda([s('skills')], SOCIAL_ID)), ro(pda([s('profile'), sfx.level.wallet.toBuffer()], SOCIAL_ID)));
  if (sfx.agent) rest.push(ro(AGENTS_ID), ro(sfx.agent.passport), ro(sfx.agent.directive));
  return idlIx('armory', 'set_access', {
    signer, item, template: templateAddress(o.templateId), itemHolding: holdingAddr(o.itemMint, o.holder), policy: accessPolicyAddress(item), lease: leaseOf(item),
  }, { mode: o.mode, exclusive: o.exclusive, licenceTerms: o.licenceTerms }, rest);
}

/** `approve(token_mint)`: a Gated item's holder lets one token equip it. */
export function approveToken(signer: PublicKey, itemMint: PublicKey, holder: PublicKey, tokenMint: PublicKey): TransactionInstruction {
  const item = itemAddress(itemMint);
  return idlIx('armory', 'approve', { signer, item, itemHolding: holdingAddr(itemMint, holder), approval: approvalAddress(item, tokenMint) }, { tokenMint });
}

/** `revoke_approval`: with the `SlotState` of every slot of the token holding the item (its notice applies). */
export function revokeApproval(signer: PublicKey, itemMint: PublicKey, holder: PublicKey, tokenMint: PublicKey, slotStates: PublicKey[]): TransactionInstruction {
  const item = itemAddress(itemMint);
  return idlIx('armory', 'revoke_approval', { signer, item, itemHolding: holdingAddr(itemMint, holder), approval: approvalAddress(item, tokenMint), tokenMint }, {}, slotStates.map(ro));
}

/**
 * The access proof suffix an equip path takes for a Gated, Licensed or Leased item (spec 14
 * section 2): `[ARMORY_ID, proof]`, before the agent suffix. Empty for Open and Exclusive.
 */
export function accessProof(mode: number, item: PublicKey, tokenMint: PublicKey): AccountMeta[] {
  const proof = mode === ACCESS.GATED ? approvalAddress(item, tokenMint) : mode === ACCESS.LICENSED ? licenseAddress(item, tokenMint) : mode === ACCESS.LEASED ? leaseOf(item) : null;
  return proof ? [ro(ARMORY_ID), ro(proof)] : [];
}

/** The proof account `enforce_access` reads for `mode` (an Approval or a License). */
export function accessProofAccount(mode: number, item: PublicKey, tokenMint: PublicKey): PublicKey {
  return mode === ACCESS.GATED ? approvalAddress(item, tokenMint) : licenseAddress(item, tokenMint);
}

/**
 * The launchpad refresh tail after an equip on a slot launch's Pool or Relation slot (spec 14
 * section 3.1): `[launch program, launch, pool registry, the launchpad's event authority, item
 * registries...]`. `pool` is the launch's pool.
 */
export function refreshTail(mint: PublicKey, pool: PublicKey, itemRegistries: PublicKey[] = []): AccountMeta[] {
  return [
    ro(LAUNCH_ID), ro(launchAddr(mint)), rw(pda([s('bordrless-hook-accounts'), pool.toBuffer()], LAUNCH_ID)), ro(pda([s('__event_authority')], LAUNCH_ID)),
    ...itemRegistries.map(ro),
  ];
}

/** `submit_template(program, code_hash, uri_hash)`: posts the Hook Lab bond (E-8). */
export function submitTemplate(submitter: PublicKey, program: PublicKey, codeHash: Uint8Array, uriHash: Uint8Array): TransactionInstruction {
  return idlIx('armory', 'submit_template', { submitter, submission: templateSubmissionAddress(program) }, { templateProgram: program, codeHash: Array.from(codeHash), uriHash: Array.from(uriHash) });
}

/** `settle_submission(approved, forfeit)` (admin): refunds or forfeits the bond. */
export function settleSubmission(admin: PublicKey, program: PublicKey, submitter: PublicKey, approved: boolean, forfeit: boolean, template: PublicKey | null): TransactionInstruction {
  return idlIx('armory', 'settle_submission', { admin, submission: templateSubmissionAddress(program), submitter, template }, { approved, forfeit });
}

/**
 * `enforce_access(slot)` (anyone): takes a lapsed Gated or Licensed item out of a slot, back to the
 * launch item (`change`, as `execute` names it), with the refresh tail for a slot launch's Pool or
 * Relation slot.
 */
export function enforceAccess(payer: PublicKey, mint: PublicKey, slot: number, item: PublicKey, mode: number, change: EquipChange, tail: AccountMeta[] = []): TransactionInstruction {
  return idlIx('armory', 'enforce_access', {
    slotState: slotStateAddress(mint, slot), proof: accessProofAccount(mode, item, mint), payer, tokenMint: mint, slotAuthority: slotAuthority(mint),
    equipState: equipStateAddress(mint, slot), ...change,
  }, { slot }, tail);
}
