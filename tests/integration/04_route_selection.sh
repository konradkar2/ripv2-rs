#!/usr/bin/env bash
set -euo pipefail

# Scenario: route selection between two next-hops.
#
# Starts a five-router asymmetric topology:
#
#   r2
#    |
#   r1 --- r3 --- r4 --- r5
#
# Both r2 and r5 advertise 10.99.0.0/24. From r1, the path through r2 is
# shorter than the path through r3-r4-r5. The long branch starts first so r3
# learns 10.99.0.0/24 from r5 before r1 can advertise the shorter path back
# to r3. The test waits until r1 installs the route via r2, then verifies that
# a later advertisement from r3 is logged and ignored as non-better.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
LOG_DIR="$LOG_ROOT/04_route_selection"
trap integration_cleanup EXIT INT TERM

build_binary
setup_topology_from 04_route_selection/setup_env.sh
mkdir -p "$LOG_DIR"

start_router r3 tests/integration/04_route_selection/r3.yml
start_router r4 tests/integration/04_route_selection/r4.yml
start_router r5 tests/integration/04_route_selection/r5.yml

wait_for_route r3 10.99.0.0/24 "via 10.0.34.4 dev r3-r4"

start_router r2 tests/integration/04_route_selection/r2.yml
start_router r1 tests/integration/04_route_selection/r1.yml

wait_for_route r1 10.99.0.0/24 "via 10.0.12.2 dev r1-r2"
wait_for_log r1 "ignoring non-better route from alternate next-hop"
assert_route_not_contains r1 10.99.0.0/24 "via 10.0.13.3 dev r1-r3"
print_routes

echo "04_route_selection passed"
