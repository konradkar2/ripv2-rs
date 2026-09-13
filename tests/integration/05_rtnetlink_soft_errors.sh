#!/usr/bin/env bash
set -euo pipefail

# Scenario: rtnetlink soft errors.
#
# Starts the basic r1 -- r2 -- r3 topology, but first manually pre-installs on
# r1 the route that RIP should later learn from r2. That makes the daemon's
# kernel add hit EEXIST, which should be logged and ignored. Then the test
# manually removes the same route from r1 before graceful shutdown, so the
# shutdown delete hits a missing-route netlink error, which should also be
# logged and ignored.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
LOG_DIR="$LOG_ROOT/05_rtnetlink_soft_errors"
trap integration_cleanup EXIT INT TERM

build_binary
setup_topology
mkdir -p "$LOG_DIR"

echo "Manually pre-installing r1 route 10.3.3.0/24 via r2"
ip netns exec r1 ip route add 10.3.3.0/24 \
    via 10.0.12.2 dev r1-r2 proto rip metric 23

start_all_routers tests/integration/05_rtnetlink_soft_errors

wait_for_log r1 "kernel route already exists, ignoring add error"
wait_for_route r1 10.3.3.0/24 "via 10.0.12.2 dev r1-r2"

echo "Manually deleting r1 route 10.3.3.0/24 before daemon shutdown"
ip netns exec r1 ip route del 10.3.3.0/24 \
    via 10.0.12.2 dev r1-r2 proto rip metric 23

stop_router r1 TERM
wait_for_router_exit r1
wait_for_log r1 "kernel route is already absent, ignoring delete error"
print_routes

echo "05_rtnetlink_soft_errors passed"
