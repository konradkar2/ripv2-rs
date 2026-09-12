#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
LOG_DIR="$LOG_ROOT/manual"
trap integration_cleanup INT TERM EXIT

build_binary
start_all_routers

echo "Routers are running. Press Ctrl+C or kill this script to stop them."
set +e
wait -n "${PIDS[@]}"
status=$?
set -e

echo "One of the routers exited, stopping the rest."
exit "$status"
