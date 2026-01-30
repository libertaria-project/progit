// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2025 Markus Maiwald
//
// ProGit Plugin System
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.

//! Plugin system for ProGit
//!
//! [ARCH] Apache 2.0 licensed to allow proprietary plugins.
//! Core TUI remains EUPL-1.2, but plugins can be any license.

pub mod sdk;
pub mod lua_engine;
pub mod manager;
pub mod cli;
pub mod registry;
pub mod lockfile;

// Re-export commonly used types
pub use sdk::*;
pub use lua_engine::LuaPluginEngine;
pub use manager::PluginManager;
pub use registry::PluginRegistry;

