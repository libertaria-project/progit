//! Plugin submission to marketplace registry

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Submit a plugin to the marketplace registry
pub async fn submit_plugin(
    manifest_path: PathBuf,
    keyid: &str,
    registry_url: Option<&str>,
) -> Result<()> {
    let registry = registry_url.unwrap_or("https://registry.progit.dev/api/submit");

    // Read and validate manifest
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;

    let manifest: serde_json::Value =
        serde_json::from_str(&content).context("Invalid JSON manifest")?;

    // Check for signature
    if !manifest
        .get("signature")
        .and_then(|s| s.get("value"))
        .is_some()
    {
        anyhow::bail!("Manifest must be signed before submission. Use 'sign-plugin' tool.");
    }

    // Submit via HTTP
    let client = reqwest::Client::new();
    let response = client
        .post(registry)
        .json(&serde_json::json!({
            "manifest": manifest,
            "keyid": keyid,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("Plugin submitted successfully!");
        println!("It will be reviewed within 24-48 hours.");
    } else {
        let error = response.text().await.unwrap_or_default();
        anyhow::bail!("Submission failed: {}", error);
    }

    Ok(())
}
