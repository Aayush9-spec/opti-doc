//! Provider-agnostic, advisory diagnostics for OptiDock recovery escalations.

pub mod prompt;
pub mod provider;
pub mod schema;

use anyhow::Result;
use async_trait::async_trait;
pub use schema::{DiagnosisRequest, DiagnosisResponse};

/// An LLM backend that can produce a structured, advisory container diagnosis.
/// Implementations never execute the proposed action.
#[async_trait]
pub trait DiagnosticProvider: Send + Sync {
    /// Diagnose the supplied immutable recovery context.
    async fn diagnose(&self, request: &DiagnosisRequest) -> Result<DiagnosisResponse>;
}
