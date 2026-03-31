use anyhow::{Context, Result, bail};
use std::time::Instant;

pub struct ClaudeCli {
    pub claude_path: String,
}

impl ClaudeCli {
    pub fn new() -> Self {
        Self {
            claude_path: "claude".to_string(),
        }
    }

    pub fn with_path(claude_path: String) -> Self {
        Self { claude_path }
    }

    /// Call Claude Code CLI in headless mode and parse JSON response.
    /// Pipes the prompt via stdin to avoid OS argument length limits.
    pub async fn call_claude(&self, prompt: &str, model: &str) -> Result<serde_json::Value> {
        let start = Instant::now();
        tracing::info!(
            "Calling claude --model {} ({} char prompt)...",
            model,
            prompt.len(),
        );

        use tokio::io::AsyncWriteExt;

        let mut child = tokio::process::Command::new(&self.claude_path)
            .arg("--print")
            .arg("--model")
            .arg(model)
            .arg("--output-format")
            .arg("json")
            .arg("--max-turns")
            .arg("1")
            .arg("--no-session-persistence")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn claude CLI — is it installed?")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .context("Failed to write prompt to claude stdin")?;
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
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::error!("Claude CLI stderr: {}", stderr);
            tracing::error!("Claude CLI stdout: {}", stdout);
            // Extract error message from JSON stdout if possible
            let error_msg = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                json.get("error")
                    .or_else(|| json.get("message"))
                    .or_else(|| json.get("result"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&stderr)
                    .to_string()
            } else if !stderr.is_empty() {
                stderr.to_string()
            } else {
                stdout.to_string()
            };
            bail!(
                "Claude CLI failed (exit {}) after {:.1}s: {}",
                output.status,
                elapsed.as_secs_f64(),
                error_msg,
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let outer: serde_json::Value =
            serde_json::from_str(&stdout).context("Failed to parse Claude CLI JSON output")?;

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

        let cleaned = strip_code_fences(response_text);

        let parsed: serde_json::Value =
            serde_json::from_str(&cleaned).with_context(|| {
                format!(
                    "Failed to parse inner JSON from Claude response. Last 200 chars: ...{}",
                    &cleaned[cleaned.len().saturating_sub(200)..],
                )
            })?;

        tracing::debug!(
            "Parsed JSON response ({} top-level keys)",
            parsed.as_object().map(|o| o.len()).unwrap_or(0),
        );

        Ok(parsed)
    }
}

/// Strip markdown code fences (```json ... ```) from a string.
pub fn strip_code_fences(s: &str) -> String {
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
