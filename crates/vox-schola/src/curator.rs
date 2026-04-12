//! Curator logic for validating generated prose prior to persistence.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorVerdict {
    pub score: f64,
    pub entropy: f64,
    pub accepted: bool,
}

/// Evaluates generated prose using a frontier API (via simple HTTP) for quality gating.
///
/// Rejects prose if typicality > 0.7 or semantic entry < 0.3.
pub async fn curator_validate(prose: &str, api_key: &str) -> anyhow::Result<CuratorVerdict> {
    // We parse constraints according to the task: typicality > 0.7 or entropy < 0.3 means REJECT.
    // In Schola we don't naturally depend on vox-runtime so we make a direct HTTP call if needed,
    // or simulate an LLM-as-judge response if the feature is missing or we want a fast fallback.

    // A real frontier API call would build a prompt here.
    // As a baseline, simple typicality heuristic fallback:
    if prose.is_empty() {
        return Ok(CuratorVerdict {
            score: 1.0,
            entropy: 0.0,
            accepted: false,
        });
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "You are a curator evaluating synthetic data. Return a JSON object with 'score' (0.0 to 1.0, where >0.7 is bad typicality), 'entropy' (0.0 to 1.0, where <0.3 is bad repetition), and 'accepted' (boolean true if score<=0.7 and entropy>=0.3). Do NOT wrap in markdown."
            },
            {
                "role": "user",
                "content": prose
            }
        ],
        "response_format": { "type": "json_object" },
        "temperature": 0.0
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        return Err(anyhow::anyhow!("Curator API returned error: {}", err_text));
    }

    let val: serde_json::Value = res.json().await?;
    let content = val["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Unexpected response format: no content string"))?;

    let verdict: CuratorVerdict = serde_json::from_str(content)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON response: {}", e))?;

    Ok(verdict)
}

pub async fn run_gate(
    data_file: std::path::PathBuf,
    out_file: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use std::io::{BufRead, Write};

    let api_key = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenaiApiKey)
        .expose()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY missing, required for curator gate"))?;

    println!("curator-gate: loading jsonl from {}", data_file.display());
    if !data_file.exists() {
        anyhow::bail!("Data file {} does not exist", data_file.display());
    }

    let file = std::fs::File::open(&data_file)?;
    let reader = std::io::BufReader::new(file);

    let outfile_path = out_file.unwrap_or_else(|| {
        let mut d = data_file.clone();
        d.set_extension("gated.jsonl");
        d
    });

    let mut out_f = std::fs::File::create(&outfile_path)?;
    let mut total = 0;
    let mut passed = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        // Assume corpus format has "text" or "completion" field
        let v: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
        let prose = v
            .get("text")
            .or(v.get("completion"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let mut accept = true;

        if !prose.is_empty() {
            match curator_validate(prose, &api_key).await {
                Ok(verdict) => {
                    accept = verdict.accepted;
                    if !accept {
                        println!(
                            "curator-gate rejected line due to typicality/entropy: score={}, entropy={}",
                            verdict.score, verdict.entropy
                        );
                    }
                }
                Err(e) => {
                    eprintln!("curator-gate error on line: {}", e);
                    // fail-open or fail-closed? Let's fail-open for now on API errors
                }
            }
        }

        if accept {
            writeln!(out_f, "{}", line)?;
            passed += 1;
        }
    }

    println!(
        "curator-gate: Processed {} lines, retained {} lines. Wrote to {}",
        total,
        passed,
        outfile_path.display()
    );
    // If inline out_file was not provided, but they want inline replacement, you could rename files here.
    // Usually CLI defaults to non-destructive write if out_file is omitted unless we want to overwrite.
    Ok(())
}
