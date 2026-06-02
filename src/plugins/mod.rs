// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Plugin system for ProGit.
//!
//! [ARCH] The TUI core (LCL-1.0) talks to plugins exclusively through the
//! `progit-plugin-sdk` crate (LSL-1.0). No `mlua` types and no concrete
//! runtime structs leak out of this module — that is the Trait Firewall
//! (Doctrine 4).
//!
//! Earlier revisions of this directory also held a parallel `Plugin` /
//! `PluginEngine` trait pair plus a second `LuaPluginEngine`. They were
//! never wired into `manager.rs` and have been deleted. If you find a
//! reference to them in old branches, that is the dead code, not this.

pub mod cli;
pub mod highlight_cache;
pub mod lang_detect;
pub mod lockfile;
pub mod manager;
pub mod registry;

// Re-export the event surface from the SDK so existing call sites
// (`crate::plugins::PluginEvent::...`) keep resolving without churn.
// `PipelineStatus`/`PipelineJob`/`PipelineState` live in
// `progit_plugin_sdk::event::*` for plugin-side consumers; the host
// only ever sees them as JSON, so we don't re-export them here.
pub use manager::PluginManager;
pub use progit_plugin_sdk::event::PluginEvent;
