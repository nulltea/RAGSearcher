use anyhow::{Context, Result, bail};
use std::path::Path;
use std::time::Instant;

pub struct ClaudeCli {
    pub claude_path: String,
}

/// Well-known locations for the Claude CLI binary.
const CLAUDE_SEARCH_PATHS: &[&str] = &["/usr/local/bin/claude", "/opt/homebrew/bin/claude"];

impl ClaudeCli {
    pub fn new() -> Self {
        let path = Self::resolve_claude_path().unwrap_or_else(|| "claude".to_string());
        tracing::info!("Claude CLI path: {}", path);
        Self { claude_path: path }
    }

    pub fn with_path(claude_path: String) -> Self {
        Self { claude_path }
    }

    /// Find the claude binary by checking ~/.local/bin first, then well-known paths.
    fn resolve_claude_path() -> Option<String> {
        // ~/.local/bin/claude (npm global install location)
        if let Some(home) = dirs::home_dir() {
            let local = home.join(".local/bin/claude");
            if local.exists() {
                return Some(local.to_string_lossy().to_string());
            }
        }

        for &p in CLAUDE_SEARCH_PATHS {
            if Path::new(p).exists() {
                return Some(p.to_string());
            }
        }

        // Fall back to bare "claude" and hope it's in PATH
        None
    }

    /// Read Claude OAuth access token from macOS keychain.
    fn read_oauth_token() -> Result<String> {
        // Check env var first (allows override / non-macOS usage)
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                return Ok(key);
            }
        }

        let output = std::process::Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                "Claude Code-credentials",
                "-w",
            ])
            .output()
            .context("Failed to run `security` — are you on macOS?")?;

        if !output.status.success() {
            bail!(
                "No Claude credentials in keychain. Set ANTHROPIC_API_KEY or run `claude` to log in."
            );
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let creds: serde_json::Value = serde_json::from_str(json_str.trim())
            .context("Failed to parse keychain credentials")?;

        creds
            .pointer("/claudeAiOauth/accessToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No accessToken found in keychain credentials"))
    }

    /// Call Claude Code CLI in headless mode and parse JSON response.
    pub async fn call_claude(&self, prompt: &str, model: &str) -> Result<serde_json::Value> {
        self.call_claude_with_context(prompt, model, None).await
    }

    /// Call Claude CLI with an optional file injected as system prompt context.
    /// Uses sandboxed workdir to isolate from project CLAUDE.md / hooks / settings.
    pub async fn call_claude_with_context(
        &self,
        prompt: &str,
        model: &str,
        context_file: Option<&str>,
    ) -> Result<serde_json::Value> {
        let start = Instant::now();

        // Sandboxed workdir so CLI can't discover project CLAUDE.md
        let work_dir = std::env::temp_dir().join(format!(
            "claude-rag-{}-{}",
            std::process::id(),
            start.elapsed().subsec_nanos(),
        ));
        std::fs::create_dir_all(&work_dir).context("Failed to create temp workdir")?;

        let claude_dir = work_dir.join(".claude");
        std::fs::create_dir_all(&claude_dir).context("Failed to create .claude dir")?;
        let settings = r#"{"disableAllHooks":true,"permissions":{"deny":["Bash(*)","Read","Write","Edit","Glob","Grep","Agent","WebFetch","WebSearch","NotebookEdit","TodoWrite","mcp__*"]},"mcpServers":{}}"#;
        std::fs::write(claude_dir.join("settings.json"), settings)
            .context("Failed to write settings.json")?;
        std::fs::write(claude_dir.join("settings.local.json"), settings)
            .context("Failed to write settings.local.json")?;

        let empty_mcp = work_dir.join("empty-mcp.json");
        std::fs::write(&empty_mcp, r#"{"mcpServers":{}}"#)
            .context("Failed to write empty MCP config")?;

        std::fs::write(
            work_dir.join("CLAUDE.md"),
            "HEADLESS EXTRACTION MODE: Output only the JSON object requested. \
             Do NOT use any tools. Do NOT read or write any files. \
             Do NOT update primer.md or lessons.md. \
             All AGENT RULES from global settings are suspended for this session.",
        )
        .context("Failed to write headless CLAUDE.md")?;

        let mut cmd = tokio::process::Command::new(&self.claude_path);
        cmd.arg("--print")
            .arg("--model")
            .arg(model)
            .arg("--output-format")
            .arg("json")
            .arg("--max-turns")
            .arg("1")
            .arg("--no-session-persistence")
            .arg("--strict-mcp-config")
            .arg("--mcp-config")
            .arg(&empty_mcp)
            .arg("--debug-file")
            .arg("/tmp/claude-extraction-debug.log");

        if let Some(file_path) = context_file {
            cmd.arg("--append-system-prompt-file").arg(file_path);
        }

        let result = self.run_claude_cmd(cmd, prompt, model, &start).await;
        let _ = std::fs::remove_dir_all(&work_dir);
        result
    }

    /// Call Claude CLI in --bare mode (no CLAUDE.md, hooks, skills, LSP, plugins).
    /// Auth via OAuth token read from macOS keychain or ANTHROPIC_API_KEY env var.
    pub async fn call_claude_bare(
        &self,
        prompt: &str,
        model: &str,
        context_file: Option<&str>,
    ) -> Result<serde_json::Value> {
        let start = Instant::now();
        let api_key = Self::read_oauth_token()?;

        let mut cmd = tokio::process::Command::new(&self.claude_path);
        cmd.arg("--print")
            .arg("--model")
            .arg(model)
            .arg("--output-format")
            .arg("json")
            .arg("--max-turns")
            .arg("1")
            .arg("--bare")
            .arg("--debug-file")
            .arg("/tmp/claude-extraction-debug.log")
            .env("ANTHROPIC_API_KEY", &api_key);

        if let Some(file_path) = context_file {
            cmd.arg("--append-system-prompt-file").arg(file_path);
        }

        self.run_claude_cmd(cmd, prompt, model, &start).await
    }

    /// Shared execution: pipe prompt via stdin, parse JSON result.
    async fn run_claude_cmd(
        &self,
        mut cmd: tokio::process::Command,
        prompt: &str,
        model: &str,
        start: &Instant,
    ) -> Result<serde_json::Value> {
        tracing::info!(
            "Calling claude --model {} ({} char prompt)...",
            model,
            prompt.len(),
        );

        use tokio::io::AsyncWriteExt;

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
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
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::error!("Claude CLI stdout: {}", stdout);
            let error_msg = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                json.get("error")
                    .or_else(|| json.get("message"))
                    .or_else(|| json.get("result"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string()
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

        let parsed: serde_json::Value = serde_json::from_str(&cleaned).with_context(|| {
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

#[test]
fn test_read_oauth_token() {
    println!("read_oauth_token(): {:?}", ClaudeCli::read_oauth_token());
}
