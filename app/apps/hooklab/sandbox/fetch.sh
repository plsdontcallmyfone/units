#!/usr/bin/env bash
# Hook Lab fetch sandbox (app pass 5): HOOKLAB_FETCH_SANDBOX="/path/to/sandbox/fetch.sh".
# The git fetch needs the network, so it gets it, with the same read-only view and limits as the
# build otherwise. The service checks the fetched tree against the tarball limits afterwards.
set -euo pipefail
work="$(pwd)"
limits=(prlimit "--cpu=${HOOKLAB_SB_CPU_SECS:-300}" "--as=$((${HOOKLAB_SB_MEM_MB:-2048} * 1024 * 1024))" "--nproc=${HOOKLAB_SB_NPROC:-128}" "--fsize=$((${HOOKLAB_SB_FSIZE_MB:-64} * 1024 * 1024))" --core=0 --)
if command -v bwrap >/dev/null 2>&1; then
  args=(bwrap --unshare-all --share-net --die-with-parent --new-session --proc /proc --dev /dev --tmpfs /tmp)
  for p in /usr /bin /lib /lib64 /etc/alternatives /etc/ssl /etc/ld.so.cache /etc/resolv.conf /etc/hosts /etc/nsswitch.conf; do [ -e "$p" ] && args+=(--ro-bind "$p" "$p"); done
  args+=(--bind "$work" "$work" --chdir "$work")
  exec "${limits[@]}" "${args[@]}" -- "$@"
fi
exec "${limits[@]}" "$@"
