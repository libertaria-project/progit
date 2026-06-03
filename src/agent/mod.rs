// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! AI Agent Module
//!
//! Handles communication with LLM providers (currently Ollama) to power
//! virtual branch agents.

pub mod context;
pub mod ollama;
pub mod ops;

use serde::Serialize;
use std::sync::mpsc::Sender;

/// Events sent from the Agent thread to the main TUI thread
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent started thinking
    Started(String), // Session ID
    /// Token received (for streaming output)
    Token(String, String), // Session ID, Token
    /// Agent finished successfully
    Completed(String, String), // Session ID, Full Response
    /// Agent encountered an error
    Error(String, String), // Session ID, Error Message
}

/// Request to an agent
#[derive(Debug, Clone, Serialize)]
pub struct AgentRequest {
    pub model: String,
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub temperature: f32,
}

impl Default for AgentRequest {
    fn default() -> Self {
        Self {
            model: "deepseek-coder".to_string(),
            prompt: "".to_string(),
            system_prompt: None,
            temperature: 0.7,
        }
    }
}

/// Trait for LLM backends
pub trait AgentClient {
    /// Send a prompt and stream the response to the sender
    fn stream_completion(
        &self,
        req: AgentRequest,
        tx: Sender<AgentEvent>,
        session_id: String,
    ) -> anyhow::Result<()>;
}
