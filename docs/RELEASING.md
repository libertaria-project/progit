# Releasing ProGit

How a ProGit release is cut, built, signed, and verified. The pipeline is
tag-driven: you cut a tag, CI does the rest.

## Artifact naming contract

Every release publishes, for each supported target:

```
prog-<version>-<target>.tar.gz        # unix archives (contain `prog`)
prog-<version>-<target>.zip           # windows archive (contains `prog.exe`)
<archive>.minisig                     # per-archive minisign signature
SHA256SUMS                            # sha256 of every archive
SHA256SUMS.minisig                    # signature of the checksum manifest
```

- `<version>` is the git tag with the leading `v` stripped (`v0.8.0-beta` -> `0.8.0-beta`).
- `<target>` is the Rust target triple.

Supported targets:

| Target | Notes |
|--------|-------|
| `x86_64-unknown-linux-musl` | static, the headline single-binary artifact |
| `aarch64-unknown-linux-musl` | static |
| `x86_64-apple-darwin` | best-effort cross via zig; not Apple-notarized |
| `aarch64-apple-darwin` | best-effort cross via zig; not Apple-notarized |
| `x86_64-pc-windows-gnu` | |

The default-feature build is enforced under 7MB on the linux-musl targets
(Doctrine 1). The `forge-backend` feature is never built into release binaries;
sovereign hosting ships as the separate `progit-forged` sidecar daemon.

## Cutting a release

1. Bump the version:
   ```bash
   ./scripts/bump-version.sh <new-version>
   ```
2. Commit the bump, then cut the tag. Interactive:
   ```bash
   ./scripts/release.sh
   ```
   Non-interactive (CI or scripted):
   ```bash
   ./scripts/release.sh --yes
   ```
   This merges `main` into `stable`, creates the annotated tag `v<version>`,
   and pushes branches and tag.
3. The push of a `v*` tag triggers `.forgejo/workflows/release.yml`, which:
   - cross-compiles all targets with `cargo-zigbuild`,
   - size-gates the linux-musl binaries under 7MB,
   - signs each archive with minisign and writes a signed `SHA256SUMS`,
   - publishes a Forgejo Release for the tag.

## Build artifacts manually (local or debugging)

```bash
PROGIT_MINISIGN_KEY=/path/to/release-minisign.key \
  ./scripts/build-release.sh <version> [target...]
```

With no targets given, the full matrix is built. Output lands in `dist/`
(gitignored). Requires `cargo-zigbuild`, `zig`, and `minisign`.

## Verifying a downloaded binary

Download the archive, its `.minisig`, and `SHA256SUMS` from the release, then:

```bash
# 1. Verify the signature against the public key.
minisign -V \
  -p <(curl -fsSL https://git.sovereign-society.org/ProGit/progit/raw/branch/main/keys/progit-minisign.pub) \
  -m prog-<version>-<target>.tar.gz

# 2. Verify the checksum. --ignore-missing: SHA256SUMS lists every target,
#    you only downloaded one archive.
sha256sum --ignore-missing -c SHA256SUMS
```

Both must succeed before trusting a binary. The expected signature output is
`Signature and comment signature verified`; the checksum line must report `OK`.

Each signature also carries an authenticated trusted comment, displayed by
`minisign -V`, identifying the release:

```
Trusted comment: ProGit <version> <target> signed by Markus Maiwald
```

The trusted comment is part of what is signed, so it cannot be altered without
invalidating the signature.

## Signing key

- The public verification key is committed at `keys/progit-minisign.pub`.
- The secret key is NEVER committed. It lives in the CI secret store
  (`MINISIGN_SECRET_KEY`) and in an offline backup held by the maintainer.
- It is generated unencrypted (`minisign -G -W`) so CI can sign without an
  interactive password; its confidentiality is protected by the secret store,
  not a passphrase.
- Rotation: generate a new keypair, commit the new public key, update the CI
  secret, and announce the change so downstreams refresh the trusted key. Keep
  the previous public key available for verifying older releases.
