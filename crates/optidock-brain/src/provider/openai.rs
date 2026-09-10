use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::future::Future;

use crate::{prompt, DiagnosisRequest, DiagnosisResponse, DiagnosticProvider};

/// Resolve an OpenAI-compatible base URL into the full chat-completions endpoint.
///
/// Heuristic rules (a documented escape hatch exists: pass a full URL ending in
/// `/chat/completions` and it passes through unchanged):
/// - trailing slashes are trimmed;
/// - a base that already contains a version segment (`v1`, `v1beta`, `v2`, ...)
///   gets `/chat/completions` appended directly — this covers both `https://…/v1`
///   and Gemini's compatible endpoint `https://generativelanguage.googleapis.com/v1beta/openai`;
/// - otherwise `/v1/chat/completions` is appended.
pub fn chat_completions_url(raw: &str) -> String {
    let base = raw.trim().trim_end_matches('/');
    let lowered = base.to_ascii_lowercase();
    if lowered.ends_with("/chat/completions") {
        return base.to_string();
    }
    // A "version" segment starts with `v` followed by a digit (`v1`, `v2`, `v1beta`).
    let has_version_segment = lowered.split('/').any(|segment| {
        segment.starts_with('v')
            && segment.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
    });
    if has_version_segment {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

/// Generic OpenAI-compatible diagnostic provider. It also serves as the shared
/// transport for any provider that speaks `/chat/completions` (OpenAI, OpenRouter,
/// Groq, Ollama, vLLM, LM Studio, llama.cpp, Together, DeepSeek, and Gemini via
/// Google's OpenAI-compatible endpoint) — configured purely by base URL, key, and
/// model. Only used when `OPTIDOCK_AGENT_AI_DIAGNOSIS` is enabled and a key is present.
pub struct OpenAiDiagnosticProvider {
    client: reqwest::Client,
    endpoint: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiDiagnosticProvider {
    /// Creates a provider from explicit connection settings.
    ///
    /// `api_base` may be a bare host, a `…/v1` base, or a full `…/chat/completions`
    /// URL (see [`chat_completions_url`]). An empty `api_key` (local endpoints such
    /// as Ollama) results in no `Authorization` header being sent.
    pub fn new(
        api_base: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let api_key = if api_key.trim().is_empty() {
            None
        } else {
            Some(api_key)
        };
        Self {
            client: reqwest::Client::new(),
            endpoint: chat_completions_url(&api_base.into()),
            api_key,
            model: model.into(),
        }
    }

    fn auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => request.bearer_auth(key),
            None => request,
        }
    }

    async fn complete_json(&self, user: String) -> Result<String> {
        // Some compatible backends (Ollama, llama.cpp, Groq) reject
        // `response_format`; self-heal by retrying once without it on a 4xx.
        let with_format = self
            .auth(
                self.client
                    .post(&self.endpoint)
                    .json(&json!({
                        "model": self.model,
                        "temperature": 0,
                        "response_format": {"type": "json_object"},
                        "messages": [
                            {"role": "system", "content": prompt::SYSTEM_PROMPT},
                            {"role": "user", "content": user}
                        ]
                    })),
            )
            .send()
            .await;
        let response = match with_format {
            Ok(response) if response.status().is_client_error() => {
                self.auth(
                    self.client
                        .post(&self.endpoint)
                        .json(&json!({
                            "model": self.model,
                            "temperature": 0,
                            "messages": [
                                {"role": "system", "content": prompt::SYSTEM_PROMPT},
                                {"role": "user", "content": user}
                            ]
                        })),
                )
                .send()
                .await?
            }
            other => other?,
        };
        let response = response.error_for_status()?;
        let value: serde_json::Value = response.json().await?;
        value
            .pointer("/choices/0/message/content")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .context("provider response did not contain choices[0].message.content")
    }

    /// Plain chat completion (no structured `response_format`) — used by the live
    /// chat path, which reuses this same transport and settings.
    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        let response = self
            .auth(
                self.client
                    .post(&self.endpoint)
                    .json(&json!({
                        "model": self.model,
                        "temperature": 0,
                        "messages": [
                            {"role": "system", "content": system},
                            {"role": "user", "content": user}
                        ]
                    })),
            )
            .send()
            .await?
            .error_for_status()?;
        let value: serde_json::Value = response.json().await?;
        value
            .pointer("/choices/0/message/content")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
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
        Self::diagnose_with(|user| self.complete_json(user), request).await
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

    #[test]
    fn url_normalization() {
        // trailing slash trimmed + /v1 appended
        assert_eq!(chat_completions_url("https://api.openai.com/"), "https://api.openai.com/v1/chat/completions");
        // bare host -> /v1
        assert_eq!(chat_completions_url("https://api.openai.com"), "https://api.openai.com/v1/chat/completions");
        // already /v1 -> no doubling
        assert_eq!(chat_completions_url("https://api.openai.com/v1"), "https://api.openai.com/v1/chat/completions");
        // openrouter /api/v1 segment
        assert_eq!(chat_completions_url("https://openrouter.ai/api/v1"), "https://openrouter.ai/api/v1/chat/completions");
        // Gemini compatible endpoint
        assert_eq!(chat_completions_url("https://generativelanguage.googleapis.com/v1beta/openai"), "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions");
        // full-URL escape hatch
        assert_eq!(chat_completions_url("http://127.0.0.1:11434/v1/chat/completions"), "http://127.0.0.1:11434/v1/chat/completions");
        // local base without /v1
        assert_eq!(chat_completions_url("http://127.0.0.1:8080"), "http://127.0.0.1:8080/v1/chat/completions");
        // empty-ish base still yields something sane
        assert_eq!(chat_completions_url(""), "/v1/chat/completions");
    }

    #[test]
    fn empty_key_produces_no_auth() {
        assert!(OpenAiDiagnosticProvider::new("http://localhost/v1", "", "m").api_key.is_none());
        assert!(OpenAiDiagnosticProvider::new("http://localhost/v1", "   ", "m").api_key.is_none());
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