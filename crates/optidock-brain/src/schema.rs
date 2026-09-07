use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Serialized deterministic recovery context supplied to a diagnostic provider.
/// It intentionally stays provider-neutral to prevent a dependency cycle with the agent crate.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosisRequest {
    pub context: Value,
    pub recovery_attempt_count: usize,
}

/// Untrusted model output. The recovery crate validates enum names and commands before use.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiagnosisResponse {
    pub refined_root_cause: String,
    pub confidence: f32,
    pub recommended_action: String,
    pub human_explanation: String,
    pub diagnostic_steps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn response_fixture_round_trips() {
        let request = DiagnosisRequest { context: json!({"container": "api"}), recovery_attempt_count: 2 };
        assert_eq!(serde_json::to_value(&request).unwrap()["recovery_attempt_count"], 2);
        let response: DiagnosisResponse = serde_json::from_str(r#"{"refined_root_cause":"ResourceExhaustion","confidence":0.9,"recommended_action":"RestartContainer","human_explanation":"Increase memory.","diagnostic_steps":["docker inspect api"]}"#).unwrap();
        assert_eq!(response.recommended_action, "RestartContainer");
    }
}
