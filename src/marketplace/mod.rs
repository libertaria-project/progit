//! Marketplace - Secure plugin distribution infrastructure
//!
//! # Components
//!
//! - [crypto] - Cryptographic primitives (BLAKE3, Dilithium3)
//! - [keyring] - Local trust store
//! - [manifest] - Plugin manifest schema
//! - [hinge] - Hinge verification layer
//! - [submit] - Plugin submission CLI
//!
//! # Usage
//!
//! ```rust,ignore
//! use progit::marketplace::{Verifier, TrustPolicy};
//!
//! let verifier = Verifier::new(keyring);
//! let result = verifier.verify(&manifest)?;
//! ```

pub mod cli;
pub mod crypto;
pub mod hinge;
pub mod keyring;
pub mod manifest;
pub mod submit;

// [ARCH] These re-exports are the public marketplace API for library users.
// The binary crate also compiles this module tree, where they appear unused.
#[allow(unused_imports)]
pub use cli::{handle_deeplink, handle_plugin_verify, handle_trust_command};
#[allow(unused_imports)]
pub use crypto::{blake3_checksum, compute_keyid, generate_keypair, sign, verify, Algorithm};
#[allow(unused_imports)]
pub use hinge::{TrustPolicy, VerificationResult, Verifier};
#[allow(unused_imports)]
pub use keyring::Keyring;
#[allow(unused_imports)]
pub use manifest::{Artifact, Capabilities, LegacyPluginManifest, PluginManifest, Publisher};
