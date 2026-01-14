// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Plugin management system
//!
//! This module provides:
//! - Runtime plugin loading and execution (manager)
//! - CLI commands for plugin management (cli)
//! - Plugin registry client (registry)
//! - Lockfile management (lockfile)

pub mod cli;
pub mod lockfile;
pub mod manager;
pub mod registry;

// Re-export commonly used types
pub use manager::PluginManager;
