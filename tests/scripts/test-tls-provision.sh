#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)
PROVISION_SCRIPT="$PROJECT_ROOT/scripts/tls-provision.sh"
TEST_ROOT=$(mktemp -d)
trap 'rm -rf -- "$TEST_ROOT"' EXIT

PASS_COUNT=0
FAIL_COUNT=0
pass() { PASS_COUNT=$((PASS_COUNT + 1)); printf '  PASS  %s\n' "$1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); printf '  FAIL  %s\n' "$1" >&2; }

output_dir="$TEST_ROOT/generated"
output=$("$PROVISION_SCRIPT" --output "$output_dir" \
    --host 192.0.2.10 --host pi.example.test)
if [[ "$output" == *'TLS_PROVISION=PASS'* ]] &&
    [[ "$output" =~ CERTIFICATE_SHA256=([0-9a-f]{64}) ]]; then
    pass "generates a copyable SHA-256 leaf pin"
else
    fail "generates a copyable SHA-256 leaf pin"
fi

if [[ $(stat -c '%a' "$output_dir/daemon-key.pem") == 600 ]] &&
    [[ $(stat -c '%a' "$output_dir/daemon-cert.pem") == 644 ]]; then
    pass "sets private key and certificate permissions"
else
    fail "sets private key and certificate permissions"
fi

certificate_text=$(openssl x509 -in "$output_dir/daemon-cert.pem" -noout -text)
if [[ "$certificate_text" == *'IP Address:192.0.2.10'* ]] &&
    [[ "$certificate_text" == *'DNS:pi.example.test'* ]]; then
    pass "includes every requested subject alternative name"
else
    fail "includes every requested subject alternative name"
fi

set +e
repeat_output=$("$PROVISION_SCRIPT" --output "$output_dir" --host 192.0.2.10 2>&1)
repeat_status=$?
invalid_output=$("$PROVISION_SCRIPT" --output "$TEST_ROOT/invalid" --host 999.1.2.3 2>&1)
invalid_status=$?
repo_output=$("$PROVISION_SCRIPT" --output "$PROJECT_ROOT/private-tls" --host 192.0.2.10 2>&1)
repo_status=$?
set -e

if [[ $repeat_status -eq 1 && "$repeat_output" == *'reason=output_already_exists'* ]]; then
    pass "refuses to overwrite existing material"
else
    fail "refuses to overwrite existing material"
fi
if [[ $invalid_status -eq 1 && "$invalid_output" == *'reason=invalid_host'* ]]; then
    pass "rejects invalid endpoint hosts"
else
    fail "rejects invalid endpoint hosts"
fi
if [[ $repo_status -eq 1 && "$repo_output" == *'reason=output_inside_repository'* ]]; then
    pass "refuses to place private material in the repository"
else
    fail "refuses to place private material in the repository"
fi

if [[ $FAIL_COUNT -ne 0 ]]; then
    printf 'FAILED: %s passed, %s failed\n' "$PASS_COUNT" "$FAIL_COUNT" >&2
    exit 1
fi
printf 'ALL TLS PROVISION TESTS PASSED (%s)\n' "$PASS_COUNT"
