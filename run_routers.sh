#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="$ROOT_DIR/target/debug/rip_v2"

PIDS=()

cleanup() {
    local status=$?

    trap - INT TERM EXIT

    if ((${#PIDS[@]} > 0)); then
        echo "Stopping RIP routers..."
        kill "${PIDS[@]}" 2>/dev/null || true
        sleep 1
        kill -9 "${PIDS[@]}" 2>/dev/null || true
        wait "${PIDS[@]}" 2>/dev/null || true
    fi

    exit "$status"
}

start_router() {
    local namespace="$1"
    local cfg_path="$2"

    echo "Starting $namespace with $cfg_path"
    ip netns exec "$namespace" "$BIN_PATH" "$ROOT_DIR/$cfg_path" &
    PIDS+=("$!")
}

trap cleanup INT TERM EXIT

start_router r1 cfgs/r1.yml
start_router r2 cfgs/r2.yml
start_router r3 cfgs/r3.yml

echo "Routers are running. Press Ctrl+C or kill this script to stop them."

set +e
wait -n "${PIDS[@]}"
status=$?
set -e

echo "One of the routers exited, stopping the rest."
exit "$status"
