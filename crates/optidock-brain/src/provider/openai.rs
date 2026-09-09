use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::future::Future;

use crate::{prompt, DiagnosisRequest, DiagnosisResponse, DiagnosticProvider};

/// OpenAI-compatible diagnostic provider. It also supports compatible local endpoints.
/// Only used when `OPTIDOCK_AGENT_AI_DIAGNOSIS` is enabled and an API key is present.
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

    /// Core diagnosis routine parameterized by the transport so tests can inject
    /// canned responses without a network round-trip.
    async fn diagnose_with<F, Fut>(complete: F, request: &DiagnosisRequest) -> Result<DiagnosisResponse>
    where
        F: Fn(String) -> Fut,
        Fut: Future<Output = Result<String>>,
    {
        let first = complete(prompt::user_prompt(request)).await?;
        match serde_json::from_str(&first) {
            Ok(response) => Ok(response),
            Err(error) => {
                let repaired = complete(format!(
                    "Your last response was not valid JSON matching the required schema: {error}. Return corrected JSON only. Original response: {first}"
                ))
                .await?;
                serde_json::from_str(&repaired).map_err(|_| anyhow::anyhow!(
                    "invalid diagnosis JSON after repair; raw model response: {repaired}"
                ))
            }
        }
    }
}

#[async_trait]
impl DiagnosticProvider for OpenAiDiagnosticProvider {
    async fn diagnose(&self, request: &DiagnosisRequest) -> Result<DiagnosisResponse> {
        Self::diagnose_with(|user| self.complete(user), request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    const FIXTURE_RESPONSE: &str = r#"{"refined_root_cause":"ResourceExhaustion","confidence":0.9,"recommended_action":"RestartContainer","human_explanation":"Container hit its memory ceiling and was OOM-killed.","diagnostic_steps":["docker stats api"]}"#;

    fn request() -> DiagnosisRequest {
        DiagnosisRequest {
            context: serde_json::json!({"container": "api"}),
            recovery_attempt_count: 2,
        }
    }

    #[tokio::test]
    async fn repairs_malformed_json_once_then_succeeds() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_closure = calls.clone();
        let result = OpenAiDiagnosticProvider::diagnose_with(
            move |_user: String| {
                let calls = calls_for_closure.clone();
                async move {
                    let n = calls.fetch_add(1, Ordering::SeqCst);
                    if n == 0 {
                        Ok("not valid json at all".to_string())
                    } else {
                        Ok(FIXTURE_RESPONSE.to_string())
                    }
                }
            },
            &request(),
        )
        .await;

        let response = result.expect("retry should recover from malformed JSON");
        assert_eq!(response.recommended_action, "RestartContainer");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn surfaces_error_when_repair_also_fails() {
        let result = OpenAiDiagnosticProvider::diagnose_with(
            |_user: String| async { Ok("still not valid json".to_string()) },
            &request(),
        )
        .await;

        let error = result.expect_err("two consecutive invalid responses must fail");
        assert!(error.to_string().contains("raw model response"), "{error}");
    }
}