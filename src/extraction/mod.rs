pub mod prompts;
pub mod types;

use anyhow::{Context, Result, bail};
use std::time::Instant;
use types::{
    EvidenceInventory, ExtractionResult, PatternExtractionOutput, VerificationResult,
};

pub struct PatternExtractor {
    claude_path: String,
}

impl PatternExtractor {
    pub fn new() -> Self {
        Self {
            claude_path: "claude".to_string(),
        }
    }

    pub fn with_path(claude_path: String) -> Self {
        Self { claude_path }
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
        let evidence_raw = self.call_claude(&evidence_prompt, "haiku").await
            .context("Pass 1 (evidence inventory) failed")?;
        let evidence: EvidenceInventory = serde_json::from_value(evidence_raw)
            .context("Failed to parse evidence inventory JSON")?;
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
        let patterns_raw = self.call_claude(&extraction_prompt, "sonnet").await
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
        let verification = match self.call_claude(&verification_prompt, "haiku").await {
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

    /// Call Claude Code CLI in headless mode and parse JSON response.
    /// Pipes the prompt via stdin to avoid file permission issues.
    async fn call_claude(&self, prompt: &str, model: &str) -> Result<serde_json::Value> {
        let start = Instant::now();
        tracing::info!(
            "Calling claude --model {} ({} char prompt)...",
            model,
            prompt.len(),
        );

        use tokio::io::AsyncWriteExt;

        let mut child = tokio::process::Command::new(&self.claude_path)
            .arg("-p")
            .arg("-")
            .arg("--model")
            .arg(model)
            .arg("--output-format")
            .arg("json")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn claude CLI — is it installed?")?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await
                .context("Failed to write prompt to claude stdin")?;
            // Drop stdin to close it and signal EOF
        }

        let output = child
            .wait_with_output()
            .await
            .context("Failed to read claude CLI output")?;

        let elapsed = start.elapsed();
        tracing::info!(
            "Claude CLI ({}) returned in {:.1}s (exit: {}, stdout: {} bytes)",
            model,
            elapsed.as_secs_f64(),
            output.status,
            output.stdout.len(),
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Claude CLI stderr: {}", stderr);
            bail!(
                "Claude CLI failed (exit {}) after {:.1}s: {}",
                output.status,
                elapsed.as_secs_f64(),
                stderr,
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Claude --output-format json returns { "result": "...", "cost_usd": ..., ... }
        let outer: serde_json::Value = serde_json::from_str(&stdout)
            .context("Failed to parse Claude CLI JSON output")?;

        if let Some(cost) = outer.get("cost_usd").and_then(|v| v.as_f64()) {
            tracing::info!("Claude CLI ({}) cost: ${:.4}", model, cost);
        }

        let response_text = outer
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| stdout.as_ref());

        tracing::debug!(
            "Raw response text (first 500 chars): {}",
            &response_text[..response_text.len().min(500)],
        );

        // Strip markdown code fences if present
        let cleaned = strip_code_fences(response_text);

        tracing::debug!(
            "Cleaned text (first 500 chars): {}",
            &cleaned[..cleaned.len().min(500)],
        );

        let parsed: serde_json::Value = serde_json::from_str(&cleaned)
            .with_context(|| format!(
                "Failed to parse inner JSON from Claude response. First 200 chars: {}",
                &cleaned[..cleaned.len().min(200)],
            ))?;

        tracing::debug!(
            "Parsed JSON response ({} top-level keys)",
            parsed.as_object().map(|o| o.len()).unwrap_or(0),
        );

        Ok(parsed)
    }
}

/// Strip markdown code fences (```json ... ```) from a string.
fn strip_code_fences(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with("```") {
        let without_start = if let Some(rest) = trimmed.strip_prefix("```json") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("```") {
            rest
        } else {
            trimmed
        };
        if let Some(content) = without_start.strip_suffix("```") {
            return content.trim().to_string();
        }
        return without_start.trim().to_string();
    }
    trimmed.to_string()
}
