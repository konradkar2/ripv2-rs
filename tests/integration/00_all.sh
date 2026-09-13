#!/usr/bin/env bash
set -euo pipefail

# Scenario: full integration suite.
#
# Runs every dedicated integration scenario as a separate script. Each scenario
# builds if needed, creates its own network namespace topology, writes logs to
# its own directory under target/integration-logs, and cleans the topology on
# exit. This runner fails fast on the first failing scenario.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
SUITE_LOG_DIR="$ROOT_DIR/target/integration-logs/00_all"

scenarios=(
    01_basic.sh
    02_sigterm_poison.sh
    03_timeout_gc.sh
    04_route_selection.sh
    05_rtnetlink_soft_errors.sh
    06_basic_http.sh
)

mkdir -p "$SUITE_LOG_DIR"
failed_scenarios=()

for scenario in "${scenarios[@]}"; do
    log_path="$SUITE_LOG_DIR/${scenario%.sh}.log"

    echo "Running integration scenario: $scenario"
    if "$SCRIPT_DIR/$scenario" >"$log_path" 2>&1; then
        echo "$scenario: passed"
    else
        echo "$scenario: failed (log: $log_path)"
        failed_scenarios+=("$scenario")
    fi
done

if ((${#failed_scenarios[@]} > 0)); then
    echo "Integration scenarios failed: ${failed_scenarios[*]}"
    exit 1
fi

echo "All integration scenarios passed"
