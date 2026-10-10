#!/usr/bin/env bash
# The drill's timed steps (docs/DEVNET.md section 5): each waits for its on-chain clock (guild delay,
# vote period, lease term, season start) and then runs through drill.mjs. Times are unix seconds,
# passed in by the caller from the earlier steps' signatures:
#   drill-timed.sh <guild_ready> <votes_end> <lease_end> <season_start>
# Every outcome lands in drill-log.jsonl; a refused step is logged and the script goes on. A time of
# 0 skips that group (a second run for the season steps only: 0 0 0 <season_start>).
set -uo pipefail
cd "$(dirname "$0")/../.."
set -a; . app/devnet.env; set +a
D="node --experimental-strip-types --no-warnings scripts/devnet/drill.mjs"
until_t() { while [ "$(date +%s)" -lt "$1" ]; do sleep 15; done; }
step() { echo "== $(date -u +%H:%M:%S) $*"; "$@" 2>&1 | grep -v "429\|Retrying" | head -12; }

if [ "$1" != 0 ]; then until_t "$1"
step $D run guilds/execute '{"guildId":0,"nonce":"0"}' --as creator; fi

if [ "$2" != 0 ]; then until_t "$2"
step $D run proposals/finalize '{"mint":"$s:ours","slot":3,"nonce":"0"}' --as trader2
step $D run proposals/finalize '{"mint":"$s:rival","slot":0,"nonce":"0"}' --as trader2; fi

if [ "$3" != 0 ]; then until_t "$3"
step $D run market/lease/end '{"itemMint":"$s:tiersA"}' --as trader1; fi

until_t "$4"
step $D run seasons/open '{}' --as trader2
step $D run quests '{"mint":"$s:ours","season":1,"questId":1,"period":0}' --as trader2
step $D run rolls '{"mint":"$s:ours","nonce":"1","oracleAccount":"$s:ours"}' --as trader2
step $D run prize/split '{}' --as trader2
echo "== done $(date -u +%H:%M:%S)"
