#!/usr/bin/env bash
set -euo pipefail

workspace=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
install_root="$workspace/target/imsg-group"
source_root="$workspace/target/imsg-upstream-0.3.1-group"
build_root="$workspace/target/imsg-group-build"
upstream_commit="8e95722146946425831360b7bc1f84ffa42e551a"

if [[ ! -d "$source_root/.git" ]]; then
  git clone --quiet --depth 1 --branch v0.3.1 \
    https://github.com/gnufood/imsg.git "$source_root"
fi
test "$(git -C "$source_root" rev-parse HEAD)" = "$upstream_commit"

cp -a "$workspace/vendor/imsg-store/src/." "$source_root/crates/imsg-store/src/"
cp -a "$workspace/vendor/imsg-store/migrations/." \
  "$source_root/crates/imsg-store/migrations/"
cp -a "$workspace/vendor/imsg-map/src/." "$source_root/crates/imsg-map/src/"
cp -a "$workspace/vendor/imsg-session/src/." "$source_root/crates/imsg-session/src/"
mkdir -p "$source_root/crates/imsg-session/examples"
cp -a "$workspace/vendor/imsg-session/examples/." \
  "$source_root/crates/imsg-session/examples/"

cmp --silent "$workspace/vendor/imsg-store/src/read.rs" \
  "$source_root/crates/imsg-store/src/read.rs"
cmp --silent "$workspace/vendor/imsg-session/src/fetch.rs" \
  "$source_root/crates/imsg-session/src/fetch.rs"
cmp --silent "$workspace/vendor/imsg-map/src/xml.rs" \
  "$source_root/crates/imsg-map/src/xml.rs"

CARGO_TARGET_DIR="$build_root" cargo build \
  --manifest-path "$source_root/Cargo.toml" \
  --package imsg \
  --release \
  --locked

mkdir -p "$install_root/bin"
install -m 755 "$build_root/release/imsg" "$install_root/bin/imsg"

test -x "$install_root/bin/imsg"
echo "PATCHED_IMSG_BUILD=PASS"
