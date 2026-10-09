/**
 * Hookwars addresses (docs/spec/00-overview.md 4.3, 06 4.4). Every seed is the spec's; the fixed
 * ones are pinned in `@hookwars/shared` FIXED_ADDRESSES and derived again in the tests.
 */
import { PublicKey } from '@solana/web3.js';
import { FIXED_ADDRESSES, HOOK_SIGNERS, PROGRAM_IDS } from '@hookwars/shared';

export const TOKEN_ID = new PublicKey(PROGRAM_IDS.token);
export const SWAP_ID = new PublicKey(PROGRAM_IDS.swap);
export const LAUNCH_ID = new PublicKey(PROGRAM_IDS.launch);
export const KIT_ID = new PublicKey(PROGRAM_IDS.kit);
export const BRIDGE_ID = new PublicKey(PROGRAM_IDS.bridge);
export const COMPANION_ID = new PublicKey(PROGRAM_IDS.companion);
export const ARMORY_ID = new PublicKey(PROGRAM_IDS.armory);
export const ITEMS_ID = new PublicKey(PROGRAM_IDS.items);
export const WAR_ID = new PublicKey(PROGRAM_IDS.war);

export const ARMORY_EVENT_AUTHORITY = new PublicKey(FIXED_ADDRESSES.armoryEventAuthority);
export const ARMORY_SIGNER = new PublicKey(FIXED_ADDRESSES.armorySigner);
export const ITEMS_EVENT_AUTHORITY = new PublicKey(FIXED_ADDRESSES.itemsEventAuthority);
export const WAR_EVENT_AUTHORITY = new PublicKey(FIXED_ADDRESSES.warEventAuthority);
export const WAR_CONFIG = new PublicKey(FIXED_ADDRESSES.warConfig);
export const WAR_SIGNER = new PublicKey(FIXED_ADDRESSES.warSigner);
export const LOOT_SIGNER = new PublicKey(FIXED_ADDRESSES.lootSigner);
export const PRIZE_VAULT = new PublicKey(FIXED_ADDRESSES.prizeVault);
export const TOKEN_ITEMS_SIGNER = new PublicKey(HOOK_SIGNERS.tokenForItems);
export const LAUNCH_ITEMS_SIGNER = new PublicKey(HOOK_SIGNERS.launchForItems);

const s = (x: string) => Buffer.from(x);
const u8 = (n: number) => Buffer.from([n]);
const u16 = (n: number) => { const b = Buffer.alloc(2); b.writeUInt16LE(n); return b; };
const u32 = (n: number) => { const b = Buffer.alloc(4); b.writeUInt32LE(n); return b; };
const u64 = (n: bigint | number) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const pda = (seeds: Buffer[], program: PublicKey) => PublicKey.findProgramAddressSync(seeds, program)[0];

// token
export const holdingAddr = (mint: PublicKey, owner: PublicKey) => pda([s('holding'), mint.toBuffer(), owner.toBuffer()], TOKEN_ID);
// armory (02 section 2)
export const armoryConfigAddress = () => pda([s('config')], ARMORY_ID);
export const templateAddress = (templateId: number) => pda([s('template'), u16(templateId)], ARMORY_ID);
export const itemAddress = (itemMint: PublicKey) => pda([s('item'), itemMint.toBuffer()], ARMORY_ID);
export const itemMintAddress = (itemsMinted: bigint | number) => pda([s('item-mint'), u64(itemsMinted)], ARMORY_ID);
export const minterAddress = () => pda([s('minter')], ARMORY_ID);
export const royaltyOwner = (item: PublicKey) => pda([s('royalty'), item.toBuffer()], ARMORY_ID);
export const slotAuthority = (mint: PublicKey) => pda([s('slots'), mint.toBuffer()], ARMORY_ID);
export const slotStateAddress = (mint: PublicKey, slot: number) => pda([s('slot-state'), mint.toBuffer(), u8(slot)], ARMORY_ID);
export const proposalAddress = (mint: PublicKey, slot: number, nonce: bigint | number) => pda([s('proposal'), mint.toBuffer(), u8(slot), u64(nonce)], ARMORY_ID);
export const voteAddress = (proposal: PublicKey, voter: PublicKey) => pda([s('vote'), proposal.toBuffer(), voter.toBuffer()], ARMORY_ID);
export const forgeCounterAddress = (wallet: PublicKey) => pda([s('forges'), wallet.toBuffer()], ARMORY_ID);
// items (04 section 4)
export const equipStateAddress = (mint: PublicKey, slot: number) => pda([s('equip'), mint.toBuffer(), u8(slot)], ITEMS_ID);
export const equipVault = (mint: PublicKey, slot: number) => holdingAddr(mint, equipStateAddress(mint, slot));
export const poolCutsAddress = (mint: PublicKey) => pda([s('pool-cuts'), mint.toBuffer()], ITEMS_ID);
export const raidLedgerAddress = (mint: PublicKey) => pda([s('raid-ledger'), mint.toBuffer()], ITEMS_ID);
export const itemRegistryAddress = (mint: PublicKey, item: PublicKey) => pda([s('bordrless-hook-accounts'), mint.toBuffer(), item.toBuffer()], ITEMS_ID);
// war (05 section 2)
export const warChestAddress = (mint: PublicKey) => pda([s('war-chest'), mint.toBuffer()], WAR_ID);
export const treatyInboxAddress = (mint: PublicKey) => pda([s('treaty-inbox'), mint.toBuffer()], WAR_ID);
export const warStateAddress = (mint: PublicKey) => pda([s('war'), mint.toBuffer()], WAR_ID);
export const warConfigAddress = () => pda([s('war-config')], WAR_ID);
export const seasonAddress = (n: number) => pda([s('season'), u32(n)], WAR_ID);
export const lootTableAddress = (season: number) => pda([s('loot'), u32(season)], WAR_ID);
export const rollAddress = (holding: PublicKey, nonce: bigint | number) => pda([s('roll'), holding.toBuffer(), u64(nonce)], WAR_ID);
/** 05 section 9 (authoritative over 04 2.10's ordering). For the Forge quest `mint` is the default key. */
export const questMarkAddress = (season: number, mint: PublicKey, owner: PublicKey) => pda([s('quest'), u32(season), mint.toBuffer(), owner.toBuffer()], WAR_ID);
export const prizeVaultAddress = () => pda([s('prize-vault')], WAR_ID);
// DEX and launch (03)
// Changed by Hookwars: the observations ring lives in the pool account (M3a), so there is no separate observations address.
export const preparedLaunchAddress = (mint: PublicKey) => pda([s('prepared'), mint.toBuffer()], LAUNCH_ID);
export const armoryCallerAddress = (mint: PublicKey) => pda([s('armory-caller'), mint.toBuffer()], LAUNCH_ID);
export const launchMintAuthority = (mint: PublicKey) => pda([s('launch-mint'), mint.toBuffer()], LAUNCH_ID);
export const launchAddr = (mint: PublicKey) => pda([s('launch'), mint.toBuffer()], LAUNCH_ID);
/** The token program's signer of a slot program's callbacks: `["hook-authority", program]` (01). */
export const tokenHookSigner = (program: PublicKey) => pda([s('hook-authority'), program.toBuffer()], TOKEN_ID);
