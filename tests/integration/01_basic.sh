#!/usr/bin/env bash
set -euo pipefail

# Scenario: basic convergence.
#
# Starts three RIP routers in a line topology:
#
#   lan1 -- r1 -- r2 -- r3 -- lan3
#
# The test waits until RIP learns routes between the edge LANs and verifies
# end-to-end connectivity with pings from r1/lan1 to r3/lan3 and back.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

require_root
LOG_DIR="$LOG_ROOT/01_basic"
trap integration_cleanup EXIT INT TERM

build_binary
setup_topology
start_all_routers

wait_for_basic_convergence
print_routes

assert_ping r1 10.1.1.1 10.3.3.3
assert_ping r3 10.3.3.3 10.1.1.1

echo "01_basic passed"
