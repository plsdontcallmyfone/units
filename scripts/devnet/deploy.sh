#!/usr/bin/env bash
# Deploys the units programs to DEVNET, one at a time. DRY RUN BY DEFAULT: prints every command and
# whether each program is already deployed and matching; sends nothing.
#
#   scripts/devnet/deploy.sh                    # dry run, all deployed programs
#   scripts/devnet/deploy.sh --send             # deploy what is missing; verify what is there
#   scripts/devnet/deploy.sh --send --upgrade   # also upgrade programs whose bytes differ
#   scripts/devnet/deploy.sh --send hookwars_war hookwars_market   # only these
#
# Run it on a build server (Agave 4.3 CLI; the .so files from `scripts/solana/programs.sh build`).
# Keys: HOOKWARS_KEYS (default ~/.config/hookwars/program-keys) holds <program>-keypair.json,
# deployer-keypair.json and protocol_authority-keypair.json. Everything is passed explicitly:
# -u devnet URL, -k deployer, --program-id, --upgrade-authority. The global solana config and
# ~/.config/solana are never read or changed.
#
# --max-len is exactly the .so size (no spare space: an upgrade to a bigger build needs
# `solana program extend` first, measured then). Hook programs whose upgrade authority the armory
# and launchpad check (hookwars_items, half_life, tax_hook) are deployed with protocol_authority as
# upgrade authority (HOOK_UPGRADE_AUTHORITIES); every other program with the deployer.
# Resumable: an interrupted upload leaves a buffer; the script closes the deployer's buffers at the
# end (returning their rent). Verification: the on-chain program bytes must hash to the local .so.
set -euo pipefail

SEND=0
UPGRADE=0
SELECTED=()
for a in "$@"; do
  case "$a" in
    --send) SEND=1 ;;
    --upgrade) UPGRADE=1 ;;
    -*) echo "unknown option $a" >&2; exit 2 ;;
    *) SELECTED+=("$a") ;;
  esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
KEYS="${HOOKWARS_KEYS:-$HOME/.config/hookwars/program-keys}"
URL="${SOLANA_DEVNET_RPC:-https://api.devnet.solana.com}"
case "$URL" in *mainnet*) echo "refusing a mainnet URL" >&2; exit 2 ;; esac
AGAVE_BIN="$HOME/.local/share/solana/install/releases/v4.3.0/solana-release/bin"
export PATH="$AGAVE_BIN:$PATH"
DEPLOY_DIR="${SBF_OUT_DIR:-$ROOT/target/deploy}"
DEPLOYER="$KEYS/deployer-keypair.json"
PROTOCOL_AUTHORITY="$KEYS/protocol_authority-keypair.json"

# Deployed programs (scripts/solana/programs.sh PROGRAMS minus TEST_ONLY), largest .so first: the
# peak balance is the kept rent of everything already deployed plus the program being deployed
# plus its temporary buffer, so the biggest buffer is held while the least rent is committed
# (docs/DEVNET.md section 2). Deploying needs no program to exist before another; init order is the
# init plan's.
PROGRAMS=(hookwars_armory hookwars_war hookwars_agents hookwars_items bordrless_launch hookwars_market
  hookwars_social bordrless_swap hookwars_craft hookwars_book bordrless_token bordrless_companion bordrless_kit
  bordrless_bridge half_life tax_hook)
HOOK_PROGRAMS=" hookwars_items half_life tax_hook "
if ((${#SELECTED[@]} > 0)); then PROGRAMS=("${SELECTED[@]}"); fi

run() {
  echo "  \$ $*"
  if ((SEND)); then "$@"; fi
}

for f in "$DEPLOYER" "$PROTOCOL_AUTHORITY"; do [[ -f "$f" ]] || { echo "missing $f" >&2; exit 1; }; done
echo "$( ((SEND)) && echo SEND || echo DRY RUN ) to $URL"
echo "deployer $(solana-keygen pubkey "$DEPLOYER"), balance $(solana balance -u "$URL" -k "$DEPLOYER" 2>/dev/null || echo unknown)"

for program in "${PROGRAMS[@]}"; do
  key="$KEYS/$program-keypair.json"
  so="$DEPLOY_DIR/$program.so"
  [[ -f "$key" ]] || { echo "missing $key" >&2; exit 1; }
  [[ -f "$so" ]] || { echo "missing $so (build first)" >&2; exit 1; }
  id="$(solana-keygen pubkey "$key")"
  size="$(stat -c %s "$so")"
  local_hash="$(sha256sum "$so" | cut -d' ' -f1)"
  if [[ "$HOOK_PROGRAMS" == *" $program "* ]]; then auth="$PROTOCOL_AUTHORITY"; else auth="$DEPLOYER"; fi
  echo "==> $program $id ($size bytes, sha256 $local_hash, upgrade authority $(solana-keygen pubkey "$auth"))"
  if solana program show -u "$URL" "$id" >/dev/null 2>&1; then
    tmp="$(mktemp)"
    solana program dump -u "$URL" "$id" "$tmp" >/dev/null
    chain_hash="$(head -c "$size" "$tmp" | sha256sum | cut -d' ' -f1)"
    chain_size="$(stat -c %s "$tmp")"
    rm -f "$tmp"
    if [[ "$chain_hash" == "$local_hash" && "$chain_size" == "$size" ]]; then
      echo "  deployed and matching; nothing to do"
      continue
    fi
    echo "  deployed but differs (chain $chain_size bytes, sha256 of first $size bytes $chain_hash)"
    if ((UPGRADE)); then
      if ((chain_size < size)); then
        run solana program extend -u "$URL" -k "$DEPLOYER" "$id" "$((size - chain_size))"
      fi
      run solana program deploy -u "$URL" -k "$DEPLOYER" --program-id "$id" --upgrade-authority "$auth" \
        --max-sign-attempts 30 "$so"
    else
      echo "  skipped (pass --upgrade to upgrade)"
    fi
  else
    run solana program deploy -u "$URL" -k "$DEPLOYER" --program-id "$key" --upgrade-authority "$auth" \
      --max-len "$size" --max-sign-attempts 30 "$so"
  fi
  if ((SEND)); then
    tmp="$(mktemp)"
    solana program dump -u "$URL" "$id" "$tmp" >/dev/null
    after="$(head -c "$size" "$tmp" | sha256sum | cut -d' ' -f1)"
    rm -f "$tmp"
    [[ "$after" == "$local_hash" ]] || { echo "  VERIFY FAILED: chain $after != local $local_hash" >&2; exit 1; }
    echo "  verified: on-chain bytes match the local .so"
  fi
done

echo "==> closing the deployer's leftover buffers (returns their rent)"
run solana program close -u "$URL" -k "$DEPLOYER" --buffers
echo "done; balance $(solana balance -u "$URL" -k "$DEPLOYER" 2>/dev/null || echo unknown)"
