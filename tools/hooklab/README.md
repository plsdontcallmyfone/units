# Hook Lab

The Hook Lab is how an outside author's hook template gets to the armory. A template is its own
Solana program that implements the units slot ABI (`bordrless_hook::TokenSlotArgs` in,
`bordrless_hook::SlotReturn` as return data). The lab runs it against the real token program
and writes a signed report. When the template passes, the report also carries the
`register_template` instruction the armory admin would sign.

Three pieces:

| Piece | Where | What it does |
| --- | --- | --- |
| `hooklab` CLI | `tools/hooklab` | Checks the manifest, builds the crate, runs the property suite and signs the report |
| Starter template | `examples/hook-template` | "Transfer Cut": a working template to copy, plus four deliberately broken builds |
| Service | `app/apps/hooklab` | Takes submissions over HTTP, queues them, runs the CLI on each one in a sandbox and serves the reports |

## Writing a template

Start from `examples/hook-template`:

- `src/lib.rs` holds one Anchor instruction per callback. It is named for the callback
  (`before_transfer`, `after_transfer`, `before_burn`, `after_burn`, `on_touch`), so its
  discriminator is `sha256("global:<name>")[..8]`, the value the token program sends.
- The callback's accounts come in this order: the token program's 5 prefix accounts (hook signer,
  mint, source, destination, authority), then the extras your manifest names, in its order.
- Check that the hook signer is `["hook-authority", your program]` under the token program, and
  that it signed. The starter precomputes this address so no call pays for a PDA search.
- Read your parameters from the armory `Item` account. `params[0]` is at byte 44: discriminator 8,
  version 1, bump 1, item_mint 32, template_id 2. Check the owner (the armory) and the key
  (`args.item`).
- Answer with `set_return_data(SlotReturn)`:
  - at most one delta, `before_transfer` only, and only into the equip vault;
  - data fields of exactly `data_bytes` bytes.
- Write zeros when a holding empties. A holding whose range still holds data cannot close.
- Commit `Cargo.lock`. The build runs with `--locked`.

### `hooklab.json`

```json
{
  "schema": 1,
  "name": "Transfer Cut",
  "program_id": "<your program id>",
  "kind": "fee",
  "callbacks": ["before_transfer"],
  "pool_callbacks": [],
  "max_cut_transfer_bps": 500,
  "may_refuse": false,
  "may_burn": false,
  "data_bytes": 5,
  "answers_touch": false,
  "max_cu_per_call": 15000,
  "extras": ["item", "equip_vault"],
  "params": { "field_count": 1, "names": ["cut_bps"], "field_min": [0], "field_max": [500] },
  "registry": { "open_authoring": true, "loot_enabled": false, "forge_enabled": false,
                "max_level": 1, "loot_royalty_bps": 0, "max_targets": 0 }
}
```

The starter declares 15,000 CU per call. The suite measured at most 14,127, at `cut_bps` 500, on 2026-10-10; at `cut_bps` 0, where nothing is cut, it measured 10,973.

Every bound the lab enforces comes from this file. It invents none of its own; the protocol's
own limits come from `bordrless_hook` and `hookwars_common`.

| Field | Meaning |
| --- | --- |
| `kind` | `fee`, `reward`, `defense` or `relation`. Pool templates are refused by this lab's suite for now (see Gaps); war templates are refused by the armory |
| `max_cut_transfer_bps` | The most a transfer may cut. It becomes the slot's bound, and the lab checks it again |
| `may_refuse` | `false` means any failure of your program counts as a violation |
| `data_bytes` | Your range in each holding. The token program adds 1 epoch byte, and the whole range must fit the holding's 64 bytes |
| `max_cu_per_call` | Your declared compute budget. The lab fails any call that measures above it |
| `extras` | `item`, `equip_vault`. `equip_vault` must appear exactly when the template cuts |

## The CLI

