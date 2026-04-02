use anyhow::{Context, Result};
use std::time::Instant;

use super::algorithm_prompts;
use super::algorithm_types::{
    AlgorithmExtractionOutput, AlgorithmExtractionResult, AlgorithmInventory,
    AlgorithmVerificationResult,
};
use super::claude_cli::ClaudeCli;

pub struct AlgorithmExtractor {
    cli: ClaudeCli,
}

impl AlgorithmExtractor {
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

    /// Run the 3-pass algorithm extraction pipeline.
    /// `text_path` is the path to the extracted paper text file (injected via system prompt).
    pub async fn extract_algorithms(
        &self,
        text_path: &str,
    ) -> Result<AlgorithmExtractionResult> {
        let total_start = Instant::now();
        tracing::info!("Starting 3-pass algorithm extraction (text: {})", text_path);

        // Pass 1: Algorithm Inventory (Haiku — identify algorithms)
        tracing::info!("Pass 1/3: Identifying algorithms (haiku)...");
        let pass1_start = Instant::now();
        let inventory_prompt = algorithm_prompts::algorithm_inventory_prompt();
        let inventory_raw = self
            .cli
            .call_claude_with_context(&inventory_prompt, "haiku", Some(text_path))
            .await
            .context("Pass 1 (algorithm inventory) failed")?;
        let inventory: AlgorithmInventory = serde_json::from_value(inventory_raw)
            .context("Failed to parse algorithm inventory JSON")?;
        tracing::info!(
            "Pass 1 complete in {:.1}s: {} algorithms identified",
            pass1_start.elapsed().as_secs_f64(),
            inventory.algorithms.len(),
        );

        if inventory.algorithms.is_empty() {
            tracing::info!("No algorithms found in paper — returning empty result");
            return Ok(AlgorithmExtractionResult {
                algorithms: Vec::new(),
                verification: None,
            });
        }

        // Pass 2: Algorithm Extraction (Sonnet — full definitions)
        tracing::info!("Pass 2/3: Extracting algorithm definitions (sonnet)...");
        let pass2_start = Instant::now();
        let inventory_json = serde_json::to_string_pretty(&inventory)?;
        let extraction_prompt =
            algorithm_prompts::algorithm_extraction_prompt(&inventory_json);
        let algorithms_raw = self
            .cli
            .call_claude_with_context(&extraction_prompt, "sonnet", Some(text_path))
            .await
            .context("Pass 2 (algorithm extraction) failed")?;
        let extraction: AlgorithmExtractionOutput = serde_json::from_value(algorithms_raw)
            .context("Failed to parse algorithm extraction JSON")?;
        tracing::info!(
            "Pass 2 complete in {:.1}s: {} algorithms extracted",
            pass2_start.elapsed().as_secs_f64(),
            extraction.algorithms.len(),
        );

        // Pass 3: Verification (Haiku — check completeness)
        tracing::info!("Pass 3/3: Verifying algorithm definitions (haiku)...");
        let pass3_start = Instant::now();
        let algorithms_json = serde_json::to_string_pretty(&extraction)?;
        let verification_prompt =
            algorithm_prompts::algorithm_verification_prompt(&algorithms_json);
        let verification = match self.cli.call_claude(&verification_prompt, "haiku").await {
            Ok(v) => match serde_json::from_value::<AlgorithmVerificationResult>(v) {
                Ok(vr) => {
                    tracing::info!(
                        "Pass 3 complete in {:.1}s: status={}, quality={}, issues={}",
                        pass3_start.elapsed().as_secs_f64(),
                        vr.verification_status,
                        vr.overall_quality,
                        vr.completeness_issues.len(),
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
            "Algorithm extraction complete in {:.1}s total ({} algorithms)",
            total_start.elapsed().as_secs_f64(),
            extraction.algorithms.len(),
        );

        Ok(AlgorithmExtractionResult {
            algorithms: extraction.algorithms,
            verification,
        })
    }
}
