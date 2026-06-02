#!/usr/bin/env bash
# build-release.sh — build, size-gate, sign, and checksum `prog`.
#
# The host-native target builds with plain `cargo build` so LuaJIT's DWARF
# unwinder and libgit2 link against the system toolchain. Explicit cross-targets
# build through cargo-zigbuild and fail the release if they fail.
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
SECRET_KEY_INPUT="${PROGIT_MINISIGN_KEY:?set PROGIT_MINISIGN_KEY to the minisign secret key path}"

fail() {
  echo "FATAL: $*" >&2
  exit 1
}

resolve_file_path() {
  local input="$1"
  local dir base
  dir="$(dirname "$input")"
  base="$(basename "$input")"
  [ -d "$dir" ] || fail "signing key directory does not exist: $dir"
  printf '%s/%s\n' "$(cd "$dir" && pwd -P)" "$base"
}

validate_signing_key() {
  local input="$1"
  local resolved mode runner_temp_real

  [ ! -L "$input" ] || fail "PROGIT_MINISIGN_KEY must not be a symlink"
  [ -f "$input" ] || fail "PROGIT_MINISIGN_KEY must point to a regular file"

  resolved="$(resolve_file_path "$input")"
  if [ -n "${RUNNER_TEMP:-}" ]; then
    runner_temp_real="$(cd "$RUNNER_TEMP" && pwd -P)"
    case "$resolved" in
      "$runner_temp_real"/*) ;;
      *) fail "CI signing key must live under RUNNER_TEMP" ;;
    esac
  fi

  mode="$(stat -c %a "$resolved" 2>/dev/null || stat -f %Lp "$resolved" 2>/dev/null || true)"
  if [[ "$mode" =~ ^[0-7]+$ ]] && (( (8#$mode & 077) != 0 )); then
    fail "signing key permissions must not allow group/other access: $mode"
  fi

  printf '%s\n' "$resolved"
}

SECRET_KEY="$(validate_signing_key "$SECRET_KEY_INPUT")"

rm -rf "$DIST"; mkdir -p "$DIST"

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
    rustup target add "$target" >/dev/null 2>&1
    cargo zigbuild --release --target "$target" --bin prog \
      || fail "target $target failed to build"
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
