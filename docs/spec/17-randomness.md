# 17. Randomness for loot (D-4)

Status: owner decision 2026-10-10, **Switchboard On-Demand randomness**. Built on branch `vrf`.

This part replaces the "D-4 open" note in 05 section 8.2 and 00 section 7. The adapter interface in
`programs/hookwars_war/src/oracle.rs` stays for the test stub (`randomness_stub`); production names
the Switchboard program itself.

## 1. Which program

`WarConfig.randomness_program` selects the source. It is set in `init_config` and changed only through
`propose_config` then `apply_config` after `admin_timelock_secs` (the existing admin timelock).

| Value | Behaviour |
| --- | --- |
| Switchboard On-Demand, mainnet `SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv` | read directly (section 2) |
| Switchboard On-Demand, devnet `Aio4gaXjXzJNVLtzwtNVmSqGKpANtXhybbkhtAC94ji2` | read directly (section 2) |
| any other program | the adapter interface (05 section 8.2): `roll` CPIs `request_randomness`, `reveal` reads the adapter's `Randomness` account |

The two ids are `ON_DEMAND_MAINNET_PID` and `ON_DEMAND_DEVNET_PID` of the Rust crate
`switchboard-on-demand` 0.13.0 and the npm package `@switchboard-xyz/on-demand` 3.10.6. The war
program does not depend on the crate: it parses the account bytes itself (section 3), so the
toolchain pins (Anchor 1.2.0, platform-tools v1.57, Agave 4.3.0) are untouched.

## 2. Flow

Switchboard randomness is commit then reveal. The trader's transaction carries Switchboard's
instruction first and the war program's second, so the war program sees the state Switchboard just
wrote.

1. **Create** (once per wallet, reusable): Switchboard `randomness_init` for a new account whose key
   the client generates. The authority and payer is the trader (API `rolls/randomness/prepare`, a
   stage with the extra signer `randomness`).
2. **Roll**: `randomness_commit` (authority = the trader) then war `roll(nonce, raid_slot)` (API
   `rolls/prepare`). `roll` spends the ticket and requires:
   - the account is owned by `randomness_program` and has the `RandomnessAccountData` discriminator;
   - `authority` is the roll's owner (`RandomnessAuthority`), so no one else can re-commit the
     account and strand the roll;
   - `seed_slot + 1 == clock.slot`: committed in this transaction (`RandomnessStale`);
   - `reveal_slot < seed_slot`: not revealed for this commit, so its value is unknown
     (`RandomnessStale`).
   The roll records `seed_slot` as `RollRequest.requested_slot` (no layout change).
3. **Reveal** (anyone, a few slots later): Switchboard `randomness_reveal` (the oracle's signed value,
   fetched from the oracle's gateway) then war `reveal` (API `rolls/reveal/prepare`, agent crank
   route). `reveal` requires:
   - the account is the roll's (`oracle_account` address check) and still owned by the program;
   - `seed_slot == RollRequest.requested_slot`: the commit the roll saw (`RandomnessStale` if the
     account was committed again);
   - `reveal_slot == clock.slot` and `reveal_slot > seed_slot`: revealed in this transaction
     (Switchboard's own freshness rule, `get_value(clock.slot)`; `RandomnessNotReady` otherwise).
   The value draws from the season's loot table exactly as before (`draw`), and the armory mints it.
4. **Expiry**: an unrevealed roll can be cancelled by its owner after `roll_expiry_secs`
   (`cancel_roll`); rent comes back, the ticket does not.

The ticket is spent at the roll, before the value exists. A trader who reveals, dislikes the value
and does not send the war `reveal` gains nothing: the value is fixed by the commit, and cancelling
never returns the ticket. This is Switchboard's "collateral on commit" rule.

## 3. Account layout read

`RandomnessAccountData` (`#[repr(C)]`, little endian) after its 8-byte discriminator
`[10, 66, 229, 135, 220, 239, 217, 114]`:

| Offset (with discriminator) | Field | Read |
| --- | --- | --- |
| 8 | `authority: Pubkey` | yes |
| 40 | `queue: Pubkey` | no |
| 72 | `seed_slothash: [u8; 32]` | no |
| 104 | `seed_slot: u64` | yes |
| 112 | `oracle: Pubkey` | no |
| 144 | `reveal_slot: u64` | yes |
| 152 | `value: [u8; 32]` | yes |
| 184 | reserved (224 bytes in the crate struct) | no |

The struct is 408 bytes with the discriminator; live accounts are 480 bytes. Checked on devnet on
2026-10-10: `getProgramAccounts` with the discriminator returned 168,519 accounts, every one 480
bytes; a sample of 40 parsed with these offsets showed `seed_slot < reveal_slot` and a non-zero
value on revealed accounts, and zeros on a fresh one. The parser accepts any length of at least 408.

Code: `hookwars_war::oracle::SbRandomness` (Rust), `hookwars.decodeSbRandomness` and
`hookwars.drawLoot` (SDK; `drawLoot` is the same draw as the program's).

## 4. Trust

The value is produced by a Switchboard oracle inside its TEE from the slot hash of `seed_slot` and
signed; Switchboard's program verifies the signature at reveal. units trusts Switchboard's oracle
set for loot only. Loot affects which item a ticket mints; it never moves a war chest or a fee.

## 5. Tests

`programs/tests/tests/switchboard.rs` (LiteSVM, the account written as a fixture owned by the devnet
program id, standing in for Switchboard's commit and reveal in the same transaction):

| Test | Checks |
| --- | --- |
| `the_switchboard_layout_reads_the_documented_offsets` | an account built field by field from the crate's struct parses to the same values; short or wrong discriminator refused |
| `an_honest_roll_commits_then_reveals_in_the_reveal_slot` | roll records `seed_slot`; reveal before reveal refused; a reveal from an earlier slot refused; reveal in this slot mints the drawn item |
| `a_roll_refuses_a_randomness_account_of_another_owner_or_authority` | wrong owning program, the other cluster's program, another wallet's account |
| `a_roll_refuses_a_stale_or_already_revealed_commit` | commit two slots back, a revealed (reused) account, a seed slot equal to the current slot |
| `a_reveal_refuses_a_recommitted_account` | re-committed after the roll; a different account than the roll's |
| `an_unrevealed_switchboard_roll_expires_without_returning_the_ticket` | `cancel_roll` before and after `roll_expiry_secs` |

The six adapter tests in `loot.rs` still pass on the stub. App: `apps/api/src/loot-prepares.test.ts`
(prepares with a mocked chain and a mocked Switchboard gateway) and `apps/agents/src/loot.test.ts`
(the keeper steps through the runtime's prepared-transaction checks).

## 6. Not done

- **A LiteSVM run against the real Switchboard program.** `randomness_commit` needs the queue, the
  oracle and the program state accounts cloned from a cluster, and `randomness_reveal` needs a
  secp256k1 signature from a live oracle's gateway, which cannot be produced offline. The layout is
  instead checked against live devnet accounts (section 3). The first devnet roll after the config
  change is the end-to-end check.
- **Compute.** The reveal transaction carries Switchboard's signature check, the war `reveal` and the
  armory mint; its compute is to measure on devnet (07 has no line for it yet).
- **Vault-held tickets.** The keeper steps cover tickets the agent key holds. A roll by an agent's
  policy vault would need `roll` through `spend` and is not wired.
