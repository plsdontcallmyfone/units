# Hookwars (working name)

A fork of Bordrless (github.com/BordrlessDex/bordrless-programs, Apache-2.0), remote `upstream`.
No remote of our own yet: never push. Spot only. No em dashes anywhere. Never invent numbers:
every parameter is named in `docs/spec/00-overview.md` "Parameters" and is "to set" until a test
measures or the owner decides it.

The spec is `docs/spec/`. `00-overview.md` is the contract every other part follows; where a part
and the overview differ, fix the part.

Toolchain: upstream pins Linux x86_64 (Agave 4.3.0, platform-tools v1.57, Anchor CLI 1.2.0, host
rustc >= 1.97.1). This Mac has the machine-wide Agave 3.1.12 that other sessions use: never replace
`~/.local/share/solana/install/active_release` and never run `agave-install`/`solana-install`.
Program keypairs live outside the repo (`keys/` is git-ignored); back them up before any
`cargo clean` or `rm -rf target`.

## Who is working on what

| Who | Area | Started | Status |
| --- | --- | --- | --- |
| Claude (spec lead, session d79dfc8e) | CLAUDE.md, NOTICE, docs/spec/00-overview.md, docs/spec/07-budgets-tests.md, integration of all spec parts | 2026-10-08 | active |
| Claude (spec writer: token slots, fork of session d79dfc8e) | docs/spec/01-token-slots.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | active |
| Claude (spec writer: armory, fork of session d79dfc8e) | docs/spec/02-armory.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | active |
| Claude (spec writer: DEX and launch, fork of session d79dfc8e) | docs/spec/03-dex-launch.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | active |
| Claude (spec writer: templates, fork of session d79dfc8e) | docs/spec/04-templates.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | active |
| Claude (spec writer: war, fork of session d79dfc8e) | docs/spec/05-war.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | active |
| Claude (spec writer: app, fork of session d79dfc8e) | docs/spec/06-app.md only; read-only elsewhere; no builds, no commits | 2026-10-08 | active |
