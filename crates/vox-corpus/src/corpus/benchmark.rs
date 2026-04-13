use std::path::Path;
use std::io::{BufRead, BufReader, Write};
use anyhow::Context;
use rand::seq::SliceRandom;

pub fn produce_benchmark(input: &Path, output: &Path, count: usize) -> anyhow::Result<usize> {
    let file = std::fs::File::open(input).with_context(|| format!("open {}", input.display()))?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        return Ok(0);
    }

    let mut rng = rand::thread_rng();
    lines.shuffle(&mut rng);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut f = std::fs::File::create(output).with_context(|| format!("create {}", output.display()))?;
    let limit = count.min(lines.len());
    
    for line in &lines[..limit] {
        writeln!(f, "{}", line)?;
    }

    Ok(limit)
}
