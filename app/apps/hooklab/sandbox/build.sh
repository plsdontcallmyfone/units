#!/usr/bin/env bash
# Hook Lab build sandbox (app pass 5): HOOKLAB_SANDBOX="/path/to/sandbox/build.sh".
# Runs its arguments with no network, a read-only view of the system and the toolchain, the run's
# work directory (the current directory) as the only writable place, and resource limits. Uses
# bubblewrap when installed, else `unshare` (network and user namespaces only, weaker: the file
# system stays visible read-write to the build user, so run the service as a dedicated user).
#
# Settings (all optional):
#   HOOKLAB_SB_RO        colon-separated extra read-only paths (toolchain, rustup, the cargo cache)
#   HOOKLAB_SB_CPU_SECS  CPU seconds per process (default 1800)
#   HOOKLAB_SB_MEM_MB    address space per process in MB (default 6144)
#   HOOKLAB_SB_NPROC     processes for the build user (default 512)
#   HOOKLAB_SB_FSIZE_MB  largest file the build may write in MB (default 512)
set -euo pipefail
work="$(pwd)"
cpu="${HOOKLAB_SB_CPU_SECS:-1800}"; mem="${HOOKLAB_SB_MEM_MB:-6144}"; nproc="${HOOKLAB_SB_NPROC:-512}"; fsize="${HOOKLAB_SB_FSIZE_MB:-512}"
limits=(prlimit "--cpu=$cpu" "--as=$((mem * 1024 * 1024))" "--nproc=$nproc" "--fsize=$((fsize * 1024 * 1024))" --core=0 --)
if command -v bwrap >/dev/null 2>&1; then
  args=(bwrap --unshare-all --die-with-parent --new-session --proc /proc --dev /dev --tmpfs /tmp)
  for p in /usr /bin /lib /lib64 /etc/alternatives /etc/ssl /etc/ld.so.cache; do [ -e "$p" ] && args+=(--ro-bind "$p" "$p"); done
  IFS=':' read -r -a extra <<< "${HOOKLAB_SB_RO:-}"
  for p in "${extra[@]}"; do [ -n "$p" ] && [ -e "$p" ] && args+=(--ro-bind "$p" "$p"); done
  args+=(--bind "$work" "$work" --chdir "$work")
  exec "${limits[@]}" "${args[@]}" -- "$@"
fi
exec "${limits[@]}" unshare --net --map-root-user -- "$@"
