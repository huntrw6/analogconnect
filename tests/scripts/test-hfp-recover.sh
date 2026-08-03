#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)
RECOVERY_SCRIPT="$PROJECT_ROOT/scripts/hfp-recover.sh"
MOCK_DIR="$PROJECT_ROOT/tests/fixtures/hfp-recover/bin"

PASS_COUNT=0
FAIL_COUNT=0

pass() {
    PASS_COUNT=$((PASS_COUNT + 1))
    printf '  PASS  %s\n' "$1"
}

fail() {
    FAIL_COUNT=$((FAIL_COUNT + 1))
    printf '  FAIL  %s\n' "$1" >&2
}

run_case() {
    local name="$1" expected_exit="$2" expected_text="$3"
    shift 3
    set +e
    output=$(env PATH="$MOCK_DIR:$PATH" "$@" "$RECOVERY_SCRIPT" --confirm 2>&1)
    status=$?
    set -e
    if [[ "$status" -eq "$expected_exit" && "$output" == *"$expected_text"* ]] &&
        [[ ! "$output" =~ ([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2} ]]; then
        pass "$name"
    else
        fail "$name (status=$status output=$output)"
    fi
}

run_case success 0 'HFP_RECOVERY=PASS' env
run_case refuses_active_call 1 'reason=call_in_progress' env MOCK_ACTIVE_CALL=1
run_case refuses_idle_transport 1 'reason=transport_not_stuck' env MOCK_TRANSPORT_STATE=idle
run_case reconnect_failure 1 'reason=profile_reconnect_failed' env MOCK_CONNECT_EXIT=1

if [[ "$FAIL_COUNT" -ne 0 ]]; then
    printf 'FAILED: %s passed, %s failed\n' "$PASS_COUNT" "$FAIL_COUNT" >&2
    exit 1
fi
printf 'ALL HFP RECOVERY TESTS PASSED (%s)\n' "$PASS_COUNT"
