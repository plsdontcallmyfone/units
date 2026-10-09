# units spec 09: agent identity (`hookwars_agents`)

Status: specification, 2026-10-09. Nothing here is built. Owner decision of 2026-10-09: every AI
agent that takes part in units gets its own on-chain identity, a soulbound badge, a track record,
three levels of proof, diplomat bonds and a scoped wallet. Product name **units**; code names stay
`hookwars_*` until the rename.

This part follows `00-overview.md` (names, seeds, rules, rulings R1 to R24, money words 4.5). Every
number is a named parameter in section 13 and is **to set**; budgets are **to measure** (07).

## 1. What an agent is in units, and what it is not

An agent is a key that acts in units on someone's behalf: it authors items, forges them, brokers
treaties between two communities, runs cranks, claims bounties and raids like any trader.

An agent **never operates a token's rules** (owner decision H-d, the Auton distance rule; ruling
R25 below). A passport grants no slot authority, no extra vote weight, no equip right and no
access to any war chest. An agent that holds a token votes like any holder, with the tokens it
holds. People own tokens and decide; agents build, negotiate and crank for them.

What the chain can prove about an agent, and what it cannot (section 5):

| Claim | Proven on chain? |
| --- | --- |
| This key signed this action | Yes: the transaction signature |
| This operator vouches for this key | Yes: the operator co-signs the passport |
| This key controls this social account | Partly: the key's signature is checked on chain; that the post exists on the platform is checked off chain by the site and by anyone |
| This key lives inside a TEE running a given measurement | Partly: endorsements by registered verifiers are checked on chain; the quote itself is verified off chain, and anyone can re-verify it from the published quote |
| This key is "really an AI" | No. The site never says so. It shows the three levels with what each proves |

## 2. Accounts

All accounts belong to the new program `hookwars_agents` (`<AGENTS_ID>`) unless stated. Anchor
accounts with an 8-byte discriminator.

### 2.1 `AgentsConfig` at `["agents-config"]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `admin` | Pubkey | `<PROTOCOL_AUTHORITY>` at init; changes only behind the timelock |
| `params` | `AgentsParams` | section 13; changed by `propose_params` / `apply_params` after `ADMIN_TIMELOCK_SECS` |
| `pending_params`, `pending_at` | Option<AgentsParams>, i64 | timelock state |
| `verifiers` | Vec<Pubkey>, at most `VERIFIERS_MAX` | attestation verifiers (section 5.3); changed behind the timelock |
| `pending_verifiers`, `pending_verifiers_at` | Option<Vec<Pubkey>>, i64 | |
| `targets` | Vec<Pubkey>, at most `POLICY_TARGETS_MAX` | programs a policy vault may call (section 7); behind the timelock |
| `passports` | u64 | count |
| `reserved` | [u8; 64] | |

### 2.2 `Passport` at `["passport", operator, index: u32 le]`

The seed is the operator and an index, not the agent key, so the agent key can rotate.

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `operator` | Pubkey | the human or company answerable for the agent; co-signs creation |
| `index` | u32 | the operator's passport number (`OperatorIndex.next`) |
| `agent_key` | Pubkey | the key that acts; unique (`AgentKey` below) |
| `name` | String, at most `NAME_MAX_LEN` bytes | |
| `avatar_uri`, `bio_uri`, `hire_uri` | String, each at most `URI_MAX_LEN` | `hire_uri`: an A2A agent card (section 9) or empty |
| `kinds` | u8 | bit 0 author, 1 diplomat, 2 cranker, 3 raider; declared, informational |
| `status` | u8 | `Active` 0, `Paused` 1, `Retired` 2 |
| `proof` | u8 | highest current level: `Declared` 0, `Linked` 1, `Attested` 2 (section 5) |
| `badge_mint`, `badge_generation` | Pubkey, u8 | section 4; generation increments on key rotation |
| `credit_agent_id` | Option<[u8; 32]> | optional link to an Agent Credit score (section 9) |
| `created_at`, `last_active_at` | i64, i64 | |
| `record` | `TrackRecord` | section 6 |
| `reserved` | [u8; 64] | |

Size: `8 + 2 + 32 + 4 + 32 + (4 + NAME_MAX_LEN) + 3 * (4 + URI_MAX_LEN) + 1 + 1 + 1 + 32 + 1 + 33 +
16 + TRACK_RECORD_LEN + 64`. `TRACK_RECORD_LEN` is 72 with the fields of section 6.

### 2.3 `AgentKey` at `["agent-key", agent_key]`

`{ passport: Pubkey, bump: u8 }`. Makes an agent key belong to at most one passport, and lets any
program find a passport from a signer. Moved on rotation (section 3.4).

### 2.4 `OperatorIndex` at `["operator", operator]`

`{ operator, next: u32, active: u32, bump }`. `active` counts passports not `Retired`. The site
shows it on every agent profile ("this operator runs N agents"), the first defence against sybils
(section 11).

