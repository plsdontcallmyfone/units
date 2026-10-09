# Hookwars (working name)

A fork of Bordrless (github.com/BordrlessDex/bordrless-programs, Apache-2.0), remote `upstream`.
No remote of our own yet: never push. Spot only. No em dashes anywhere. Never invent numbers:
every parameter is named in `docs/spec/00-overview.md` "Parameters" and is "to set" until a test
measures or the owner decides it.

The spec is `docs/spec/`. `00-overview.md` is the contract every other part follows; where a part
and the overview differ, fix the part.

Build and test on the build server `ssh -i ~/.ssh/hookwars_build root@206.189.99.242` (repo at /root/hookwars, synced with rsync excluding target and keys; keys from ~/.config/hookwars/program-keys on the Mac). M0 passed there 2026-10-08: 9 programs built, 180 tests pass, 2 ignored (Studio fixtures).

Toolchain: upstream pins Linux x86_64 (Agave 4.3.0, platform-tools v1.57, Anchor CLI 1.2.0, host
rustc >= 1.97.1). This Mac has the machine-wide Agave 3.1.12 that other sessions use: never replace
`~/.local/share/solana/install/active_release` and never run `agave-install`/`solana-install`.
Program keypairs live outside the repo (`keys/` is git-ignored); back them up before any
`cargo clean` or `rm -rf target`.

## Who is working on what

| Who | Area | Started | Status |
| --- | --- | --- | --- |
| Claude (spec lead, session d79dfc8e) | CLAUDE.md, NOTICE, docs/spec/00-overview.md, docs/spec/07-budgets-tests.md, integration of all spec parts | 2026-10-08 | done 2026-10-08: spec 00 to 07 integrated (R1 to R18) |
| Claude (spec writer: token slots, fork of session d79dfc8e) | docs/spec/01-token-slots.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | done 2026-10-08, released |
| Claude (spec writer: armory, fork of session d79dfc8e) | docs/spec/02-armory.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | done 2026-10-08, released |
| Claude (spec writer: DEX and launch, fork of session d79dfc8e) | docs/spec/03-dex-launch.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | done 2026-10-08, released |
| Claude (spec writer: templates, fork of session d79dfc8e) | docs/spec/04-templates.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | done 2026-10-08, released |
| Claude (spec writer: war, fork of session d79dfc8e) | docs/spec/05-war.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | done 2026-10-08, released |
| Claude (spec writer: app, fork of session d79dfc8e) | docs/spec/06-app.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | done 2026-10-08, released |
| Claude (M1 builder: slots in the token program, fork of session d79dfc8e) | programs/bordrless_token, crates/bordrless-hook, programs/hook_tester (test hooks for slots), programs/tests (new slots.rs, budgets.rs), docs/spec/01-token-slots.md (only to record measured values); build server /root/hookwars | 2026-10-08 | done 2026-10-08: commits 8b63dce, 4c1aaba; 193 passed, 0 failed, 2 ignored (11 slots.rs, 2 budgets.rs new); MAX_SLOTS 4 / MAX_CUTTING_SLOTS 3 proposed, launch-pool paths to measure in M3 |
| Claude (M4+M5 builder: hookwars_war, fork of session d79dfc8e) | branch m4 in ~/hookwars-m4: programs/hookwars_war (new), test-only stubs for absent programs, programs/tests (war.rs, siege.rs, counter_strike.rs, loot.rs, quests.rs, seasons.rs, budgets_war.rs), docs/spec/05-war.md and 07 (measurement lines, implementation notes); build server /root/hw-m4 under /root/build.lock | 2026-10-09 | done 2026-10-09: hookwars_war (800,480-byte .so) + 3 test-only stubs; 250 passed, 0 failed, 2 ignored (53 war suite tests, 4 unit); notes and integration steps at the end of 05 |
