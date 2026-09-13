#!/usr/bin/env bash
set -euo pipefail

# Scenario: basic HTTP routes API.
#
# Starts the same three-router topology as 01_basic:
#
#   lan1 -- r1 -- r2 -- r3 -- lan3
#
# The test waits for RIP convergence and then checks that r1 exposes the
# expected routing database snapshot through GET /api/v1/routes.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

assert_basic_routes_json() {
    local namespace="r1"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Checking $namespace HTTP routes JSON"
    while ((SECONDS < deadline)); do
        if ip netns exec "$namespace" python3 - <<'PY'
import json
import sys
import urllib.request

try:
    with urllib.request.urlopen("http://127.0.0.1:8080/api/v1/routes", timeout=2) as response:
        routes = json.load(response)
except Exception as error:
    print(f"failed to fetch routes JSON: {error}", file=sys.stderr)
    sys.exit(1)

expected_routes = [
    {
        "destination": "10.0.12.0",
        "prefix": 24,
        "netmask": "255.255.255.0",
        "next_hop": "0.0.0.0",
        "metric": 1,
        "route_type": "local",
        "state": "active",
        "in_kernel": False,
    },
    {
        "destination": "10.0.23.0",
        "prefix": 24,
        "netmask": "255.255.255.0",
        "next_hop": "10.0.12.2",
        "metric": 2,
        "route_type": "remote",
        "state": "active",
        "in_kernel": True,
    },
    {
        "destination": "10.1.1.0",
        "prefix": 24,
        "netmask": "255.255.255.0",
        "next_hop": "0.0.0.0",
        "metric": 1,
        "route_type": "local",
        "state": "active",
        "in_kernel": False,
    },
    {
        "destination": "10.3.3.0",
        "prefix": 24,
        "netmask": "255.255.255.0",
        "next_hop": "10.0.12.2",
        "metric": 3,
        "route_type": "remote",
        "state": "active",
        "in_kernel": True,
    },
]

if len(routes) != len(expected_routes):
    print(f"expected {len(expected_routes)} routes, got {len(routes)}", file=sys.stderr)
    print(json.dumps(routes, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)

for expected in expected_routes:
    if not any(all(route.get(key) == value for key, value in expected.items()) for route in routes):
        print("missing expected route:", json.dumps(expected, sort_keys=True), file=sys.stderr)
        print(json.dumps(routes, indent=2, sort_keys=True), file=sys.stderr)
        sys.exit(1)

for route in routes:
    if not isinstance(route.get("interface_index"), int) or route["interface_index"] <= 0:
        print("route has invalid interface_index:", json.dumps(route, sort_keys=True), file=sys.stderr)
        sys.exit(1)
PY
        then
            return 0
        fi

        sleep 1
    done

    echo "HTTP routes JSON check failed for $namespace" >&2
    return 1
}

require_root
LOG_DIR="$LOG_ROOT/06_basic_http"
trap integration_cleanup EXIT INT TERM

build_binary
setup_topology
start_all_routers tests/integration/01_basic

wait_for_basic_convergence
assert_basic_routes_json

echo "06_basic_http passed"
