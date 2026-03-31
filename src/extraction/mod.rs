pub mod algorithm_prompts;
pub mod algorithm_types;
pub mod algorithms;
pub mod claude_cli;
pub mod prompts;
pub mod types;

use anyhow::{Context, Result};
use claude_cli::ClaudeCli;
use std::time::Instant;
use types::{EvidenceInventory, ExtractionResult, PatternExtractionOutput, VerificationResult};

pub use algorithms::AlgorithmExtractor;

pub struct PatternExtractor {
    cli: ClaudeCli,
}

impl PatternExtractor {
    pub fn new() -> Self {
        Self {
            cli: ClaudeCli::new(),
        }
    }

    pub fn with_path(claude_path: String) -> Self {
        Self {
            cli: ClaudeCli::with_path(claude_path),
        }
    }

    /// Run the 3-pass extraction pipeline on paper text.
    pub async fn extract_patterns(&self, text: &str) -> Result<ExtractionResult> {
        let total_start = Instant::now();
        let text_len = text.len();
        let word_count = text.split_whitespace().count();
        tracing::info!(
            "Starting 3-pass extraction pipeline (text: {} chars, ~{} words)",
            text_len,
            word_count,
        );

        // Pass 1: Evidence Inventory (Haiku — fast, cheap)
        tracing::info!("Pass 1/3: Extracting evidence inventory (haiku)...");
        let pass1_start = Instant::now();
        let evidence_prompt = prompts::evidence_inventory_prompt(text);
        tracing::debug!("Pass 1 prompt: {} chars", evidence_prompt.len());
        let evidence_raw = self
            .cli
            .call_claude(&evidence_prompt, "haiku")
            .await
            .context("Pass 1 (evidence inventory) failed")?;
        let raw_str = serde_json::to_string_pretty(&evidence_raw).unwrap_or_default();
        tracing::info!(
            "Pass 1 raw JSON type={}, len={}, first 1000 chars:\n{}",
            match &evidence_raw {
                serde_json::Value::Object(_) => "object",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Bool(_) => "bool",
                serde_json::Value::Null => "null",
            },
            raw_str.len(),
            &raw_str[..raw_str.len().min(1000)],
        );
        let evidence: EvidenceInventory =
            serde_json::from_value(evidence_raw.clone()).with_context(|| {
                format!(
                    "Failed to parse evidence inventory JSON. First 500 chars: {}",
                    &raw_str[..raw_str.len().min(500)],
                )
            })?;
        tracing::info!(
            "Pass 1 complete in {:.1}s: {} evidence items extracted (paper: \"{}\")",
            pass1_start.elapsed().as_secs_f64(),
            evidence.evidence_items.len(),
            evidence.paper_title,
        );

        // Pass 2: Pattern Extraction (Sonnet — most capable)
        tracing::info!("Pass 2/3: Extracting patterns with evidence citations (sonnet)...");
        let pass2_start = Instant::now();
        let evidence_json = serde_json::to_string_pretty(&evidence)?;
        let extraction_prompt = prompts::pattern_extraction_prompt(text, &evidence_json);
        tracing::debug!("Pass 2 prompt: {} chars", extraction_prompt.len());
        let patterns_raw = self
            .cli
            .call_claude(&extraction_prompt, "sonnet")
            .await
            .context("Pass 2 (pattern extraction) failed")?;
        let extraction: PatternExtractionOutput = serde_json::from_value(patterns_raw)
            .context("Failed to parse pattern extraction JSON")?;
        tracing::info!(
            "Pass 2 complete in {:.1}s: {} patterns extracted, {} evidence cited",
            pass2_start.elapsed().as_secs_f64(),
            extraction.patterns.len(),
            extraction.total_evidence_used,
        );

        // Pass 3: Verification (Haiku — fast verification)
        tracing::info!("Pass 3/3: Verifying extraction quality (haiku)...");
        let pass3_start = Instant::now();
        let patterns_json = serde_json::to_string_pretty(&extraction)?;
        let verification_prompt = prompts::verification_prompt(&evidence_json, &patterns_json);
        tracing::debug!("Pass 3 prompt: {} chars", verification_prompt.len());
        let verification = match self.cli.call_claude(&verification_prompt, "haiku").await {
            Ok(v) => match serde_json::from_value::<VerificationResult>(v) {
                Ok(vr) => {
                    tracing::info!(
                        "Pass 3 complete in {:.1}s: status={}, quality={}, issues={}, unused_evidence={}",
                        pass3_start.elapsed().as_secs_f64(),
                        vr.verification_status,
                        vr.overall_quality,
                        vr.citation_issues.len(),
                        vr.unused_evidence.len(),
                    );
                    Some(vr)
                }
                Err(e) => {
                    tracing::warn!(
                        "Pass 3 failed to parse in {:.1}s: {}",
                        pass3_start.elapsed().as_secs_f64(),
                        e,
                    );
                    None
                }
            },
            Err(e) => {
                tracing::warn!(
                    "Pass 3 call failed in {:.1}s (non-fatal): {}",
                    pass3_start.elapsed().as_secs_f64(),
                    e,
                );
                None
            }
        };

        tracing::info!(
            "Extraction pipeline complete in {:.1}s total ({} patterns from {} evidence items)",
            total_start.elapsed().as_secs_f64(),
            extraction.patterns.len(),
            evidence.evidence_items.len(),
        );

        Ok(ExtractionResult {
            patterns: extraction.patterns,
            evidence,
            verification,
        })
    }
}
