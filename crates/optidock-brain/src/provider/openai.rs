use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::{prompt, DiagnosisRequest, DiagnosisResponse, DiagnosticProvider};

/// OpenAI-compatible diagnostic provider. It also supports compatible local endpoints.
pub struct OpenAiDiagnosticProvider {
    client: reqwest::Client,
    api_base: String,
    api_key: String,
    model: String,
}

impl OpenAiDiagnosticProvider {
    /// Creates a provider from explicit connection settings.
    pub fn new(api_base: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self { client: reqwest::Client::new(), api_base: api_base.into().trim_end_matches('/').to_string(), api_key: api_key.into(), model: model.into() }
    }

    async fn complete(&self, user: String) -> Result<String> {
        let response = self.client.post(format!("{}/chat/completions", self.api_base))
            .bearer_auth(&self.api_key)
            .json(&json!({"model": self.model, "temperature": 0, "response_format": {"type": "json_object"}, "messages": [{"role":"system","content":prompt::SYSTEM_PROMPT},{"role":"user","content":user}]}))
            .send().await?.error_for_status()?;
        let value: serde_json::Value = response.json().await?;
        value.pointer("/choices/0/message/content").and_then(serde_json::Value::as_str).map(str::to_owned)
            .context("provider response did not contain choices[0].message.content")
    }
}

#[async_trait]
impl DiagnosticProvider for OpenAiDiagnosticProvider {
    async fn diagnose(&self, request: &DiagnosisRequest) -> Result<DiagnosisResponse> {
        let first = self.complete(prompt::user_prompt(request)).await?;
        match serde_json::from_str(&first) {
            Ok(response) => Ok(response),
            Err(error) => {
                let repaired = self.complete(format!("Your last response was not valid JSON matching the required schema: {error}. Return corrected JSON only. Original response: {first}")).await?;
                serde_json::from_str(&repaired).map_err(|_| anyhow::anyhow!("invalid diagnosis JSON after repair; raw model response: {repaired}"))
            }
        }
    }
}