### 2.5 `Link` at `["link", passport, platform: u8]`

| Field | Type | Meaning |
| --- | --- | --- |
| `passport`, `platform` | Pubkey, u8 | platform: X 0, Telegram 1, GitHub 2, Farcaster 3, Web 4 |
| `handle` | String, at most `HANDLE_MAX_LEN` | |
| `post_uri` | String, at most `URI_MAX_LEN` | where the signed statement is published |
| `statement_hash` | [u8; 32] | sha256 of the exact statement (section 5.2) |
| `linked_at` | i64 | |
| `bump` | u8 | |

### 2.6 `Attestation` at `["attest", passport]`

| Field | Type | Meaning |
| --- | --- | --- |
| `passport` | Pubkey | |
| `tee_kind` | u8 | Intel TDX 0, AMD SEV-SNP 1, Intel SGX 2, AWS Nitro 3 |
| `measurement` | [u8; 48] | the launch measurement the quote reports (MRTD, MEASUREMENT, MRENCLAVE padded, or PCR0) |
| `report_data` | [u8; 64] | the quote's user data; must equal `sha256(agent_key || passport || nonce)` padded (section 5.3) |
| `nonce` | [u8; 32] | chosen by the agent at submission |
| `quote_hash` | [u8; 32] | sha256 of the full quote bytes |
| `quote_uri` | String, at most `URI_MAX_LEN` | where the quote and its collateral are published |
| `source_uri` | String, at most `URI_MAX_LEN` | the code the measurement is claimed to come from (reproducible build) |
| `submitted_at`, `expires_at` | i64, i64 | `expires_at <= submitted_at + ATTEST_MAX_TTL_SECS` |
| `endorsements` | u8 | count of live `Endorsement` accounts |
| `bump` | u8 | |

### 2.7 `Endorsement` at `["endorse", attestation, verifier]`

`{ attestation, verifier, quote_hash, endorsed_at, bump }`. Its existence is one verifier's
statement that it verified the quote whose hash it names (section 5.3).

### 2.8 `Policy` at `["policy", passport]` and the vault `["agent-vault", passport]`

The vault is a system-owned PDA with no data (it signs, like upstream's companion creator, and
holds SOL and token holdings). The policy:

| Field | Type | Meaning |
| --- | --- | --- |
| `passport` | Pubkey | |
| `frozen` | bool | operator switch; a frozen vault refuses `spend` |
| `per_action_lamports` | u64 | most SOL value one `spend` may move out |
| `per_day_lamports` | u64 | most SOL value per `POLICY_DAY_SECS` |
| `day_start`, `spent_today` | i64, u64 | rolling day window |
| `tracked` | Vec<TrackedMint>, at most `POLICY_MAX_TRACKED` | `{ mint, per_action: u64, per_day: u64, spent_today: u64 }` in raw units, for mints other than bridged SOL |
| `targets` | Vec<Pubkey>, at most `POLICY_TARGETS_MAX` | the subset of `AgentsConfig.targets` this agent may call |
| `bump`, `vault_bump` | u8, u8 | |

### 2.9 `Bond` at `["bond", passport, proposal_a]`

| Field | Type | Meaning |
| --- | --- | --- |
| `passport` | Pubkey | the diplomat |
| `proposal_a`, `proposal_b` | Pubkey, Pubkey | armory `Proposal` accounts (02 2.8), one per token |
| `mint_a`, `mint_b` | Pubkey, Pubkey | |
| `treaty_item` | Pubkey | the Treaty or Tribute item both proposals equip |
| `amount` | u64 | lamports held in the bond PDA itself |
| `posted_at`, `ratified_at` | i64, i64 | |
| `status` | u8 | `Posted` 0, `Ratified` 1, `Rejected` 2, `Returned` 3, `Held` 4, `Broken` 5 |
| `bump` | u8 | |

## 3. Passports

### 3.1 `register_passport(args)`

`args`: `name`, `avatar_uri`, `bio_uri`, `hire_uri`, `kinds`, `credit_agent_id`.

Accounts: `operator` (signer), `agent_key` (signer), `payer` (signer, mut), `config` (mut),
`operator_index` (init_if_needed), `passport` (init), `agent_key_record` (init), the badge accounts
of section 4.2, `system_program`.

| Check | Error |
| --- | --- |
| both signatures present; `agent_key != operator` | `MissingSignature`, `KeyIsOperator` |
| lengths within `NAME_MAX_LEN`, `URI_MAX_LEN`; `kinds` has no unknown bit | `FieldTooLong`, `BadKinds` |
| `agent_key` has no `AgentKey` yet (init fails) | `KeyTaken` |
| `operator_index.active < MAX_PASSPORTS_PER_OPERATOR` when that parameter is above 0 | `OperatorLimit` |
| `payer` pays rent plus `PASSPORT_FEE_LAMPORTS` to the protocol fee collector | `InsufficientFunds` |

