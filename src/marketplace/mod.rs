//! ProGit Marketplace Module
//!
//! This module provides the secure plugin distribution infrastructure based on
//! Janus Hinge's supply chain principles.
//!
//! ## Key Components
//!
//! - **Verification**: Hinge-compatible signature verification
//! - **Keyring**: Local trust store for publisher keys
//! - **Manifest**: Plugin manifest schema with signatures
//! - **Registry**: Marketplace registry client

pub mod cli;
pub mod crypto;
pub mod keyring;
pub mod manifest;

// Re-export from submodules
pub use crypto::compute_keyid;
pub use keyring::Keyring;

// Hinge verification
mod hinge;
pub use hinge::{TrustPolicy, Verifier};
