#!/usr/bin/env bash
set -euo pipefail

# Manual helper: start the basic RIP topology and expose r1's HTTP UI to the
# host browser.
#
# URL:
#   http://172.31.255.2:8080

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/integration/lib.sh
source "$SCRIPT_DIR/lib.sh"

ADMIN_HOST_IF="host-r1-admin"
ADMIN_NS_IF="r1-admin"
ADMIN_HOST_ADDR="172.31.255.1/30"
ADMIN_ROUTER_ADDR="172.31.255.2/30"
ADMIN_URL="http://172.31.255.2:8080"

setup_admin_link() {
    echo "Creating admin link host <-> r1"
    ip link add "$ADMIN_HOST_IF" type veth peer name "$ADMIN_NS_IF"
    ip link set "$ADMIN_NS_IF" netns r1

    ip addr add "$ADMIN_HOST_ADDR" dev "$ADMIN_HOST_IF"
    ip link set "$ADMIN_HOST_IF" up

    ip netns exec r1 ip addr add "$ADMIN_ROUTER_ADDR" dev "$ADMIN_NS_IF"
    ip netns exec r1 ip link set "$ADMIN_NS_IF" up
}

wait_for_http_console() {
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Waiting for HTTP console at $ADMIN_URL"
    while ((SECONDS < deadline)); do
        if python3 - "$ADMIN_URL/api/v1/routes" <<'PY'
import sys
import urllib.request

try:
    with urllib.request.urlopen(sys.argv[1], timeout=2) as response:
        if response.status == 200:
            sys.exit(0)
except Exception:
    pass

sys.exit(1)
PY
        then
            return 0
        fi

        sleep 1
    done

    echo "HTTP console did not become ready at $ADMIN_URL" >&2
    return 1
}

require_root
LOG_DIR="$LOG_ROOT/manual_admin_console"
trap integration_cleanup EXIT INT TERM

build_binary
setup_topology
setup_admin_link

RIP_HTTP_LISTEN_ADDR="0.0.0.0:8080"
export RIP_HTTP_LISTEN_ADDR
start_all_routers tests/integration/01_basic

wait_for_basic_convergence
wait_for_http_console
print_routes

echo "Admin console is available at $ADMIN_URL"

echo "Routers are running. Press Ctrl+C or kill this script to stop them."
set +e
wait -n "${PIDS[@]}"
status=$?
set -e

echo "One of the routers exited, stopping the rest."
exit "$status"
