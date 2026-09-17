use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

fn pcm_from_le_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

/// Uses the `ffmpeg` CLI (if installed) to extract mono 16kHz f32le audio
/// from a video container. This runs as a subprocess.
pub fn extract_audio_ffmpeg(path: &Path) -> Result<Vec<f32>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-v")
        .arg("error")
        .arg("-nostdin")
        .arg("-i")
        .arg(path)
        .arg("-vn") // no video
        .arg("-ac")
        .arg("1") // mono
        .arg("-ar")
        .arg("16000") // 16kHz
        .arg("-f")
        .arg("f32le") // 32-bit float little-endian
        .arg("-"); // output to stdout

    let output = cmd
        .output()
        .context("Failed to launch ffmpeg subprocess. Is ffmpeg in PATH?")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg failed with status {}: {}", output.status, stderr);
    }

    // Interpret raw bytes as little-endian f32 samples.
    Ok(pcm_from_le_bytes(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::pcm_from_le_bytes;

    #[test]
    fn decodes_complete_f32_samples_and_ignores_trailing_bytes() {
        let mut bytes = 1.5_f32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(-2.25_f32).to_le_bytes());
        bytes.push(0xff);

        assert_eq!(pcm_from_le_bytes(&bytes), vec![1.5, -2.25]);
    }
}
