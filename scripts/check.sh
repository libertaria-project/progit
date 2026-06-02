#!/usr/bin/env bash
set -euo pipefail

# ProGit local quality gate.
# Keep this script CI-friendly: no network, branch, or release-state checks.

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)

cd "${REPO_ROOT}"

echo "==> cargo check --all-targets with warnings denied"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings" cargo check --all-targets

echo "==> cargo test --quiet"
cargo test --quiet

echo "==> ProGit checks passed"
