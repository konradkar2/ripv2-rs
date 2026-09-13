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
        "interface_name": "r1-r2",
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
        "interface_name": "r1-r2",
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
        "interface_name": "lan1",
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
        "interface_name": "r1-r2",
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
    if "interface_index" in route:
        print("route leaked interface_index:", json.dumps(route, sort_keys=True), file=sys.stderr)
        sys.exit(1)
    if not isinstance(route.get("interface_name"), str) or not route["interface_name"]:
        print("route has invalid interface_name:", json.dumps(route, sort_keys=True), file=sys.stderr)
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

assert_logs_text() {
    local namespace="r1"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Checking $namespace HTTP logs text"
    while ((SECONDS < deadline)); do
        if ip netns exec "$namespace" python3 - <<'PY'
import sys
import urllib.request

try:
    with urllib.request.urlopen("http://127.0.0.1:8080/api/v1/logs", timeout=2) as response:
        content = response.read().decode("utf-8")
except Exception as error:
    print(f"failed to fetch logs text: {error}", file=sys.stderr)
    sys.exit(1)

expected_fragments = [
    "starting RIP daemon",
    "HTTP server listening",
]

for fragment in expected_fragments:
    if fragment not in content:
        print(f"missing expected log fragment: {fragment}", file=sys.stderr)
        print(content[-4000:], file=sys.stderr)
        sys.exit(1)
PY
        then
            return 0
        fi

        sleep 1
    done

    echo "HTTP logs text check failed for $namespace" >&2
    return 1
}

assert_add_local_route() {
    local namespace="r1"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Adding local route through $namespace HTTP API"
    ip netns exec "$namespace" python3 - <<'PY'
import json
import sys
import urllib.request

payload = {
    "address": "10.44.44.0",
    "prefix": 24,
    "dev": "lan1",
}
request = urllib.request.Request(
    "http://127.0.0.1:8080/api/v1/local-routes",
    data=json.dumps(payload).encode("utf-8"),
    headers={
        "Accept": "application/json",
        "Content-Type": "application/json",
    },
    method="POST",
)

try:
    with urllib.request.urlopen(request, timeout=2) as response:
        body = json.load(response)
except Exception as error:
    print(f"failed to add local route: {error}", file=sys.stderr)
    sys.exit(1)

if body.get("status") != "ok":
    print(f"unexpected add local route response: {body}", file=sys.stderr)
    sys.exit(1)
PY

    echo "Checking added local route in $namespace HTTP routes JSON"
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

expected = {
    "destination": "10.44.44.0",
    "prefix": 24,
    "netmask": "255.255.255.0",
    "next_hop": "0.0.0.0",
    "metric": 1,
    "interface_name": "lan1",
    "route_type": "local",
    "state": "active",
    "in_kernel": False,
}

if not any(all(route.get(key) == value for key, value in expected.items()) for route in routes):
    print("missing added local route:", json.dumps(expected, sort_keys=True), file=sys.stderr)
    print(json.dumps(routes, indent=2, sort_keys=True), file=sys.stderr)
    sys.exit(1)
PY
        then
            return 0
        fi

        sleep 1
    done

    echo "Added local route check failed for $namespace" >&2
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
assert_add_local_route
assert_logs_text

echo "06_basic_http passed"
