#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="$ROOT_DIR/target/debug/rip_v2"
LOG_DIR="$ROOT_DIR/target/integration-logs"
RUST_LOG="${RUST_LOG:-info}"
CONVERGENCE_TIMEOUT_SECS="${CONVERGENCE_TIMEOUT_SECS:-45}"

PIDS=()

if ((EUID != 0)); then
    echo "integration_test.sh must be run as root, for example: sudo ./integration_test.sh" >&2
    exit 1
fi

run_as_project_user() {
    if [[ -n "${SUDO_USER:-}" && "$SUDO_USER" != "root" ]]; then
        local user_home
        user_home="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
        sudo -u "$SUDO_USER" HOME="$user_home" bash -lc "$*"
    else
        bash -lc "$*"
    fi
}

cleanup() {
    local status=$?

    trap - EXIT INT TERM

    if ((${#PIDS[@]} > 0)); then
        echo "Stopping RIP routers"
        kill "${PIDS[@]}" 2>/dev/null || true
        sleep 1
        kill -9 "${PIDS[@]}" 2>/dev/null || true
        wait "${PIDS[@]}" 2>/dev/null || true
    fi

    "$ROOT_DIR/cleanup_env.sh" || true
    exit "$status"
}

start_router() {
    local namespace="$1"
    local cfg_path="$2"
    local log_path="$LOG_DIR/$namespace.log"

    echo "Starting $namespace, logs: $log_path"
    RUST_LOG="$RUST_LOG" ip netns exec "$namespace" "$BIN_PATH" "$ROOT_DIR/$cfg_path" \
        >"$log_path" 2>&1 &
    PIDS+=("$!")
}

route_contains() {
    local namespace="$1"
    local prefix="$2"
    local expected="$3"

    ip netns exec "$namespace" ip -4 route show "$prefix" | grep -Fq "$expected"
}

wait_for_route() {
    local namespace="$1"
    local prefix="$2"
    local expected="$3"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Waiting for $namespace route $prefix to contain: $expected"
    while ((SECONDS < deadline)); do
        if route_contains "$namespace" "$prefix" "$expected"; then
            return 0
        fi
        sleep 1
    done

    echo "Route check failed for $namespace $prefix" >&2
    ip netns exec "$namespace" ip -4 route show >&2 || true
    return 1
}

assert_ping() {
    local namespace="$1"
    local source="$2"
    local target="$3"

    echo "Pinging from $namespace ($source -> $target)"
    ip netns exec "$namespace" ping -c 3 -W 1 -I "$source" "$target"
}

trap cleanup EXIT INT TERM

mkdir -p "$LOG_DIR"

echo "Building rip_v2"
run_as_project_user "cd '$ROOT_DIR' && cargo build --manifest-path '$ROOT_DIR/Cargo.toml'"

"$ROOT_DIR/cleanup_env.sh" || true
bash "$ROOT_DIR/setup_env.sh"

start_router r1 cfgs/r1.yml
start_router r2 cfgs/r2.yml
start_router r3 cfgs/r3.yml

wait_for_route r1 10.3.3.0/24 "via 10.0.12.2 dev r1-r2"
wait_for_route r2 10.1.1.0/24 "via 10.0.12.1 dev r2-r1"
wait_for_route r2 10.3.3.0/24 "via 10.0.23.3 dev r2-r3"
wait_for_route r3 10.1.1.0/24 "via 10.0.23.2 dev r3-r2"

echo "Routes after convergence"
ip netns exec r1 ip -4 route show
ip netns exec r2 ip -4 route show
ip netns exec r3 ip -4 route show

assert_ping r1 10.1.1.1 10.3.3.3
assert_ping r3 10.3.3.3 10.1.1.1

echo "Integration test passed"