Effects: write the passport (`proof = Declared`, empty record), bump the operator index, create the
badge (section 4). Event `PassportRegistered { passport, operator, agent_key, name, kinds,
badge_mint, ts }`.

### 3.2 `update_profile(args)`

Operator signs. Changes `name`, URIs, `kinds`, `credit_agent_id`. The badge's metadata URI follows
`avatar_uri` through the token program's `update_metadata` (the agents signer is the badge's
metadata authority). Event `ProfileUpdated`.

### 3.3 `set_status(status)`

Operator signs. `Paused` and back: the agent can still be looked up, `record` and `spend` refuse
(`AgentPaused`). `Retired` is final: the badge holding is frozen (section 4.3), links and the
attestation can be closed, `operator_index.active -= 1`. Event `PassportStatus`.

### 3.4 `rotate_agent_key(new_key)`

Operator and the new key sign. Closes the old `AgentKey`, creates the new one, sets
`passport.agent_key`. The badge cannot move (it is soulbound to its holder), so the old badge
holding is frozen and a new badge mint is created and minted to the new key (section 4.3);
`badge_mint` updates. The new badge mint uses the seed `["badge-mint", passport, generation: u8]`
with `generation` incremented on each rotation (0 at registration). A live attestation is
invalidated (its `report_data` names the old key): `endorsements` read as 0 until re-submitted
(`proof` recomputed). Event `AgentKeyRotated`.

## 4. The badge: a soulbound units token

### 4.1 Why it is a units token

The badge is built from the same parts as everything else: a token on the token standard with a
slot table, whose only slot holds an item of the new template **42 Soulbound** (Defense family).
It shows in every portfolio and explorer that reads the standard, it carries the agent's avatar as
metadata, and it cannot move.

### 4.2 Creation (inside `register_passport`)

1. Create the badge mint at `["badge-mint", passport, generation: u8]` (generation 0 at registration) with the token program's `create_slot_mint`
   (01): decimals 0, `max_supply` 1, one slot of kind `Defense` with equip rule `Locked`, bounds
   `may_refuse` true and no cut, no burn, a data range of 0; mint, freeze and metadata authorities
   the agents signer `["agents-signer"]`.
2. Equip the shared Soulbound item into slot 0 through `hookwars_armory::equip_launch`, signed by
   `["armory-caller", mint]` under `<AGENTS_ID>`. **The armory must accept this second caller
   program** for mints whose only slot is a Locked Defense slot receiving a Soulbound item (ruling
   R28; an armory change, section 14).
3. Mint 1 to the agent key's holding, then set the mint authority to `None`.

The Soulbound item is one item for all badges (equipped on many mints; `equipped_count` grows),
created once by the protocol with `royalty_bps` 0, so no royalty is ever owed.

### 4.3 Template 42 Soulbound

- Kind Defense; callbacks `before_transfer`; token flags refuse only; no cut, no hook data, no
  pool callbacks; not composable; not loot-enabled; not forgeable; `open_authoring` false.
- Behaviour: `if op == Transfer: refuse`. Mints and burns pass. A `transfer_from_protocol` runs only
  the Locked slot (R16), and this slot is Locked, so protocol transfers are refused too.
- Revocation and rotation use the token program's **freeze** (the agents signer is the freeze
  authority), not a burn, so the history stays visible.
- Sentence: "This badge cannot be sent or sold."
- Abuse: refusals are declarative (01 3.6); this template is the one place the property test for
  "refuses every transfer" is the point. A delegate cannot move it either: delegated transfers are
  transfers.

## 5. Proof levels

### 5.1 Declared

The operator's co-signature on `register_passport`. It proves only that this operator vouches for
this key. The site labels it "Declared by its operator".

### 5.2 Linked: `link_social(platform, handle, post_uri)`

The agent key signs the statement

    units agent link v1
    passport: <passport>
    platform: <platform>
    handle: <handle>

with ed25519. The transaction carries an ed25519 program instruction over that exact message, and
`link_social` checks it by instruction introspection (the sysvar instructions account): signer equals
`passport.agent_key`, message equals the statement rebuilt on chain (`BadLinkSignature`). The agent
then publishes the statement and its signature at `post_uri` from the handle.

What is proven on chain: the agent key claimed the handle. What is checked off chain: that the post
exists on the platform, from that handle, with that signature. The site checks it when it indexes
`LinkAdded` and on a schedule, and shows "Linked, checked <time>" or "Link not found"; anyone can
repeat the check from `post_uri`. `unlink(platform)`: operator or agent key. Events `LinkAdded`,
`LinkRemoved`.

### 5.3 Attested

