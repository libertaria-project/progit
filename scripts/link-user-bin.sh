#!/usr/bin/env bash
# link-user-bin.sh - expose a built ProGit binary as ~/bin/prog.
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/link-user-bin.sh [path/to/prog]

Links the given ProGit binary to ~/bin/prog. The default binary is
target/release/prog. Override the destination with PROGIT_USER_BIN.
USAGE
}

fail() {
  echo "FATAL: $*" >&2
  exit 1
}

resolve_path() {
  local input="$1"
  local dir base

  dir="$(dirname "$input")"
  base="$(basename "$input")"
  [ -d "$dir" ] || fail "binary directory does not exist: $dir"

  printf '%s/%s\n' "$(cd "$dir" && pwd -P)" "$base"
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

binary="${1:-target/release/prog}"
dest="${PROGIT_USER_BIN:-$HOME/bin/prog}"

binary_abs="$(resolve_path "$binary")"
[ -f "$binary_abs" ] || fail "binary does not exist: $binary_abs"
[ -x "$binary_abs" ] || fail "binary is not executable: $binary_abs"

dest_dir="$(dirname "$dest")"
mkdir -p "$dest_dir"
dest_abs="$(resolve_path "$dest_dir")/$(basename "$dest")"

if [ -d "$dest_abs" ] && [ ! -L "$dest_abs" ]; then
  fail "destination is a directory, not a link/file: $dest_abs"
fi

ln -sfn "$binary_abs" "$dest_abs"

echo "linked $dest_abs -> $binary_abs"
"$dest_abs" --version

case ":$PATH:" in
  *":$dest_dir:"*) ;;
  *) echo "warning: $dest_dir is not currently on PATH" >&2 ;;
esac