```sh
hooklab manifest examples/hook-template                 # static checks only
hooklab check examples/hook-template \
  --key lab-keypair.json --report report.json \
  [--so prebuilt.so] [--features a,b] [--seed 1] [--random-sets 4] [--ops 48] \
  [--template-id N --admin <armory admin>]
hooklab verify report.json --signer <lab pubkey>
```

Exit codes: 0 means pass (or verified), 1 means fail (or not verified), and 2 means a usage or
I/O error.

`check` runs these steps:

1. **Manifest.** Static checks against the protocol:
   - kinds and callbacks;
   - the cut needs `before_transfer`;
   - the data range fits;
   - touch rules;
   - extras;
   - the parameter schema (at most 11 fields, min ≤ max);
   - the registry fields;
   - the program id is not a protocol program.
2. **Build.** `cargo build-sbf --tools-version v1.57 --arch v3 -- --locked`, the same pins as
   `scripts/solana/programs.sh`. A stack frame over 4,096 bytes fails the build. The report
   records the toolchain line and the features. `HOOKLAB_CARGO_TARGET_DIR` shares one target
   directory across builds.
3. **Property suite.** The suite runs in LiteSVM with the real `bordrless_token`. The armory stub
   signs as the mint's slot authority. For each parameter set, it:
   1. makes a fresh slot mint, one item slot, a fake armory `Item` holding the parameters, the
      equip vault and 4 funded wallets;
   2. measures a baseline transfer with the slot empty, then equips the template;
   3. runs `--ops` seeded random transfers and burns. One in six empties the sender.

   The parameter sets are all minimums, all maximums, then `--random-sets` random sets inside the
   bounds.
4. **Report.** The CLI signs the report and prints it.

### What the suite checks

| Class | Caught by | Meaning |
| --- | --- | --- |
| `over_cut` | the token program (`SlotCutExceeded`), then the lab | The cut went above `max_cut_transfer_bps` |
| `bad_answer` | the token program (`SlotDataLength`, `UnsupportedHookReturn`, `InvalidHookReturn`, `TooManyDeltas`, `ZeroDelta`, `WrongEquipVault`) | The answer broke the ABI |
| `refusal` | the lab | Your program failed while the manifest says `may_refuse: false` |
| `cu_over_declared` | the lab | Compute went above `max_cu_per_call` |
| `cu_blowup` | the runtime | The transaction ran out of compute |
| `conservation` | the lab | Supply ≠ the sum of all holdings, the vault's gain ≠ the cut, or a burn moved supply by the wrong amount |
| `not_closeable` | the token program (`HookDataNotEmpty`) | An emptied holding cannot close |
| `equip`, `no_transfers`, `other` | the lab | Setup failed, nothing went through, or an unclassified error |

The compute per call is measured as the transaction's units minus those of the same transfer with
the slot empty. That figure includes the token program's slot dispatch, so it overstates your
program a little. Declare with that in mind.

### The report

```json
{ "domain": "units:hooklab:report:v1", "body": { ... }, "signer": "<lab pubkey>", "signature": "<base58>" }
```

- **What is signed:** the lab key signs `"units:hooklab:report:v1" || 0x00 || sha256(body)`,
  where the body is canonical JSON (sorted keys, no whitespace). The domain prefix means the
  signature cannot be replayed as any other units statement.
- **Submission:** the body names the submission only by a hash. That is the tarball's sha256, or
  for a local crate a tree hash of its files. The author is never in the report.
- **Body fields:**
  - `verdict`, the manifest and its hash, `so_len` and `build`;
  - `code_hash`: the sha256 of the program with trailing zeros removed. This equals
    `solana-verify get-executable-hash` and is what the armory stores;
  - the suite's per-set results and violations;
  - `register_template`: the accounts, data hex and args. It is present only on a pass, and only
    when `--template-id` and `--admin` are given.
- **Before registering**, the admin checks the two `requires` lines: the program is deployed with
  no upgrade authority, and its executable hash equals `code_hash`.

## The service

`node app/apps/hooklab/src/main.ts`. The default port is 9980; ports 9980 to 9989 are this
service's block.

