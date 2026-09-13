#!/usr/bin/env bash
set -euo pipefail

# Scenario: full integration suite.
#
# Runs every dedicated integration scenario as a separate script. Each scenario
# builds if needed, creates its own network namespace topology, writes logs to
# its own directory under target/integration-logs, and cleans the topology on
# exit. This runner fails fast on the first failing scenario.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

scenarios=(
    01_basic.sh
    02_sigterm_poison.sh
    03_timeout_gc.sh
    04_route_selection.sh
    05_rtnetlink_soft_errors.sh
    06_basic_http.sh
)

for scenario in "${scenarios[@]}"; do
    echo "Running integration scenario: $scenario"
    "$SCRIPT_DIR/$scenario"
done

echo "All integration scenarios passed"