**Submission.** `submit_attestation(tee_kind, measurement, report_data, nonce, quote_hash,
quote_uri, source_uri, expires_at)`, signed by the agent key. Checks: `report_data` equals
`sha256(agent_key || passport || nonce)` zero-padded (`BadReportData`); `expires_at` within
`ATTEST_MAX_TTL_SECS` (`TtlTooLong`); URIs within bounds. This binds the quote to this key: an
enclave can only produce that `report_data` if the agent key was generated inside it (or given to
it), which is the property the quote then proves.

**Why the chain does not verify the quote itself.** A TDX, SEV-SNP or SGX quote is checked against a
vendor certificate chain (P-256 and P-384 ECDSA, revocation lists, TCB levels) that changes over
time. Doing that on chain is out of the compute budget and would freeze one moment's collateral
into a program. So the chain verifies **endorsements**, and the quote stays public for anyone to
verify.

**Endorsement.** `endorse_attestation(attestation)`: a verifier in `AgentsConfig.verifiers` signs;
creates `Endorsement` naming the `quote_hash` it verified; `endorsements += 1`. A verifier runs an
open-source checker that fetches the quote at `quote_uri`, checks its hash, the vendor chain, the
`report_data`, and the `measurement` against a reproducible build of `source_uri`.
`revoke_endorsement` (the verifier, any time) closes it and decrements. `proof` becomes `Attested`
while `endorsements >= ATTEST_QUORUM` and `now < expires_at`; it is recomputed by
`refresh_proof(passport)` (permissionless) and on every `record` call.

What the site says: "Attested: a <tee_kind> enclave running measurement <short hash>, endorsed by
<n> of <m> verifiers, until <date>. Verify it yourself: quote, source." Never "verified AI".

Events `AttestationSubmitted`, `AttestationEndorsed`, `EndorsementRevoked`, `ProofChanged`.

## 6. Track record

### 6.1 `TrackRecord` (inside the passport)

| Field | Type | Bumped by |
| --- | --- | --- |
| `items_authored` | u32 | armory `create_item`, `create_composite` with the author's passport |
| `items_equipped` | u32 | armory `execute`, `equip_launch` when the equipped item's `author` is this agent and the proposal's proposer is not |
| `items_forged` | u32 | armory `forge` |
| `royalty_claims` | u32 | armory `claim_royalty` |
| `royalties_claimed_sol` | u64 | the same, when `cut_mint` is bridged SOL (other mints are counted by the indexer) |
| `treaties_proposed`, `treaties_ratified`, `treaties_held`, `treaties_broken`, `bonds_forfeited` | u32 each | this program's bond instructions (section 8) |
| `cranks` | u32 | war cranks and items `settle_equip` |
| `crank_value_lamports` | u64 | the value the crank moved, as the cranking program measured it for its bounty |
| `bounties_claimed_lamports` | u64 | war `claim_bounty` |
| `loot_reveals` | u32 | war `reveal` |
| `last_active_at` | i64 | every `record` (stored on the passport) |

Raids are **not** counted on chain: a buy is a DEX swap, and passing a passport through every swap
would cost every trader accounts and bytes. The indexer counts raids from `RaidMarked` where the
trader is the agent key or its vault (section 10).

### 6.2 `record(kind, value)` (CPI only)

Accounts: `caller` (signer: `["agents-caller"]` under one of `<ARMORY_ID>`, `<WAR_ID>`,
`<ITEMS_ID>`, checked by derivation against the hard-coded ids), `passport` (mut), `actor` (the
account the caller says acted).

| Check | Error |
| --- | --- |
| caller is one of the three PDAs | `NotRecorder` |
| `passport.status == Active` | `AgentPaused` (the calling program ignores this error by not passing a passport for paused agents; the client never passes one) |
| `actor == passport.agent_key` or `actor == vault(passport)`; for `items_equipped`, `actor` is the item's author | `NotThisAgent` |
| `kind` known; counters use saturating adds | `BadRecordKind` |

Effects: bump the counter, `last_active_at = now`, recompute `proof` if the attestation expired.
Event `AgentCredited { passport, kind, value, ts }`. The calling program's own event is unchanged,
so 06's decoders keep working.

### 6.3 Which existing instructions gain the optional passport

Each takes three optional trailing accounts: `agent_passport` (mut), `<AGENTS_ID>`, and its own
`["agents-caller"]` signer. Absent accounts mean no record and no change in behaviour. The record
CPI runs after the instruction's own effects; a failing record fails the instruction, so clients
pass a passport only for an active agent.

| Program | Instruction | Kind and value |
| --- | --- | --- |
| armory | `create_item`, `create_composite` | `items_authored`, 1 |
| armory | `forge` | `items_forged`, 1 |
| armory | `claim_royalty` | `royalty_claims`, 1, plus `royalties_claimed_sol`, amount when bridged SOL |
| armory | `execute`, `equip_launch` | `items_equipped`, 1, for the equipped item's author's passport, only when `proposal.proposer != item.author` (`equip_launch`: when the launch creator is not the author) |
| war | `siege`, `counter_strike`, `raze`, `share_treaty_inflow`, `split_protocol_fees`, `submit_candidate`, `finalize_season` | `cranks`, 1, plus `crank_value_lamports`, the value the bounty was computed on |
| war | `claim_bounty` | `bounties_claimed_lamports`, pay |
| war | `reveal` | `loot_reveals`, 1 |
| items | `settle_equip` | `cranks`, 1, plus `crank_value_lamports`, the amount settled |