| Route | |
| --- | --- |
| `POST /v1/submissions` | Body is either a gzip tarball of the crate (`application/gzip`), or JSON `{ "git": "https://github.com/<owner>/<repo>", "commit": "<40 hex>" }` |
| `GET /v1/submissions/:id` | `queued`, `running`, `done` (with `verdict`) or `error` (the run could not finish; this is not a verdict) |
| `GET /v1/submissions/:id/report` | The signed report |
| `GET /v1/lab` | The lab's signer, report domain and limits |
| `GET /healthz` | Health check |

How the service handles submissions:

- **Tarballs** are parsed in process, with no system `tar`:
  - only regular files and directories are written;
  - links, devices and fifos are refused, and so are absolute paths, `..`, backslashes and
    duplicate paths;
  - pax and GNU long names are honored, and still checked;
  - size, file count and path length are limited;
  - the crate must have `Cargo.toml`, `Cargo.lock` and `hooklab.json` at its root, or under a
    single top directory.
- **Git submissions:**
  - only `https://github.com/<owner>/<repo>` URLs are accepted, at a full commit id;
  - the fetch runs with `core.symlinks=false` and only the https protocol;
  - `.git` is removed before the run.
- **Submission id:** the sha256 of the bytes sent, or of `git:<url>@<commit>`. Sending the same
  thing twice returns the same record. The submitter's address is held only in memory, for rate
  limiting; it is never written.
- **Runs:**
  - each run gets a fresh temporary directory, deleted afterwards;
  - `HOOKLAB_TIMEOUT_MS` kills the run at its limit;
  - a restart requeues interrupted runs.
- **Sandbox:** building a crate runs the submitter's code (build scripts, proc macros). Every run
  therefore goes through `HOOKLAB_SANDBOX`, a command prefix such as a bubblewrap or container line
  with no network. With no sandbox set, every run ends in an error. `HOOKLAB_UNSANDBOXED=1` exists
  for local development only.
- **Settings:** every setting is the operator's; none is a protocol parameter. `main.ts` lists
  them.

## Tests

```sh
cargo test -p hooklab                        # unit tests (manifest, report, suite helpers)
SBF_OUT_DIR=<protocol .so dir> cargo test -p hooklab -- --include-ignored   # + the pipeline
cd app/apps/hooklab && pnpm test             # service
```

The pipeline tests build the starter and its four fixture builds. The lab must name each defect:

| Feature | Defect | Expected class |
| --- | --- | --- |
| `overcut` | Cuts 100 bps over the parameter | `over_cut`, only where `cut_bps` is over 400 |
| `refuse` | Fails when the amount is a multiple of 3 | `refusal` |
| `badwrite` | Writes one byte too many | `bad_answer` (`SlotDataLength`) |
| `cuburn` | Runs a 40,000-step loop | `cu_over_declared` or `cu_blowup` |

The pipeline tests also cover these:

- the CLI writes a report that it then verifies;
- the report's instructions (queue, then `register_external_template`) apply against the real armory, and the starter is authored, equipped, cuts a transfer and settles.

## Gaps (integration requests for the armory and items lanes)

Gaps 1 to 4 are closed by protocol pass 4a (docs/spec/14-pass-4a.md section 3): the armory
registers an external template with `register_external_template` behind its admin queue (the
report carries both instructions), authors and equips its items, the items program settles its
equip vault by the same waterfall, and the launchpad records its pool cuts. The pipeline test
`the_armory_registers_and_equips_the_starter_end_to_end` runs the starter through the real armory.
The lab's own suite still refuses `pool_callbacks` (it has no pool model yet); the armory accepts
pool manifests.

5. **`on_touch` is accepted by the manifest but not driven by the suite yet.**
6. **The build is not yet a verifiable build.** Neither `solana-verify` nor Docker is installed on
   the build server. The report records the toolchain line, and the admin compares `code_hash`
   with the deployed program.
7. **The service needs an operator sandbox and a vendored or cached crate registry.** Builds
   inside a no-network sandbox need their dependencies available offline.
