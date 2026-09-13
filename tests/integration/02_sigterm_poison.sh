#!/usr/bin/env bash
set -euo pipefail

# Scenario: graceful shutdown poisoning.
#
# Starts the same r1 -- r2 -- r3 topology and waits for convergence. Then r3
# is stopped with SIGTERM, so its shutdown handler can advertise poisoned
# routes before exiting. r2 should remove the route learned from r3 and
# propagate the poison to r1, causing both routers to drop 10.3.3.0/24.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
LOG_DIR="$LOG_ROOT/02_sigterm_poison"
trap integration_cleanup EXIT INT TERM

build_binary
setup_topology
start_all_routers tests/integration/02_sigterm_poison

wait_for_basic_convergence
stop_router r3 TERM
wait_for_router_exit r3

wait_for_log r3 "shutting down RIP daemon, advertising poisoned routes"
wait_for_log r2 "received poisoned route"
wait_for_log r1 "received poisoned route"

wait_for_no_route r2 10.3.3.0/24
wait_for_no_route r1 10.3.3.0/24
print_routes

echo "02_sigterm_poison passed"
