# Binary Size Audit Report

**Date:** 2026-01-16
**Baseline:** 17 MiB (release, stripped)
**Target:** <10 MiB (stretch: <5 MiB)

## Current Profile Settings (Already Optimized)

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## Size Breakdown by Crate

| Crate | Size | % | Source | Action |
|-------|------|---|--------|--------|
| `openssl_sys` | 1.9 MiB | 18.2% | git2 vendored | **HIGH: Switch to system OpenSSL or gitoxide** |
| `zbus` + `zvariant` + `secret_service` + `async_io` | ~1.2 MiB | 11% | keyring (Linux D-Bus) | **MEDIUM: Make optional feature** |
| `libgit2_sys` | 346 KiB | 3.2% | git2 | Coupled with git2 |
| `rustls` + `ring` | ~600 KiB | 5.6% | reqwest TLS | Consider native-tls as option |
| `regex_automata` + `regex_syntax` + `aho_corasick` | ~600 KiB | 5.5% | regex crate | Minimal - needed |
| `clap_builder` | 218 KiB | 2.0% | CLI parsing | Acceptable |
| `mlua_sys` + `mlua` | ~270 KiB | 2.5% | Plugin runtime | **LOW: Make optional** |
| `pulldown_cmark` | 105 KiB | 1.0% | Markdown | Acceptable |
| `syntect` + `onig_sys` | ~70 KiB | 0.7% | Syntax highlighting | Acceptable |
| `prog` (our code) | 744 KiB | 6.8% | Application | Acceptable |
| `std` | 1.1 MiB | 10.1% | Rust stdlib | Fixed |

## Recommendations

### 1. HIGH IMPACT: Replace git2's vendored OpenSSL (~1.9 MiB saved)

**Option A: Use system OpenSSL (Recommended for Linux)**
```toml
git2 = { version = "0.20", default-features = false, features = ["https"] }
```
- Removes vendored OpenSSL
- Uses system's libssl
- Trade-off: Requires OpenSSL installed on target system

**Option B: Migrate to gitoxide (gix)**
- Pure Rust, smaller binary
- Active development, modern API
- Trade-off: API changes needed

### 2. MEDIUM IMPACT: Feature-gate keyring (~1.2 MiB saved)

```toml
[features]
default = ["keyring"]
keyring = ["dep:keyring"]
minimal = []  # No keyring, manual token input only
```

The Linux keyring integration (`secret_service` via D-Bus) is heavy. Users who don't need secure credential storage could use `--no-default-features`.

### 3. LOW IMPACT: Feature-gate plugins (~270 KiB saved)

```toml
[features]
default = ["plugins"]
plugins = ["dep:mlua"]
```

Not all users need LuaJIT plugin support.

### 4. MINIMAL: opt-level = "z"

May reduce binary ~10-15% but impacts performance. Not recommended for a TUI app where responsiveness matters.

## Implementation Plan

### Phase 1: Quick Wins (No Breaking Changes)
- [ ] Test with system OpenSSL instead of vendored
- [ ] Add `minimal` feature flag for keyring-free builds

### Phase 2: Feature Flags
- [ ] Make `plugins` feature optional
- [ ] Document feature combinations in README

### Phase 3: Architectural (Future)
- [ ] Evaluate gitoxide migration (significant work)
- [ ] Consider lazy-loading syntax definitions

## Estimated Savings

| Optimization | Savings | Effort |
|--------------|---------|--------|
| System OpenSSL | ~1.9 MiB | Low |
| Optional keyring | ~1.2 MiB | Medium |
| Optional plugins | ~270 KiB | Low |
| **Total potential** | **~3.4 MiB** | - |

**Projected size:** ~13.6 MiB (from 17 MiB)

With all optimizations: ~10-12 MiB realistic target.

## Notes

- The 17 MiB baseline is reasonable for a feature-rich TUI with git integration
- Major reductions require trading portability for size
- Consider providing both "full" and "minimal" release binaries
