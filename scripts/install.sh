#!/usr/bin/env bash
# ProGit — sovereign one-line installer
#
# Usage:
#   curl -fsSL https://git.sovereign-society.org/ProGit/progit/raw/branch/main/scripts/install.sh | sh
#
# Downloads the latest signed ProGit release from the sovereign Forgejo,
# verifies checksums and minisign signature, installs to ~/.local/bin/.
#
# Environment variables:
#   PROGIT_VERSION   — pin a specific version (default: latest)
#   PROGIT_BINDIR    — install directory (default: ~/.local/bin)
#   PROGIT_TARGET    — force a specific Rust target triple
#   PROGIT_VERIFY    — set to "0" to skip signature verification
#   PROGIT_DRY_RUN   — set to "1" to print what would happen without installing

set -euo pipefail

# ─── Config ──────────────────────────────────────────────────────────────────

RELEASE_BASE="https://git.sovereign-society.org/ProGit/progit/releases/download"
RAW_BASE="https://git.sovereign-society.org/ProGit/progit/raw/branch/main"
MINISIGN_PUB_URL="${RAW_BASE}/keys/progit-minisign.pub"

VERSION="${PROGIT_VERSION:-latest}"
BINDIR="${PROGIT_BINDIR:-${HOME}/.local/bin}"
TARGET="${PROGIT_TARGET:-}"
VERIFY="${PROGIT_VERIFY:-1}"
DRY_RUN="${PROGIT_DRY_RUN:-0}"

# ─── Helpers ─────────────────────────────────────────────────────────────────

info()  { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
err()   { printf '\033[1;31m==>\033[0m %s\n' "$*" >&2; exit 1; }
dry()   { [ "$DRY_RUN" = "1" ]; }

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "required command '$1' not found — install it and retry"
  fi
}

cleanup() {
  [ -n "${TMPDIR:-}" ] && [ -d "$TMPDIR" ] && rm -rf "$TMPDIR"
}
trap cleanup EXIT

# ─── Detect target triple ────────────────────────────────────────────────────

detect_target() {
  local arch os

  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64)  arch="x86_64"  ;;
    aarch64|arm64) arch="aarch64"  ;;
    *)             err "unsupported architecture: $arch (contributions welcome)" ;;
  esac

  os="$(uname -s)"
  case "$os" in
    Linux)  os="unknown-linux-gnu"    ;;
    Darwin) os="apple-darwin"         ;;
    *)      err "unsupported OS: $os (contributions welcome)" ;;
  esac

  printf '%s-%s\n' "$arch" "$os"
}

# ─── Resolve latest version ──────────────────────────────────────────────────

resolve_latest() {
  # Fetch the latest release tag from the Forgejo API
  # Falls back to v0.8.4-beta if the API is unreachable (version is hardcoded
  # in the script; update on new releases).
  local latest
  latest="$(curl -fsSL "https://git.sovereign-society.org/api/v1/repos/ProGit/progit/releases/latest" 2>/dev/null \
    | grep -o '"tag_name":"[^"]*"' \
    | head -1 \
    | cut -d'"' -f4 || true)"

  if [ -z "$latest" ]; then
    warn "could not determine latest release from API — defaulting to v0.8.4-beta"
    printf 'v0.8.4-beta\n'
    return
  fi
  printf '%s\n' "$latest"
}

# ─── Download with progress ──────────────────────────────────────────────────

download() {
  local url="$1" out="$2" desc="$3"
  info "downloading ${desc}..."
  if dry; then
    info "[DRY RUN] would download: ${url}"
    return
  fi
  curl -fsSL --retry 3 --retry-delay 2 -o "$out" "$url"
}

# ─── Verify ───────────────────────────────────────────────────────────────────

verify_sha256() {
  local archive="$1" expected="$2"
  if ! command -v sha256sum >/dev/null 2>&1; then
    warn "sha256sum not available — skipping checksum verification"
    return 0
  fi

  local actual
  actual="$(sha256sum "$archive" | cut -d' ' -f1)"
  if [ "$actual" != "$expected" ]; then
    err "checksum mismatch!\n  expected: ${expected}\n  actual:   ${actual}"
  fi
  info "checksum verified ✓"
}

verify_minisign() {
  local archive="$1" sigfile="$2" pubkey="$3"
  if ! command -v minisign >/dev/null 2>&1; then
    warn "minisign not available — skipping PGP-level signature verification"
    return 0
  fi

  if dry; then
    info "[DRY RUN] would verify minisign signature"
    return
  fi

  minisign -V -P "$pubkey" -x "$sigfile" -m "$archive" -q 2>/dev/null || {
    err "minisign signature verification FAILED — binary may be tampered"
  }
  info "minisign signature verified ✓"
}

# ─── Main ─────────────────────────────────────────────────────────────────────

