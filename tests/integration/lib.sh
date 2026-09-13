#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN_PATH="$ROOT_DIR/target/debug/rip_v2"
LOG_ROOT="$ROOT_DIR/target/integration-logs"
RUST_LOG="${RUST_LOG:-info}"
CONVERGENCE_TIMEOUT_SECS="${CONVERGENCE_TIMEOUT_SECS:-45}"

PIDS=()
declare -Ag ROUTER_PIDS=()

require_root() {
    if ((EUID != 0)); then
        echo "$(basename "$0") must be run as root, for example: sudo $0" >&2
        exit 1
    fi
}

run_as_project_user() {
    if [[ -n "${SUDO_USER:-}" && "$SUDO_USER" != "root" ]]; then
        local user_home
        user_home="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
        sudo -u "$SUDO_USER" HOME="$user_home" bash -lc "export PATH='$user_home/.cargo/bin':\"\$PATH\"; $*"
    else
        bash -lc "export PATH='${HOME:-}/.cargo/bin':\"\$PATH\"; $*"
    fi
}

build_binary() {
    echo "Building rip_v2"
    run_as_project_user "cd '$ROOT_DIR' && cargo build --manifest-path '$ROOT_DIR/Cargo.toml'"
}

cleanup_topology() {
    "$SCRIPT_DIR/cleanup_env.sh" || true
}

setup_topology() {
    cleanup_topology
    bash "$SCRIPT_DIR/setup_env.sh"
}

setup_topology_from() {
    local setup_script="$1"

    cleanup_topology
    bash "$SCRIPT_DIR/$setup_script"
}

integration_cleanup() {
    local status=$?

    trap - EXIT INT TERM

    if ((${#PIDS[@]} > 0)); then
        echo "Stopping RIP routers"
        kill "${PIDS[@]}" 2>/dev/null || true
        sleep 1
        kill -9 "${PIDS[@]}" 2>/dev/null || true
        wait "${PIDS[@]}" 2>/dev/null || true
    fi

    cleanup_topology
    exit "$status"
}

start_router() {
    local namespace="$1"
    local cfg_path="$2"
    local log_path="$LOG_DIR/$namespace.log"
    local stdio_log_path="$LOG_DIR/$namespace.stdio.log"
    local env_args=("RUST_LOG=$RUST_LOG")
    local name

    for name in \
        RIP_TIMEOUT_CHECK_INTERVAL_SECS \
        RIP_ROUTE_TIMEOUT_SECS \
        RIP_GARBAGE_COLLECTION_INTERVAL_SECS \
        RIP_GARBAGE_COLLECTION_LIFETIME_SECS \
        RIP_HTTP_LISTEN_ADDR; do
        if [[ -n "${!name:-}" ]]; then
            env_args+=("$name=${!name}")
        fi
    done

    echo "Starting $namespace, logs: $log_path"
    env "${env_args[@]}" ip netns exec "$namespace" "$BIN_PATH" "$ROOT_DIR/$cfg_path" "$log_path" \
        >"$stdio_log_path" 2>&1 &
    local pid=$!
    PIDS+=("$pid")
    ROUTER_PIDS["$namespace"]="$pid"
}

start_all_routers() {
    local cfg_dir="${1:-tests/integration/01_basic}"

    mkdir -p "$LOG_DIR"
    start_router r1 "$cfg_dir/r1.yml"
    start_router r2 "$cfg_dir/r2.yml"
    start_router r3 "$cfg_dir/r3.yml"
}

stop_router() {
    local namespace="$1"
    local signal_name="$2"
    local pid="${ROUTER_PIDS[$namespace]}"

    echo "Stopping $namespace with $signal_name"
    kill "-$signal_name" "$pid"
}

wait_for_router_exit() {
    local namespace="$1"
    local pid="${ROUTER_PIDS[$namespace]}"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    while ((SECONDS < deadline)); do
        if ! kill -0 "$pid" 2>/dev/null; then
            wait "$pid" 2>/dev/null || true
            return 0
        fi
        sleep 1
    done

    echo "$namespace did not exit in time" >&2
    return 1
}

route_contains() {
    local namespace="$1"
    local prefix="$2"
    local expected="$3"

    ip netns exec "$namespace" ip -4 route show "$prefix" | grep -Fq "$expected"
}

route_exists() {
    local namespace="$1"
    local prefix="$2"

    [[ -n "$(ip netns exec "$namespace" ip -4 route show "$prefix")" ]]
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

wait_for_no_route() {
    local namespace="$1"
    local prefix="$2"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Waiting for $namespace route $prefix to disappear"
    while ((SECONDS < deadline)); do
        if ! route_exists "$namespace" "$prefix"; then
            return 0
        fi
        sleep 1
    done

    echo "Route still present for $namespace $prefix" >&2
    ip netns exec "$namespace" ip -4 route show "$prefix" >&2 || true
    return 1
}

assert_route_not_contains() {
    local namespace="$1"
    local prefix="$2"
    local unexpected="$3"

    echo "Checking $namespace route $prefix does not contain: $unexpected"
    if route_contains "$namespace" "$prefix" "$unexpected"; then
        echo "Route check failed for $namespace $prefix, unexpected: $unexpected" >&2
        ip netns exec "$namespace" ip -4 route show "$prefix" >&2 || true
        return 1
    fi
}

wait_for_log() {
    local namespace="$1"
    local pattern="$2"
    local log_path="$LOG_DIR/$namespace.log"
    local deadline=$((SECONDS + CONVERGENCE_TIMEOUT_SECS))

    echo "Waiting for $namespace log to contain: $pattern"
    while ((SECONDS < deadline)); do
        if grep -Fq "$pattern" "$log_path"; then
            return 0
        fi
        sleep 1
    done

    echo "Log check failed for $namespace: $pattern" >&2
    sed -n '1,220p' "$log_path" >&2 || true
    return 1
}

assert_ping() {
    local namespace="$1"
    local source="$2"
    local target="$3"

    echo "Pinging from $namespace ($source -> $target)"
    ip netns exec "$namespace" ping -c 3 -W 1 -I "$source" "$target"
}

wait_for_basic_convergence() {
    wait_for_route r1 10.3.3.0/24 "via 10.0.12.2 dev r1-r2"
    wait_for_route r2 10.1.1.0/24 "via 10.0.12.1 dev r2-r1"
    wait_for_route r2 10.3.3.0/24 "via 10.0.23.3 dev r2-r3"
    wait_for_route r3 10.1.1.0/24 "via 10.0.23.2 dev r3-r2"
}

print_routes() {
    echo "Routes after scenario"
    local namespace

    for namespace in $(ip netns list | awk '{print $1}' | sort); do
        echo "[$namespace]"
        ip netns exec "$namespace" ip -4 route show || true
    done
}
