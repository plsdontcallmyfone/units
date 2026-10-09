# units (formerly the placeholder Hookwars)

**Name: units** (owner, 2026-10-09), lowercase wordmark, text only. "Hookwars" remains in paths, crate names (hookwars_*) and older docs until the global rename, which runs after every parallel branch has merged. Keep the name out of program ids.

A fork of Bordrless (github.com/BordrlessDex/bordrless-programs, Apache-2.0), remote `upstream`.
Remote `origin` = github.com/plsdontcallmyfone/units (private, created 2026-10-09). Only the lead pushes, and only main after a green merge; workers never push. Spot only. No em dashes anywhere. Never invent numbers:
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
| Claude (M2 builder: armory + items armory-facing entry points, fork of session d79dfc8e) | branch m2, worktree ~/hookwars-m2, server /root/hw-m2; heavy builds inside flock /root/build.lock | 2026-10-09 | done, merged into main (218 pass, 2 ignored; main after merge 228 pass) |
| Claude (M3a builder: DEX observations, swap_route, route context, fork of session d79dfc8e) | branch m3a, worktree ~/hookwars-m3a, server /root/hw-m3a | 2026-10-09 | done, merged into main: 215 pass on branch; ring in the pool account; MAX_ROUTE_HOPS 3 proposed |
| Claude (M4/M5 builder: hookwars_war, fork of session d79dfc8e) | branch m4, worktree ~/hookwars-m4, server /root/hw-m4 | 2026-10-09 | done, merged into main: 250 pass on branch (53 war suite + 4 unit), integration steps at the end of 05 |
| Claude (kit + companion builder (R9, war_bps), fork of session d79dfc8e) | branch kitcomp, worktree ~/hookwars-kitcomp, server /root/hw-kitcomp; heavy builds inside flock /root/build.lock | 2026-10-09 | done, merged into main 4b3e859 (203 pass, 2 ignored) |
| Claude (M6 app builder (app/: sdk, indexer, api, web, bots), fork of session d79dfc8e) | branch app, worktree ~/hookwars-app, server /root/hw-app, server ports 9960 to 9969 | 2026-10-09 | done, merged into main: 147 tests, next build passes; see app/INTEGRATION.md |
| Claude (app integration round 1: real IDLs for token, armory, items, war, kit, companion; fork of session d79dfc8e) | branch appint, worktree ~/hookwars-appint, server /root/hw-appint (CARGO_TARGET_DIR /root/cache-appint); app/ and idl/ only (plus the type-equivalent IDL fix in armory and items, Cargo.lock) | 2026-10-09 | done 2026-10-09: commits b752df7..221f42e; Rust 286 pass 2 ignored; app 189 tests pass, typecheck clean, next build passes; remaining in app/INTEGRATION.md section 2 |

Branch workflow (2026-10-09): parallel workers commit on their own branch in their own worktree; only the spec lead merges into main.
| Claude (M3b items builder, fork of session d79dfc8e) | branch m3bi, worktree ~/hookwars-m3bi, server /root/hw-m3bi: programs/hookwars_items, crates/hookwars-common, armory (composites, settle bounty, Performance ring reader, sparse template ids), war integration, token (Pool slots may cut), tests | 2026-10-09 | done: 331 passed, 0 failed, 2 ignored on branch; notes at the end of docs/spec/04 |
