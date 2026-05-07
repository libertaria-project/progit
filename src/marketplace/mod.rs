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
//! ```rust
//! use progit::marketplace::{Verifier, TrustPolicy};
//!
//! let verifier = Verifier::new(keyring);
//! let result = verifier.verify(&manifest)?;
//! ```

pub mod crypto;
pub mod keyring;
pub mod manifest;
pub mod hinge;
pub mod submit;
pub mod cli;

pub use crypto::{blake3_checksum, compute_keyid, generate_keypair, sign, verify, Algorithm};
pub use keyring::Keyring;
pub use manifest::{PluginManifest, LegacyPluginManifest, Publisher, Artifact, Capabilities};
pub use hinge::{Verifier, TrustPolicy, VerificationResult};
pub use cli::{handle_trust_command, handle_plugin_verify, handle_deeplink};
