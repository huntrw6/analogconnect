#!/usr/bin/env bash
set -euo pipefail

workspace=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$workspace"

git diff --check
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --manifest-path vendor/imsg-store/Cargo.toml
cargo test --manifest-path vendor/imsg-map/Cargo.toml
cargo test --manifest-path vendor/imsg-session/Cargo.toml
./android/build.sh
PYTHONPYCACHEPREFIX=/tmp/analogconnect-pycache python3 -m py_compile scripts/*.py
python3 -c 'import sys, yaml; document = yaml.safe_load(open(sys.argv[1], encoding="utf-8")); assert str(document["openapi"]).startswith("3.")' protocol/openapi-v1.yaml
shellcheck scripts/*.sh
echo "ANALOGCONNECT_VALIDATION=PASS"
