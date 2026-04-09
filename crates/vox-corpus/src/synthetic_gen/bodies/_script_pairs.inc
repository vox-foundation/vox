
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
    ];

    for (prompt, script, category) in scripts {
        let response = json!({
            "script": script,
            "language": if category.contains("powershell") { "powershell" }
                       else if category.contains("batch") { "batch" }
                       else { "shell" },
        });
        emit_line(out, prompt, &response, category, "script_trace")?;
        count += 1;
    }

    Ok(count)
}

