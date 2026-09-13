#!/usr/bin/env bash
set -euo pipefail

# Scenario: timeout, poison propagation, and garbage collection.
#
# Starts the same r1 -- r2 -- r3 topology and waits for convergence. Then r3
# is killed with SIGKILL, so it cannot run the shutdown handler or advertise
# poisoned routes. r2 must detect the missing updates by timeout, remove the
# route from the kernel routing table, move it to RIP garbage with metric 16,
# and advertise the poison to r1. After the garbage lifetime expires, r2
# removes the poisoned route from its RIP database.
#
# The timeout/garbage timers are shortened only for this integration scenario
# to keep the test fast.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
LOG_DIR="$LOG_ROOT/03_timeout_gc"
trap integration_cleanup EXIT INT TERM

export RIP_TIMEOUT_CHECK_INTERVAL_SECS="${RIP_TIMEOUT_CHECK_INTERVAL_SECS:-1}"
export RIP_ROUTE_TIMEOUT_SECS="${RIP_ROUTE_TIMEOUT_SECS:-5}"
export RIP_GARBAGE_COLLECTION_INTERVAL_SECS="${RIP_GARBAGE_COLLECTION_INTERVAL_SECS:-1}"
export RIP_GARBAGE_COLLECTION_LIFETIME_SECS="${RIP_GARBAGE_COLLECTION_LIFETIME_SECS:-3}"

build_binary
setup_topology
mkdir -p "$LOG_DIR"
RIP_ROUTE_TIMEOUT_SECS=30 start_router r1 tests/integration/03_timeout_gc/r1.yml
RIP_ROUTE_TIMEOUT_SECS=6 start_router r2 tests/integration/03_timeout_gc/r2.yml
RIP_ROUTE_TIMEOUT_SECS=30 start_router r3 tests/integration/03_timeout_gc/r3.yml

wait_for_basic_convergence
stop_router r3 KILL
wait_for_router_exit r3

wait_for_log r2 "route timeout expired, moving to garbage"
wait_for_no_route r2 10.3.3.0/24
wait_for_log r1 "received poisoned route"
wait_for_no_route r1 10.3.3.0/24
wait_for_log r2 "garbage collection removed"
print_routes

echo "03_timeout_gc passed"
