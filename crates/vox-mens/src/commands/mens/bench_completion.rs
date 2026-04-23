use anyhow::Result;
use reqwest::Client;
use std::time::Instant;

/// Benchmark FIM completion server latency.
pub async fn run_bench(url: &str, count: usize, warmups: usize) -> Result<()> {
    let client = Client::builder().pool_max_idle_per_host(4).build()?;

    let payload = serde_json::json!({
        "prompt": "<|fim_prefix|>fn greet(name: str) -> str {\n    <|fim_suffix|>\n}<|fim_middle|>",
        "max_tokens": 10,
        "temperature": 0.0,
        "n": 1,
        "language": "vox"
    });

    println!("Warming up server with {} requests...", warmups);
    for _ in 0..warmups {
        let _ = client.post(url).json(&payload).send().await?;
    }

    println!("Benchmarking server with {} requests...", count);
    let mut times = Vec::with_capacity(count);

    for i in 0..count {
        let t0 = Instant::now();
        let resp = client.post(url).json(&payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Server returned error {status}: {body}");
        }
        let latency = t0.elapsed().as_millis() as u64;
        times.push(latency);
        println!("Request {}: {} ms", i + 1, latency);
    }

    times.sort_unstable();
    let min = times[0];
    let max = times[times.len() - 1];
    let p50 = times[times.len() / 2];
    let p95 = times[(times.len() as f64 * 0.95) as usize];

    println!("\nLatency summary ({} requests):", count);
    println!("  Min: {} ms", min);
    println!("  P50: {} ms", p50);
    println!("  P95: {} ms", p95);
    println!("  Max: {} ms", max);

    if p95 > 50 {
        println!("\nWARNING: P95 latency ({} ms) exceeds 50 ms target.", p95);
    } else {
        println!("\nSUCCESS: P95 latency ({} ms) within 50 ms target.", p95);
    }

    // crate::benchmark_telemetry::record_opt(
    //     "populi_bench_completion",
    //     Some(p95 as f64),
    //     Some(details),
    // )
    // .await;

    Ok(())
}
