// Changed by Hookwars: addresses of the agents, market and social programs and the items payouts.
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
export const AGENTS_ID = new PublicKey(PROGRAM_IDS.agents);
export const MARKET_ID = new PublicKey(PROGRAM_IDS.market);
export const SOCIAL_ID = new PublicKey(PROGRAM_IDS.social);
export const CRAFT_ID = new PublicKey(PROGRAM_IDS.craft);
export const BOOK_ID = new PublicKey(PROGRAM_IDS.book);

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

// items payouts (08 arsenal waves D and E: `crates/hookwars-common` `arsenal2::seeds`)
export const referredAddress = (mint: PublicKey, buyer: PublicKey) => pda([s('referred'), mint.toBuffer(), buyer.toBuffer()], ITEMS_ID);
export const referralVaultOwner = (mint: PublicKey) => pda([s('referral'), mint.toBuffer()], ITEMS_ID);
export const loyaltyPotAddress = (mint: PublicKey) => pda([s('loyalty'), mint.toBuffer()], ITEMS_ID);
export const loyaltyClaimAddress = (mint: PublicKey, holder: PublicKey) => pda([s('loyalty-claim'), mint.toBuffer(), holder.toBuffer()], ITEMS_ID);
export const firstBloodAddress = (mint: PublicKey) => pda([s('first-blood'), mint.toBuffer()], ITEMS_ID);
// armory composites (R19)
export const compositeAddress = (item: PublicKey) => pda([s('composite'), item.toBuffer()], ARMORY_ID);
// agents (09: programs/hookwars_agents/src/constants.rs)
export const agentsConfigAddress = () => pda([s('agents-config')], AGENTS_ID);
export const passportAddress = (operator: PublicKey, index: number) => pda([s('passport'), operator.toBuffer(), u32(index)], AGENTS_ID);
export const agentKeyAddress = (key: PublicKey) => pda([s('agent-key'), key.toBuffer()], AGENTS_ID);
export const operatorIndexAddress = (operator: PublicKey) => pda([s('operator'), operator.toBuffer()], AGENTS_ID);
export const linkAddress = (passport: PublicKey, platform: number) => pda([s('link'), passport.toBuffer(), u8(platform)], AGENTS_ID);
export const attestationAddress = (passport: PublicKey) => pda([s('attest'), passport.toBuffer()], AGENTS_ID);
export const endorsementAddress = (attestation: PublicKey, verifier: PublicKey) => pda([s('endorse'), attestation.toBuffer(), verifier.toBuffer()], AGENTS_ID);
export const policyAddress = (passport: PublicKey) => pda([s('policy'), passport.toBuffer()], AGENTS_ID);
export const agentVaultAddress = (passport: PublicKey) => pda([s('agent-vault'), passport.toBuffer()], AGENTS_ID);
export const bondAddress = (passport: PublicKey, proposalA: PublicKey) => pda([s('bond'), passport.toBuffer(), proposalA.toBuffer()], AGENTS_ID);
export const bondMarkAddress = (proposal: PublicKey) => pda([s('bond-mark'), proposal.toBuffer()], AGENTS_ID);
export const agentsSignerAddress = () => pda([s('agents-signer')], AGENTS_ID);
export const agentBadgeMintAddress = (passport: PublicKey, generation: number) => pda([s('badge-mint'), passport.toBuffer(), u8(generation)], AGENTS_ID);
// market (10: programs/hookwars_market/src/state.rs `seeds`)
export const marketConfigAddress = () => pda([s('market-config')], MARKET_ID);
export const listingAddress = (itemMint: PublicKey) => pda([s('listing'), itemMint.toBuffer()], MARKET_ID);
export const marketEscrowAddress = (itemMint: PublicKey) => pda([s('escrow'), itemMint.toBuffer()], MARKET_ID);
export const collectionAddress = (id: number) => pda([s('collection'), u32(id)], MARKET_ID);
export const leaseAddress = (item: PublicKey) => pda([s('lease'), item.toBuffer()], MARKET_ID);
export const leaseEscrowAddress = (itemMint: PublicKey) => pda([s('lease-escrow'), itemMint.toBuffer()], MARKET_ID);
export const commissionAddress = (tokenMint: PublicKey, nonce: bigint | number) => pda([s('commission'), tokenMint.toBuffer(), u64(nonce)], MARKET_ID);
export const commissionVaultAddress = (commission: PublicKey) => pda([s('commission-vault'), commission.toBuffer()], MARKET_ID);
export const submissionAddress = (commission: PublicKey, item: PublicKey) => pda([s('submission'), commission.toBuffer(), item.toBuffer()], MARKET_ID);
// social (10: programs/hookwars_social/src/lib.rs `seeds`)
export const socialConfigAddress = () => pda([s('social-config')], SOCIAL_ID);
export const badgeTypeAddress = (id: number) => pda([s('badge'), u32(id)], SOCIAL_ID);
export const badgeMintAddress = (id: number) => pda([s('badge-mint'), u32(id)], SOCIAL_ID);
export const badgeAwardAddress = (id: number, recipient: PublicKey) => pda([s('award'), u32(id), recipient.toBuffer()], SOCIAL_ID);
export const badgeMinterAddress = () => pda([s('badge-minter')], SOCIAL_ID);
export const guildAddress = (id: number) => pda([s('guild'), u32(id)], SOCIAL_ID);
export const guildTreasuryAddress = (id: number) => pda([s('guild-treasury'), u32(id)], SOCIAL_ID);
export const guildActionAddress = (id: number, nonce: bigint | number) => pda([s('guild-action'), u32(id), u64(nonce)], SOCIAL_ID);
// agents economy (11 section 4: programs/hookwars_agents/src/constants.rs)
export const memoConfigAddress = () => pda([s('memo-config')], AGENTS_ID);
export const directiveAddress = (passport: PublicKey, seq: number) => pda([s('directive'), passport.toBuffer(), u32(seq)], AGENTS_ID);
export const commitmentAddress = (passport: PublicKey, reference: Uint8Array) => pda([s('commit'), passport.toBuffer(), Buffer.from(reference)], AGENTS_ID);
// social economy (11 section 3.3)
export const skillsAddress = () => pda([s('skills')], SOCIAL_ID);
export const profileAddress = (wallet: PublicKey) => pda([s('profile'), wallet.toBuffer()], SOCIAL_ID);
// armory counters and presets (12 section 3 I-5, 13 section 4)
export const authorCounterAddress = (wallet: PublicKey) => pda([s('authored'), wallet.toBuffer()], ARMORY_ID);
export const claimCounterAddress = (wallet: PublicKey) => pda([s('claimed'), wallet.toBuffer()], ARMORY_ID);
export const presetAddress = (id: number) => pda([s('preset'), u16(id)], ARMORY_ID);
// craft (11 section 5: programs/hookwars_craft/src/state.rs `seeds`)
export const craftConfigAddress = () => pda([s('craft-config')], CRAFT_ID);
export const materialAddress = (id: number) => pda([s('material'), u16(id)], CRAFT_ID);
export const materialMintAddress = (id: number) => pda([s('material-mint'), u16(id)], CRAFT_ID);
export const recipeAddress = (id: number) => pda([s('recipe'), u16(id)], CRAFT_ID);
export const wearAddress = (item: PublicKey) => pda([s('wear'), item.toBuffer()], CRAFT_ID);
export const dropRuleAddress = (source: number) => pda([s('drop'), u8(source)], CRAFT_ID);
export const craftMinterAddress = () => pda([s('craft-minter')], CRAFT_ID);
export const craftSignerAddress = () => pda([s('craft-signer')], CRAFT_ID);
// book (11 section 6: programs/hookwars_book/src/state.rs `seeds`)
export const bookConfigAddress = () => pda([s('book-config')], BOOK_ID);
export const bookMarketAddress = (baseMint: PublicKey) => pda([s('book'), baseMint.toBuffer()], BOOK_ID);
export const bookEscrowAddress = (market: PublicKey) => pda([s('book-escrow'), market.toBuffer()], BOOK_ID);
export const classBidAddress = (bidder: PublicKey, nonce: bigint | number) => pda([s('class-bid'), bidder.toBuffer(), u64(nonce)], BOOK_ID);
// the callers' PDAs the economy suffixes name (crates/hookwars-common `agents_record`, `economy`, `market`)
export const agentsCallerAddress = (program: PublicKey) => pda([s('agents-caller')], program);
export const socialCallerAddress = (program: PublicKey) => pda([s('social-caller')], program);
export const craftCallerAddress = (program: PublicKey) => pda([s('craft-caller')], program);
export const marketCallerAddress = () => pda([s('market-caller')], MARKET_ID);
/** An Anchor program's `["__event_authority"]`. */
export const eventAuthorityOf = (program: PublicKey) => pda([s('__event_authority')], program);
