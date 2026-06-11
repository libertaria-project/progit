# typed: false
# frozen_string_literal: true

# ProGit — terminal-native project management for git repositories.
#
# This formula installs the prebuilt Linux x86_64 binary from the sovereign
# Forgejo (git.sovereign-society.org). Binary is signed with minisign and
# checksummed — Homebrew verifies the SHA256 automatically.
#
# Usage (from the progit repo checkout):
#   brew install ./contrib/homebrew/Formula/progit.rb
#
# Usage (from any tap):
#   brew tap ProGit/homebrew-progit https://git.sovereign-society.org/ProGit/homebrew-progit.git
#   brew install progit
#
# macOS and ARM Linux users: use the curl installer or `cargo install progit`:
#   curl -fsSL https://git.sovereign-society.org/ProGit/progit/raw/branch/main/scripts/install.sh | sh

class Progit < Formula
  desc "Terminal-native project management for git repositories"
  homepage "https://git.sovereign-society.org/ProGit/progit"
  version "0.8.4-beta"
  license "LicenseRef-LCL-1.0"  # ProGit Core License — not SPDX, but authoritative

  # Only x86_64 Linux is currently shipped as a prebuilt binary.
  # Cross-compile for aarch64/darwin/windows is blocked by LuaJIT+musl
  # _Unwind_* symbol failures. Tracking issue: GAP-CROSS.
  on_linux do
    if Hardware::CPU.intel? && Hardware::CPU.is_64_bit?
      url "https://git.sovereign-society.org/ProGit/progit/releases/download/v0.8.4-beta/prog-0.8.4-beta-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "89c87bbfde7997b8afc1fbd682bac7071f7f6d52b64eff34fa0bd4a8f952568d"
    end
    # ARM Linux: no prebuilt bottle yet — install via curl or crates.io.
  end

  # macOS: no prebuilt bottle yet — use the curl installer or crates.io:
  #   curl -fsSL https://git.sovereign-society.org/ProGit/progit/raw/branch/main/scripts/install.sh | sh

  # Runtime dependencies (Linux only — macOS bundles these)
  depends_on "openssl@3"                          # reqwest HTTPS client
  depends_on "gcc"                                # LuaJIT libgcc_s runtime dep

  # The tarball contains just the `prog` binary (no directory nesting).
  def install
    bin.install "prog"
  end

  test do
    assert_match "prog #{version}", shell_output("#{bin}/prog --version 2>&1")
  end
end
