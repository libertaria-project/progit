// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Ollama Client
//! 
//! Connects to a local Ollama instance (default: http://localhost:11434).

use super::{AgentClient, AgentEvent, AgentRequest};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub struct OllamaClient {
    base_url: String,
}

impl OllamaClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            base_url: url.into(),
        }
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new("http://localhost:11434")
    }
}

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
    #[serde(default)]
    #[allow(dead_code)]
    total_duration: u64,
}

impl AgentClient for OllamaClient {
    fn stream_completion(
        &self,
        req: AgentRequest,
        tx: Sender<AgentEvent>,
        session_id: String,
    ) -> Result<()> {
        let url = format!("{}/api/generate", self.base_url);
        
        let body = OllamaGenerateRequest {
            model: req.model,
            prompt: req.prompt,
            system: req.system_prompt,
            stream: true,
            options: Some(OllamaOptions {
                temperature: req.temperature,
            }),
        };

        // Notify start
        let _ = tx.send(AgentEvent::Started(session_id.clone()));

        // Make request (blocking)
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120)) // Long timeout for LLM
            .build()?;

        let res = client.post(&url)
            .json(&body)
            .send()
            .context("Failed to connect to Ollama")?;

        if !res.status().is_success() {
            let err = format!("Ollama returned {}", res.status());
            let _ = tx.send(AgentEvent::Error(session_id.clone(), err.clone()));
            return Err(anyhow!(err));
        }

        // Stream response line by line
        let reader = BufReader::new(res);
        let mut full_response = String::new();

        for line in reader.lines() {
            let line = line?;
            if let Ok(json) = serde_json::from_str::<OllamaResponse>(&line) {
                if !json.response.is_empty() {
                    let _ = tx.send(AgentEvent::Token(session_id.clone(), json.response.clone()));
                    full_response.push_str(&json.response);
                }

                if json.done {
                    break;
                }
            }
        }

        let _ = tx.send(AgentEvent::Completed(session_id, full_response));
        Ok(())
    }
}
