#!/usr/bin/env bash
set -euo pipefail

if ((EUID != 0)); then
    echo "cleanup_env.sh must be run as root, for example: sudo ./cleanup_env.sh" >&2
    exit 1
fi

NAMESPACES=(r1 r2 r3)
HOST_LINKS=(r1-r2 r2-r1 r2-r3 r3-r2)

for namespace in "${NAMESPACES[@]}"; do
    if ip netns list | awk '{print $1}' | grep -qx "$namespace"; then
        pids="$(ip netns pids "$namespace" || true)"
        if [[ -n "$pids" ]]; then
            echo "Stopping processes in namespace $namespace"
            kill $pids 2>/dev/null || true
            sleep 1
            kill -9 $pids 2>/dev/null || true
        fi

        echo "Deleting namespace $namespace"
        ip netns del "$namespace" 2>/dev/null || true
    fi
done

for link in "${HOST_LINKS[@]}"; do
    if ip link show "$link" >/dev/null 2>&1; then
        echo "Deleting leftover link $link"
        ip link del "$link" 2>/dev/null || true
    fi
done
