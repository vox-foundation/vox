use crate::cli_args::RepairArgs;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::process::Command;
use vox_config::inference::{OPENROUTER_CHAT_COMPLETIONS_URL, openrouter_chat_model_preference};
use vox_secrets::{SecretId, resolve_secret};

#[derive(Debug, Deserialize)]
struct SpanPayload {
    start_line: usize,
    #[allow(dead_code)]
    start_col: usize,
    #[allow(dead_code)]
    end_line: usize,
    #[allow(dead_code)]
    end_col: usize,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SuggestedFix {
    label: String,
    replacement: String,
    span: SpanPayload,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DiagnosticPayload {
    error_code: String,
    message: String,
    span: SpanPayload,
    correction_hints: Vec<String>,
    suggested_fixes: Vec<SuggestedFix>,
}

pub async fn run(args: RepairArgs) -> Result<()> {
    let file_path = &args.file;
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    println!("Starting automated repair loop for {}", file_path.display());

    let http = vox_http_client::client_builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut attempts = 0;
    let max_attempts = 3;

    while attempts < max_attempts {
        attempts += 1;
        println!("\nAttempt {}/{}...", attempts, max_attempts);

        // 1. Run `vox check --format json` on the file
        let output = Command::new(std::env::current_exe().unwrap_or_else(|_| "vox".into()))
            .arg("check")
            .arg("--output-format")
            .arg("json")
            .arg(file_path)
            .output()
            .context("Failed to run vox check")?;

        if output.status.success() {
            println!("✓ No errors found. File is clean!");
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let diagnostics: Vec<DiagnosticPayload> = match serde_json::from_str(&stdout) {
            Ok(d) => d,
            Err(_) => {
                // If it's not a list, maybe it's a single one or error
                eprintln!("Failed to parse JSON diagnostics. Raw output:\n{}", stdout);
                return Ok(());
            }
        };

        if diagnostics.is_empty() {
            println!(
                "✓ Check failed but returned no diagnostics. Assuming clean or external error."
            );
            return Ok(());
        }

        println!(
            "Found {} compiler errors. Generating repair patch via LLM...",
            diagnostics.len()
        );

        // 2. Resolve API key
        let token_opt = resolve_secret(SecretId::OpenRouterApiKey)
            .expose()
            .map(|s| s.to_string());
        let token = match token_opt {
            Some(t) => t,
            None => {
                anyhow::bail!(
                    "OpenRouter API key (VOX_OPENROUTER_API_KEY) not found. Repair requires an LLM backend."
                );
            }
        };

        // 3. Build prompt
        let source_code = fs::read_to_string(file_path)?;
        let mut error_summary = String::new();
        for d in &diagnostics {
            error_summary.push_str(&format!(
                "- [{}] Line {}: {}\n",
                d.error_code, d.span.start_line, d.message
            ));
            for hint in &d.correction_hints {
                error_summary.push_str(&format!("  Hint: {}\n", hint));
            }
        }

        let system_prompt = "You are an expert Vox language repair agent.
Your goal is to fix compiler errors in the provided Vox source code.
You will be given the original source code and a list of structured compiler diagnostics.
Return ONLY the full corrected source code inside a single markdown code block.
Do not provide explanations or chat.
Focus on correctness and adhering to Vox language standards (Wave 1: non-null by default, colon blocks).";

        let user_prompt = format!(
            "File: {}\n\nSOURCE CODE:\n```vox\n{}\n```\n\nCOMPILER ERRORS:\n{}\n\nPlease fix these errors and return the full corrected file.",
            file_path.display(),
            source_code,
            error_summary
        );

        // 4. Call LLM
        let openrouter_model = openrouter_chat_model_preference();
        println!("Calling LLM ({openrouter_model}) via OpenRouter...");
        let response = http
            .post(OPENROUTER_CHAT_COMPLETIONS_URL)
            .header("Authorization", format!("Bearer {}", token))
            .header("X-Title", "Vox Repair Loop")
            .json(&serde_json::json!({
                "model": openrouter_model,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_prompt }
                ],
                "temperature": 0.1,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("LLM API error ({}): {}", status, body);
        }

        let resp_json: serde_json::Value = response.json().await?;
        let assistant_text = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Empty LLM response"))?;

        // 5. Extract code block and apply
        let new_code = if let Some(start) = assistant_text.find("```") {
            let after_start = &assistant_text[start + 3..];
            let content_start = after_start.find('\n').map(|i| i + 1).unwrap_or(0);
            let content = &after_start[content_start..];
            if let Some(end) = content.find("```") {
                &content[..end]
            } else {
                content
            }
        } else {
            assistant_text
        };

        fs::write(file_path, new_code.trim())?;
        println!("✓ Applied suggested fix. Re-checking...");
    }

    println!(
        "Repair loop exhausted after {} attempts without converging.",
        max_attempts
    );
    Ok(())
}
