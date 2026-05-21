#!/usr/bin/env bash
# build-release.sh — build, size-gate, sign, and checksum `prog`.
#
# The host-native target builds with plain `cargo build` so LuaJIT's DWARF
# unwinder and libgit2 link against the system toolchain (musl-static + LuaJIT
# via zig fails to resolve _Unwind_* symbols — a known cross-compile reef; see
# follow-up). Other targets cross-compile via cargo-zigbuild and are BEST-EFFORT
# — they must never sink the release. Net-new in Phase 1a.
set -euo pipefail

VERSION="${1:?usage: build-release.sh <version> [target...]}"
shift || true

# The host arch (x86_64 linux) builds natively and is the must-pass artifact.
NATIVE_TARGET="x86_64-unknown-linux-gnu"

TARGETS=("$@")
if [ ${#TARGETS[@]} -eq 0 ]; then
  # FOLLOW-UP: widen this matrix once LuaJIT cross-links under zig. musl-static
  # and aarch64/darwin/windows all currently fail at the LuaJIT _Unwind_* link
  # (lead: RUSTFLAGS="-C link-arg=-lunwind", or a native aarch64 runner). Until
  # then, ship the proven native x86_64-gnu binary rather than burn CI on doomed
  # cross-builds. Pass targets explicitly to override.
  TARGETS=("$NATIVE_TARGET")
fi

DIST="$(pwd)/dist"
MAX_BYTES=$((7 * 1024 * 1024))   # 7MB hard limit (Doctrine 1)
SECRET_KEY="${PROGIT_MINISIGN_KEY:?set PROGIT_MINISIGN_KEY to the minisign secret key path}"

rm -rf "$DIST"; mkdir -p "$DIST"
FAILED_OPTIONAL=()   # best-effort cross targets that failed to build

for target in "${TARGETS[@]}"; do
  echo ">>> building $target"
  # DEFAULT features only — never enable forge-backend (sovereign sidecar tier).
  if [[ "$target" == "$NATIVE_TARGET" ]]; then
    # Native host build — no cross-compile, system toolchain links LuaJIT/libgit2.
    if ! cargo build --release --bin prog; then
      echo "FATAL: native target $target failed to build"; exit 1
    fi
    bindir="target/release"
  else
    rustup target add "$target" >/dev/null 2>&1 || true
    if ! cargo zigbuild --release --target "$target" --bin prog; then
      echo "WARN: best-effort target $target failed — skipping"
      FAILED_OPTIONAL+=("$target"); continue
    fi
    bindir="target/$target/release"
  fi

  ext=""; [[ "$target" == *windows* ]] && ext=".exe"
  bin="$bindir/prog$ext"

  # Size-gate the must-pass native binary.
  if [[ "$target" == "$NATIVE_TARGET" ]]; then
    size=$(stat -c%s "$bin")
    (( size <= MAX_BYTES )) || { echo "FAIL: $target is $size bytes (> 7MB)"; exit 1; }
    echo "    size OK: $size bytes"
  fi

  stem="prog-$VERSION-$target"
  if [[ "$target" == *windows* ]]; then
    archive="$DIST/$stem.zip"
    (cd "$bindir" && zip -q "$archive" "prog$ext")
  else
    archive="$DIST/$stem.tar.gz"
    tar -C "$bindir" -czf "$archive" "prog$ext"
  fi

  minisign -S -s "$SECRET_KEY" \
    -t "ProGit $VERSION $target signed by Markus Maiwald" \
    -m "$archive" >/dev/null
  echo "    signed: ${archive}.minisig"
done

if [ ${#FAILED_OPTIONAL[@]} -gt 0 ]; then
  echo ">>> NOTE: best-effort targets skipped (non-fatal): ${FAILED_OPTIONAL[*]}"
fi

# Checksum the archives ONLY — never the .minisig signatures, which would
# break `sha256sum -c`. nullglob tolerates a target set without a .zip.
(
  cd "$DIST"
  shopt -s nullglob
  sha256sum prog-*.tar.gz prog-*.zip > SHA256SUMS
)
minisign -S -s "$SECRET_KEY" \
  -t "ProGit $VERSION checksums signed by Markus Maiwald" \
  -m "$DIST/SHA256SUMS" >/dev/null
echo ">>> artifacts:"; ls -1 "$DIST"
