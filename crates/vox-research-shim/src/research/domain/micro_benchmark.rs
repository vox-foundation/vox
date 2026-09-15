use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MicroBenchmarkReport {
    pub passed: bool,
    pub iterations: usize,
    pub median_ns: u64,
    pub mean_ns: u64,
    pub stderr: String,
}

pub fn scaffold_rust_benchmark_source(inner_statement: &str, iterations: usize) -> String {
    format!(
        r#"
fn main() {{
    // Warmup cache & avoid cold-start timer artifacts
    for _ in 0..10 {{
        let res = std::hint::black_box({{
            {inner_statement}
        }});
        std::hint::black_box(res);
    }}

    let mut timings = Vec::with_capacity({iterations});
    for _ in 0..{iterations} {{
        let start = std::time::Instant::now();
        let res = std::hint::black_box({{
            {inner_statement}
        }});
        std::hint::black_box(res);
        let elapsed = start.elapsed().as_nanos() as u64;
        timings.push(elapsed);
    }}
    timings.sort_unstable();
    let median = timings[timings.len() / 2];
    let sum: u64 = timings.iter().sum();
    let mean = sum / (timings.len() as u64).max(1);
    println!("BENCHMARK_RESULT:{{}}:{{}}", median, mean);
}}
"#
    )
}

pub async fn run_rust_micro_benchmark(
    inner_statement: &str,
    iterations: usize,
    timeout_ms: u64,
) -> anyhow::Result<MicroBenchmarkReport> {
    let temp_dir_guard = tempfile::Builder::new().prefix("vox-bench-").tempdir()?;
    let temp_dir = temp_dir_guard.path();
    let bin_name = if cfg!(windows) {
        "bench_bin.exe"
    } else {
        "bench_bin"
    };
    let bin_path = temp_dir.join(bin_name);

    let source = scaffold_rust_benchmark_source(inner_statement, iterations);

    let mut compile_cmd = Command::new("rustc");
    compile_cmd
        .kill_on_drop(true)
        .arg("-O")
        .arg("-o")
        .arg(&bin_path)
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = compile_cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(source.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin);
    }

    let compile_output =
        match tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait_with_output())
            .await
        {
            Ok(res) => res?,
            Err(_) => {
                return Ok(MicroBenchmarkReport {
                    passed: false,
                    iterations: 0,
                    median_ns: 0,
                    mean_ns: 0,
                    stderr: format!("Benchmark compilation timed out after {timeout_ms}ms"),
                });
            }
        };

    if !compile_output.status.success() {
        return Ok(MicroBenchmarkReport {
            passed: false,
            iterations: 0,
            median_ns: 0,
            mean_ns: 0,
            stderr: String::from_utf8_lossy(&compile_output.stderr).to_string(),
        });
    }

    let mut exec_cmd = Command::new(&bin_path);
    exec_cmd.kill_on_drop(true);
    exec_cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let exec_child = exec_cmd.spawn()?;
    let run_output = match tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        exec_child.wait_with_output(),
    )
    .await
    {
        Ok(res) => res?,
        Err(_) => {
            return Ok(MicroBenchmarkReport {
                passed: false,
                iterations: 0,
                median_ns: 0,
                mean_ns: 0,
                stderr: format!("Benchmark execution timed out after {timeout_ms}ms"),
            });
        }
    };

    let stdout_str = String::from_utf8_lossy(&run_output.stdout);
    for line in stdout_str.lines() {
        if let Some(rest) = line.strip_prefix("BENCHMARK_RESULT:") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 {
                let median: u64 = parts[0].parse().unwrap_or(0);
                let mean: u64 = parts[1].parse().unwrap_or(0);
                return Ok(MicroBenchmarkReport {
                    passed: true,
                    iterations,
                    median_ns: median,
                    mean_ns: mean,
                    stderr: String::new(),
                });
            }
        }
    }

    Ok(MicroBenchmarkReport {
        passed: false,
        iterations: 0,
        median_ns: 0,
        mean_ns: 0,
        stderr: "Failed to parse benchmark stdout".to_string(),
    })
}