main() {
  info "ProGit installer — https://progit.sovereign-society.org"
  info ""

  # Resolve version and target
  if [ "$VERSION" = "latest" ]; then
    VERSION="$(resolve_latest)"
  fi
  # Strip leading 'v' if present for filename construction
  VER_STR="${VERSION#v}"
  TAG="$VERSION"  # Tag always includes 'v' prefix

  if [ -z "$TARGET" ]; then
    TARGET="$(detect_target)"
  fi
  info "target: ${TARGET}  version: ${TAG}"

  # Construct filenames and URLs
  STEM="prog-${VER_STR}-${TARGET}"
  ARCHIVE="${STEM}.tar.gz"
  SIGFILE="${ARCHIVE}.minisig"
  CHECKSUMS_FILE="SHA256SUMS"
  CHECKSUMS_SIG="SHA256SUMS.minisig"

  ARCHIVE_URL="${RELEASE_BASE}/${TAG}/${ARCHIVE}"
  SIG_URL="${RELEASE_BASE}/${TAG}/${SIGFILE}"
  CHECKSUMS_URL="${RELEASE_BASE}/${TAG}/${CHECKSUMS_FILE}"
  CHECKSUMS_SIG_URL="${RELEASE_BASE}/${TAG}/${CHECKSUMS_SIG}"

  # Temporary directory
  TMPDIR="$(mktemp -d "/tmp/progit-install-XXXXXX")"
  cd "$TMPDIR"

  # Download archive
  download "$ARCHIVE_URL" "$ARCHIVE" "ProGit binary"

  # Download checksum manifest and its signature
  download "$CHECKSUMS_URL" "$CHECKSUMS_FILE" "SHA256 checksums"
  download "$CHECKSUMS_SIG_URL" "$CHECKSUMS_SIG" "checksum signature (minisign)"

  # ─── Verification block ──────────────────────────────────────────────────
  if [ "$VERIFY" = "1" ]; then
    # Extract expected SHA256 for our binary from the manifest
    local expected_hash
    expected_hash="$(grep "${ARCHIVE}" "$CHECKSUMS_FILE" | head -1 | awk '{print $1}')" || true

    if [ -n "$expected_hash" ]; then
      verify_sha256 "$ARCHIVE" "$expected_hash"

      # Also verify the checksum manifest itself (optional — only if we can
      # fetch the public key). We check the manifest's minisig to prove the
      # checksums were signed by Markus's key.
      if command -v minisign >/dev/null 2>&1; then
        local pubkey
        pubkey="$(curl -fsSL "$MINISIGN_PUB_URL" 2>/dev/null | tail -1)" || true
        if [ -n "$pubkey" ] && [ "$(printf '%s' "$pubkey" | wc -c)" -gt 20 ]; then
          verify_minisign "$CHECKSUMS_FILE" "$CHECKSUMS_SIG" "$pubkey"
        else
          warn "could not fetch public key — skipping minisign verification"
        fi
      fi
    else
      warn "binary not found in checksum manifest — skipping verification"
    fi
  fi

  # ─── Extract and install ─────────────────────────────────────────────────
  if dry; then
    info "[DRY RUN] would extract ${ARCHIVE} and install prog to ${BINDIR}/"
    info "[DRY RUN] complete! Run without PROGIT_DRY_RUN=1 to actually install."
    exit 0
  fi

  info "extracting..."
  tar -xzf "$ARCHIVE"
  rm -f "$ARCHIVE"

  # Ensure the binary is executable
  chmod +x prog 2>/dev/null || true

  # Create BINDIR if needed
  mkdir -p "$BINDIR"

  # Check for conflicts and install
  if [ -f "${BINDIR}/prog" ]; then
    local old_hash new_hash
    old_hash="$(sha256sum "${BINDIR}/prog" 2>/dev/null | cut -d' ' -f1)" || true
    new_hash="$(sha256sum "$TMPDIR/prog" | cut -d' ' -f1)"
    if [ "$old_hash" = "$new_hash" ]; then
      info "prog ${TAG} already installed at ${BINDIR}/prog (same binary)"
      exit 0
    fi
    warn "overwriting existing prog at ${BINDIR}/prog"
  fi

  mv prog "${BINDIR}/prog"
  info "installed prog to ${BINDIR}/prog"

  # Check PATH
  local in_path=false
  IFS=':' read -ra PATH_DIRS <<< "$PATH"
  for dir in "${PATH_DIRS[@]}"; do
    if [ "$dir" = "$BINDIR" ]; then
      in_path=true
      break
    fi
  done

  if [ "$in_path" = false ]; then
    warn "${BINDIR} is not in your PATH"
    info "add this to your shell profile:  export PATH=\"\$HOME/.local/bin:\$PATH\""
  fi

  # Verify the binary runs
  info "verifying installation..."
  if "${BINDIR}/prog" --version >/dev/null 2>&1; then
    local installed_version
    installed_version="$("${BINDIR}/prog" --version 2>&1 | head -1)"
    info "ProGit ${VER_STR} installed successfully!"
    info "run 'prog --help' to get started"
  else
    warn "binary installed but version check failed — check ${BINDIR}/prog"
  fi
}

main "$@"
