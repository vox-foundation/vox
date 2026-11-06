
// ─── Shell script SFT pairs ──────────────────────────────────────────────────

pub(crate) fn generate_script_pairs(out: &mut impl Write, _cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0;

    let scripts: &[(&str, &str, &str)] = &[
        // (prompt, script content, category)
        (
            "Write a PowerShell script to monitor QLoRA training telemetry",
            r#"$TelemetryPath = "mens\runs\qwen35_qlora\telemetry.jsonl"
if (Test-Path $TelemetryPath) {
    Get-Content $TelemetryPath -Wait -Tail 10 | ForEach-Object {
        $event = $_ | ConvertFrom-Json
        if ($event.event -eq "step") {
            Write-Host "Step $($event.payload.step) | Loss: $($event.payload.loss) | ETA: $($event.payload.eta_sec)s" -ForegroundColor Cyan
        }
    }
} else {
    Write-Host "Telemetry file not found: $TelemetryPath" -ForegroundColor Red
}"#,
            "powershell_script",
        ),
        (
            "Write a batch file to build Vox with CUDA support on Windows",
            r#"@echo off
setlocal
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if %errorlevel% neq 0 (
    echo MSVC init failed
    exit /b 1
)
cargo build -p vox-cli --release --features gpu,mens-candle-cuda
endlocal"#,
            "batch_script",
        ),
        (
            "Write a PowerShell script to generate and mix the Mens training corpus",
            r#"$RepoRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $RepoRoot

Write-Host "Generating synthetic corpus..." -ForegroundColor Cyan
& "$RepoRoot\target\release\vox.exe" mens corpus generate --output mens/data/synthetic.jsonl

Write-Host "Mixing corpus..." -ForegroundColor Cyan
& "$RepoRoot\target\release\vox.exe" mens corpus mix --config mens/config/mix.yaml

Write-Host "Corpus ready at target/dogfood/train_mixed.jsonl" -ForegroundColor Green"#,
            "powershell_script",
        ),
        (
            "Write a shell command to run all Vox workspace tests",
            "cargo test --workspace",
            "shell_command",
        ),
        (
            "Write a command to check a specific Vox crate compiles",
            "cargo check -p vox-corpus",
            "shell_command",
        ),
        (
            "Write a PowerShell script to start QLoRA training detached with persistent logging",
            r#"$RunDir = "mens\runs\qwen35_qlora"
New-Item -ItemType Directory -Force -Path $RunDir | Out-Null
New-Item -ItemType Directory -Force -Path "target\dogfood" | Out-Null

Start-Process -FilePath "launch_train.bat" -RedirectStandardOutput "$RunDir\train_run.log" -RedirectStandardError "$RunDir\train_err.log" -NoNewWindow
Write-Host "Training started. Monitor with: Get-Content $RunDir\telemetry.jsonl -Wait -Tail 5""#,
            "powershell_script",
        ),
        (
            "Write a command to push local Git commits to main",
            "git add -A && git commit -m \"update\" && git push origin main",
            "shell_command",
        ),
        (
            "Write a Cargo command to build the Vox release binary with GPU features",
            "cargo build -p vox-cli --release --features gpu",
            "shell_command",
        ),
        (
            "Write a PowerShell command to tail the training log file",
            "Get-Content C:\\Users\\Owner\\vox\\mens\\runs\\qwen35_qlora\\telemetry.jsonl -Wait -Tail 10",
            "shell_command",
        ),
        (
            "Write a command to check line endings for cross-platform integrity",
            "cargo run -p vox-cli -- ci line-endings",
            "shell_command",
        ),
        (
            "Write a .vox automation script to format dirty Rust files in chunks without overflowing command-line limits",
            "fn chunk_size() to int {\n    return 50\n}\n\nfn format_batch(batch_count: int) to Result[str, FmtError] {\n    if batch_count <= 0 {\n        return Ok(\"clean: 0 files formatted\")\n    }\n    return Ok(\"formatted batches\")\n}\n\nfn main() to Unit {\n    let res = format_batch(chunk_size())\n    match res {\n        Ok(msg) -> print(msg)\n        Error(e) -> print(\"error: \" + e)\n    }\n}",
            "vox_script",
        ),
        (
            "Write a .vox script that validates a corpus file count and exits with status diagnostics",
            "type CorpusCheck {\n    pairs: int\n    min_required: int\n}\n\nfn verify_corpus(check: CorpusCheck) to Result[str, GateError] {\n    if check.pairs < check.min_required {\n        return Error(\"insufficient pairs below threshold\")\n    }\n    return Ok(\"corpus gate passed\")\n}\n\nfn main() to Unit {\n    let gate = CorpusCheck { pairs: 1500, min_required: 1000 }\n    match verify_corpus(gate) {\n        Ok(msg) -> print(msg)\n        Error(reason) -> print(\"gate failed: \" + reason)\n    }\n}",
            "vox_script",
        ),
        (
            "Write a .vox script with an actor worker and workflow for durable CI pre-push verification",
            "actor GateRunner {\n    on run_gate(step: str) to Result[str, GateError] {\n        if step == \"\" {\n            return Error(\"empty gate step\")\n        }\n        return Ok(\"passed: \" + step)\n    }\n}\n\nworkflow pre_push_pipeline(branch: str) to Result[str, GateError] {\n    if branch == \"\" {\n        return Error(\"missing branch\")\n    }\n    return Ok(\"verified branch: \" + branch)\n}\n\nfn main() to Unit {\n    match pre_push_pipeline(\"main\") {\n        Ok(status) -> print(status)\n        Error(err) -> print(\"failed: \" + err)\n    }\n}",
            "vox_script",
        ),
        (
            "Write a .vox script that defines an MCP tool and HTTP health route for local agent orchestration",
            "tool workspace_health(target: str) to Result[str, HealthError] {\n    if target == \"\" {\n        return Error(\"target required\")\n    }\n    return Ok(\"healthy: \" + target)\n}\n\nhttp get \"/health\" to str {\n    return \"ok\"\n}\n\nfn main() to Unit {\n    match workspace_health(\"vox-corpus\") {\n        Ok(msg) -> print(msg)\n        Error(err) -> print(err)\n    }\n}",
            "vox_script",
        ),
    ];

    for (prompt, script, category) in scripts {
        let response = json!({
            "script": script,
            "language": if category.contains("vox") { "vox" }
                       else if category.contains("powershell") { "powershell" }
                       else if category.contains("batch") { "batch" }
                       else { "shell" },
        });
        emit_line(out, prompt, &response, category, "script_trace")?;
        count += 1;

        if *category == "vox_script" {
            let line = json!({
                "prompt": prompt,
                "response": format!("```vox\n{script}\n```"),
                "messages": [
                    {"role": "user", "content": prompt},
                    {"role": "assistant", "content": format!("```vox\n{script}\n```")}
                ],
                "category": "vox_script_writer",
                "format": "vox_organic",
                "schema_version": "vox_dogfood_v1",
            });
            writeln!(out, "{}", serde_json::to_string(&line)?)?;
            count += 1;
        }
    }

    Ok(count)
}