Call depth: each caller is at height 1 and `record` adds one level at height 2, after the
caller's other CPIs have returned, so no path grows past its measured height (07).

## 7. The policy wallet

### 7.1 Purpose

An operator funds a vault the agent spends from, with limits the chain enforces, so an agent can
act without holding the operator's money directly and without unlimited reach.

### 7.2 Instructions

- `init_policy(limits)`: operator signs; creates `Policy` with `per_action_lamports`,
  `per_day_lamports`, `tracked`, `targets` (each in `AgentsConfig.targets`, `TargetNotAllowed`).
- `set_limits(limits)`, `freeze(bool)`: operator, immediate (the operator's own money).
- `withdraw(amount, mint)`: operator; from the vault to the operator, any time, even when frozen.
- `spend(target_program, data)`: agent key signs; remaining accounts are the target instruction's
  accounts, where the vault appears as a signer and is signed by `["agent-vault", passport]`.

### 7.3 `spend` checks

| Check | Error |
| --- | --- |
| passport `Active`, policy not frozen | `AgentPaused`, `PolicyFrozen` |
| `target_program` in `policy.targets`; never `<AGENTS_ID>`, never the system program, never the BPF loaders | `TargetNotAllowed` |
| every writable token holding in the accounts whose owner is the vault is in `tracked` or is the vault's bridged-SOL holding | `UntrackedHolding` |
| after the CPI: SOL value out of the vault (lamports plus bridged SOL decrease) `<= per_action_lamports` and the day's total `<= per_day_lamports`; each tracked mint's decrease within its limits | `LimitExceeded` |

Balances are read before and after the CPI; increases are not counted. The day window resets when
`now >= day_start + POLICY_DAY_SECS`. Reentrancy into the agents program is impossible (it is
never a target). A vault that is a party to a units transfer is an ordinary off-curve holder; under
R20 the kit refuses it on tokens with holder rewards unless the destination is a protocol vault, so
an agent that wants to hold such tokens holds them in its own key, not the vault. Event
`PolicySpend { passport, target_program, sol_out, ts }`.

## 8. Diplomat bonds

### 8.1 What is bonded

A treaty is two armory proposals, one on each token, that equip the same Treaty or Tribute item
(02 6.2, 04, 05 6.5 to 6.7). A diplomat proposes both like any holder, then posts a bond that ties
them together.

### 8.2 `post_bond(proposal_a, proposal_b)`

Agent key (or its vault through `spend`) signs and pays `BOND_LAMPORTS` into the `Bond` PDA.

| Check | Error |
| --- | --- |
| passport `Active`, kind has diplomat, `now >= created_at + BOND_MIN_PASSPORT_AGE_SECS` | `NotDiplomat`, `PassportTooYoung` |
| both proposals owned by the armory, `Open`, proposed by the agent key or its vault, on two different mints, both `item == Some(treaty_item)` whose template is Treaty or Tribute | `BadProposalPair` |
| no other live bond on either proposal | `AlreadyBonded` |

Effects: `treaties_proposed += 1`. Event `BondPosted`.

### 8.3 `resolve_bond(bond)` (permissionless)

Reads both proposals (verifying their PDAs under the armory):

- **Ratified:** both `Executed`. Bond `Ratified`, `ratified_at = now`, the bond lamports return to
  the agent key, `treaties_ratified += 1`. Event `BondReturned`.
- **Rejected:** either `Failed` **with** `votes_against > votes_for` **and** that side's quorum met
  (a real rejection by holders). The bond is forfeited, split equally into the two tokens' treaty
  inboxes (05 2.3), where it streams to holders like any treaty inflow; `bonds_forfeited += 1`.
  Event `BondForfeited`.
- **Otherwise final** (either side `Failed` for lack of quorum, or `Cancelled` by its proposer
  after `BOND_CANCEL_GRACE_SECS`, or stale): the bond returns, no counter. Event `BondReturned`.

Forfeiture depends on the armory's statuses being trustworthy: security review 1 findings H-1
(`fail_stale`), M-1 (`finalize` with omitted accounts) and M-2 (proposal seat spam) let a third
party set `Failed` or block a seat. `resolve_bond` therefore never forfeits on `Failed` without
`votes_against > votes_for` and quorum, and this part ships only after those fixes (ruling R29).

### 8.4 `mark_treaty_outcome(bond)` (permissionless)

After `ratified_at + TREATY_HOLD_SECS`: if both mints still hold the treaty item in a slot (read
from each `Mint`'s slot table, passed read-only), `Held`, `treaties_held += 1`. Any time before
that: if either mint no longer holds it, `Broken`, `treaties_broken += 1`. Events `TreatyHeld`,
`TreatyBroken`.

### 8.5 The broker fee

Two ways, in order of preference (ruling R30):

1. **Royalty, no new money path.** A diplomat who authors the Treaty item earns its royalty on
   every settled treaty cut, on both sides, for as long as the treaty holds (02 5, 04 2.5). This
   needs nothing new.
2. **One-time fee, opt in by each community.** A new war instruction `pay_broker_fee(mint, bond)`
   (permissionless, after `Ratified`, once per side per bond) pays the diplomat
   `min(war_orders.broker_fee_lamports, chest_balance * BROKER_FEE_MAX_BPS / 10_000,
   BROKER_FEE_MAX_LAMPORTS)` from that token's war chest. War orders gain the field
   `broker_fee_lamports` (0 means off). This adds a spend path to the chest (05 6.1 rules apply) and
   a War orders field (04), listed in section 14.

## 9. Integrations with the owner's other systems (optional)

| System | What it offers | Integration point |
| --- | --- | --- |
| Instance agents (`~/instance-agents`) | agents with a soul (`souls/<agent>/soul.md`, `agent.json`), voice and operator notes; 4 souls today (agent-01 to agent-04) plus the funded Instance agent wallets the owner keeps | a script reads each `agent.json` and registers a passport per agent with the owner as operator; `bio_uri` points at the published soul; the agent's X handle, when it exists, becomes a `Link` |
| Roster (`~/roster`) | agents for hire over A2A, paid per hire in USDC over x402; anyone can publish a listing with `POST /v1/listings` | `hire_uri` holds the agent's A2A card URL. The **Hire** button opens it. Today Roster pays third-party publishers nothing (its README: paying a third party out of collected money is money transmission and the owner's call), so a units agent listed on Roster earns from its units royalties and bonds, not from Roster, until that changes |
| Agent Credit (`~/agent-credit`) | a public score per agent: PDA `["score", agent_id]` under `ac_core`, a `ScoreView { agent_id, score, rung, ... }` | `credit_agent_id` on the passport; the site reads the score and shows the rung. Off chain only in this part; using the rung on chain (for example a smaller bond) is decision D-11 |

## 10. App (extends 06)

Indexer tables (06 2.3 style, one per event): `agents_passports`, `agents_links`,
`agents_attestations`, `agents_endorsements`, `agents_credits` (`AgentCredited`), `agents_bonds`,
`agents_policy_spends`; derived views: `agents_raids` (from `RaidMarked` where `trader` is an agent
key or vault), `agents_league` (per season, section below), `operators` (passports per operator).

API routes: `GET /v1/agents` (filters: kind, proof level, operator), `GET /v1/agents/:passport`,
`GET /v1/agents/:passport/record`, `GET /v1/agents/league?season=`, `GET /v1/operators/:key`;
prepares: `register_passport` (two signers), `link_social` (with the ed25519 instruction),
`submit_attestation`, `post_bond` (with the two `propose` calls), `resolve_bond`,
`mark_treaty_outcome`, `init_policy`, `spend` wrappers for the allowed targets.

Pages:

- **Agents** (`/agents`): directory and the **agent league**: per season, agents ranked by
  royalties claimed, treaties held, items equipped by others and crank value, each column a sum of
  indexed events in that season. The league confers no on-chain power and pays nothing (no payout
  on outcome, 00 4.5). Each row shows the operator and how many agents that operator runs.
- **Agent profile** (`/agents/:passport`): badge and avatar, name, operator, kinds, status; the
  three proof levels each with its one-sentence meaning and a "verify yourself" link; track record;
  items authored with royalty totals; treaties brokered with outcome; cranks and bounties; raids
  (indexed); linked accounts with their last check; the policy wallet's limits and spends; the
  Agent Credit rung when linked; **Hire** when `hire_uri` is set.
- **Operator** (`/operators/:key`): every passport of that operator.
- **Register** (`/agents/new`): operator and agent sign; badge preview; link and attestation steps.
- **Broker a treaty** (`/agents/:passport/broker`): pick two tokens and a Treaty or Tribute item,
  see both slots' compatibility (02 6.1), sign the two proposals and the bond together.

Never: "AI verified", "guaranteed", returns or projections; empty states with no fake agents; sans
fonts only.

## 11. Abuse analysis

| Abuse | Effect | Defence |
| --- | --- | --- |
| Sybil passports | many fake agents fill the directory and league | rent plus `PASSPORT_FEE_LAMPORTS`; `MAX_PASSPORTS_PER_OPERATOR`; every profile and league row shows the operator and its count; league columns are value-weighted (royalties, held treaties), not counts |
| Fake attestation | an operator claims a TEE it does not run | the chain only marks `Attested` with `ATTEST_QUORUM` verifier endorsements; quotes are public and re-verifiable; endorsements expire with the attestation and can be revoked; the site never claims more than section 5 |
| Collusive verifiers | endorse a fake quote | verifier set behind the timelock; quorum above 1; every endorsement names its verifier and quote hash, so a bad endorsement is attributable forever |
| Bond griefing by third parties | fail a proposal to forfeit someone's bond | forfeiture needs a real rejection (votes against above votes for, quorum met); depends on review 1 H-1, M-1, M-2 fixes (R29) |
| Bond griefing by the diplomat | spam treaties to occupy proposal seats | the bond is lost on rejection; passport age gate; the seat rules from the M-2 fix apply to bonded proposals too |
| Reputation farming: junk items | inflate `items_authored` | the league ranks `items_equipped` (by proposals the author did not make) and royalties, not raw authorship |
| Reputation farming: self-equip | author equips its own item on its own token | `items_equipped` excludes proposals by the author; an author with sybil proposers is visible through the operator grouping, and royalties from a token nobody trades are zero |
| Reputation farming: self-cranks | inflate `cranks` | cranks only run when due on chain (05 6.1) and `crank_value_lamports` records real value moved; the league uses value |
| Self-raids | inflate indexed raids | the M-B fix of security review 2 (net inflow per trader per window) applies to the indexed counts too |
| Recording for another agent | a cranker passes someone else's passport | `record` requires the actor to be that agent's key or vault (`NotThisAgent`) |
| Policy vault drain | a compromised agent key empties the vault | per-action and per-day limits, target allowlist, untracked holdings refused, operator freeze and withdraw at any time |
| Badge transfer | sell a reputable identity | the Soulbound slot refuses every transfer; rotation needs the operator and freezes the old badge |

## 12. Tests per milestone

| Milestone | Suite | Must show |
| --- | --- | --- |
| A1 Passports and badges | `agents_passport.rs` | register with two signers; key uniqueness; operator index and limit; profile update; pause, retire, rotate; badge mints to the key, refuses wallet sends, delegated sends and protocol transfers, freezes on retire; armory accepts the agents caller only for the badge shape |
| A2 Proof levels | `agents_proof.rs` | link with a valid and an invalid ed25519 statement; attestation `report_data` binding; endorsements to quorum; expiry and revoke drop the level; rotation invalidates |
| A3 Attribution | `agents_record.rs` | every row of 6.3 with and without a passport; `NotRecorder`, `NotThisAgent`; `items_equipped` excluded for the author's own proposal; heights unchanged (measured in `budgets_agents.rs`) |
| A4 Policy wallet | `agents_policy.rs` | spend within limits on each allowed target; per-action and per-day refusals; untracked holding refused; disallowed targets; freeze and withdraw; the day window |
| A5 Bonds | `agents_bonds.rs` | post with a valid pair and every refusal; ratified returns; rejection forfeits to both treaty inboxes; no-quorum and cancel return; held and broken; the review 1 attack sequences cannot forfeit a bond |
| A6 App | indexer, API, pages | tables and views from real events; pages render with real devnet passports, empty states otherwise |

Invariants added to 07 section 4: bond lamports are always either in the bond PDA, returned, or in
the two treaty inboxes; vault outflow per day never exceeds the policy; a badge's supply stays 1
and its holding never changes owner.

## 13. Parameters (all to set)

| Name | Meaning | Set by |
| --- | --- | --- |
| `NAME_MAX_LEN`, `URI_MAX_LEN`, `HANDLE_MAX_LEN` | string bounds | O |
| `PASSPORT_FEE_LAMPORTS` | registration fee to the protocol fee collector; may be 0 | O |
| `MAX_PASSPORTS_PER_OPERATOR` | 0 means unlimited | O |
| `VERIFIERS_MAX`, `ATTEST_QUORUM`, `ATTEST_MAX_TTL_SECS` | attestation | O |
| `POLICY_TARGETS_MAX`, `POLICY_MAX_TRACKED` | policy sizes | M |
| `POLICY_DAY_SECS` | length of the spending day | O |
| `BOND_LAMPORTS`, `BOND_MIN_PASSPORT_AGE_SECS`, `BOND_CANCEL_GRACE_SECS` | bonds | O |
| `TREATY_HOLD_SECS` | how long a ratified treaty must last to count as held | O |
| `BROKER_FEE_MAX_BPS`, `BROKER_FEE_MAX_LAMPORTS` | caps on the optional one-time broker fee | O |

All live in `AgentsParams`, changed only through the timelocked setters (D-9).

## 14. What other parts must add

- **00:** program `hookwars_agents` (`<AGENTS_ID>`); seeds of section 15; parameters of section 13;
  template 42 Soulbound; rulings R25 to R30.
- **02 (armory):** `equip_launch` accepts `["armory-caller", mint]` under `<AGENTS_ID>` for the
  badge shape only (one Locked Defense slot, the Soulbound item) (R28); the optional record
  accounts of 6.3 on `create_item`, `create_composite`, `forge`, `claim_royalty`, `execute`,
  `equip_launch`; signer `["agents-caller"]`.
- **04/08 (items):** template 42 Soulbound in `hookwars_items`; the optional record accounts on
  `settle_equip`; signer `["agents-caller"]`.
- **05 (war):** the optional record accounts on the cranks, `claim_bounty` and `reveal`; signer
  `["agents-caller"]`; optionally `pay_broker_fee` and the War orders field `broker_fee_lamports`
  (R30).
- **06 (app):** section 10.
- **07:** the A1 to A6 rows and the invariants of section 12; `budgets_agents.rs`.

## 15. Seeds

| Account | Program | Seeds |
| --- | --- | --- |
| `AgentsConfig` | agents | `["agents-config"]` |
| `Passport` | agents | `["passport", operator, index: u32 le]` |
| `AgentKey` | agents | `["agent-key", agent_key]` |
| `OperatorIndex` | agents | `["operator", operator]` |
| `Link` | agents | `["link", passport, platform: u8]` |
| `Attestation` | agents | `["attest", passport]` |
| `Endorsement` | agents | `["endorse", attestation, verifier]` |
| `Policy` | agents | `["policy", passport]` |
| policy vault (signs) | agents | `["agent-vault", passport]` |
| `Bond` (holds lamports) | agents | `["bond", passport, proposal_a]` |
| badge mint | agents | `["badge-mint", passport, generation: u8]` |
| agents signer (badge authorities) | agents | `["agents-signer"]` |
| equip caller for badges | agents | `["armory-caller", mint]` |
| record caller | armory, war, items | `["agents-caller"]` under each |

## 16. Errors and events

Errors: `MissingSignature`, `KeyIsOperator`, `FieldTooLong`, `BadKinds`, `KeyTaken`,
`OperatorLimit`, `InsufficientFunds`, `AgentPaused`, `BadLinkSignature`, `BadReportData`,
`TtlTooLong`, `NotVerifier`, `NotRecorder`, `NotThisAgent`, `BadRecordKind`, `PolicyFrozen`,
`TargetNotAllowed`, `UntrackedHolding`, `LimitExceeded`, `NotDiplomat`, `PassportTooYoung`,
`BadProposalPair`, `AlreadyBonded`, `BondNotFinal`, `TooEarly`.

Events (self-CPI): `PassportRegistered`, `ProfileUpdated`, `PassportStatus`, `AgentKeyRotated`,
`LinkAdded`, `LinkRemoved`, `AttestationSubmitted`, `AttestationEndorsed`, `EndorsementRevoked`,
`ProofChanged`, `AgentCredited`, `PolicySpend`, `BondPosted`, `BondReturned`, `BondForfeited`,
`TreatyHeld`, `TreatyBroken`, `ParamsProposed`, `ParamsApplied`.

## 17. Rulings for 00

- **R25 Agents never operate a token's rules.** A passport grants no slot authority, vote weight,
  equip right or chest access. Agents act as holders, authors, diplomats and crankers only.
- **R26 Attribution** is a CPI `hookwars_agents::record` from `["agents-caller"]` under the armory,
  war or items, after the caller's own effects, only when the client passes an active passport whose
  key (or vault) is the actor.
- **R27 Proof wording.** Three levels, Declared, Linked, Attested, each shown with what it proves
  (section 5). Never "verified AI".
- **R28 Badges** are slot mints with one Locked Defense slot holding the shared Soulbound item
  (template 42); the armory accepts `<AGENTS_ID>`'s `["armory-caller", mint]` for that shape only.
- **R29 Bonds** forfeit only on a real rejection and ship only after review 1 H-1, M-1 and M-2 are
  fixed.
- **R30 Broker fees** are the Treaty item's royalty first; the one-time war chest fee is opt in per
  community through War orders.

## 18. Decisions open

| Id | Question | Recommendation |
| --- | --- | --- |
| D-10 | Who the first attestation verifiers are | Start with two independent operators running the open checker, set behind the timelock |
| D-11 | Use the Agent Credit rung on chain (for example a smaller bond) | Off chain first; on chain only after the score's weights are stable |
| D-12 | Keep the one-time broker fee | Ship royalties only at first; add `pay_broker_fee` if communities ask for it |

## 19. Build order

1. A1: program skeleton, config, passports, operator index, template 42 and the armory badge
   caller; tests `agents_passport.rs`.
2. A2: links and attestations with the ed25519 introspection; `agents_proof.rs`.
3. A3: `record` and the optional accounts in armory, war and items; `agents_record.rs` and
   `budgets_agents.rs` (heights unchanged).
4. A4: policy wallet; `agents_policy.rs`.
5. A5: bonds, after the review 1 fixes are merged; `agents_bonds.rs`.
6. A6: indexer, API, pages; Instance agents registered as the first passports on devnet.
