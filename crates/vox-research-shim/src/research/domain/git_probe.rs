// vox-arch-check: allow git-exec

use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitProbeOutcome {
    pub passed: bool,
    pub head_sha: String,
    pub stdout: String,
    pub stderr: String,
}

pub async fn execute_shallow_clone_and_probe(
    repo_url: &str,
    probe_command: &str,
    args: &[&str],
    timeout_secs: u64,
) -> anyhow::Result<GitProbeOutcome> {
    if !repo_url.starts_with("https://")
        && !repo_url.starts_with("git://")
        && !repo_url.starts_with("file://")
    {
        anyhow::bail!("Invalid git repository URL scheme: {repo_url}");
    }

    let temp_dir_guard = tempfile::Builder::new()
        .prefix("vox-git-probe-")
        .tempdir()?;
    let clone_dest = temp_dir_guard.path().join("repo");

    let mut clone_cmd = Command::new("git");
    clone_cmd.kill_on_drop(true);
    clone_cmd.env("GIT_TERMINAL_PROMPT", "0");
    clone_cmd.env("GIT_ASKPASS", "");
    clone_cmd
        .args(["clone", "--depth", "1", "--", repo_url])
        .arg(&clone_dest);

    let clone_res =
        tokio::time::timeout(Duration::from_secs(timeout_secs), clone_cmd.output()).await;

    let clone_output = match clone_res {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => {
            return Ok(GitProbeOutcome {
                passed: false,
                head_sha: String::new(),
                stdout: String::new(),
                stderr: err.to_string(),
            });
        }
        Err(_) => anyhow::bail!("Git clone timed out after {timeout_secs}s"),
    };

    if !clone_output.status.success() {
        return Ok(GitProbeOutcome {
            passed: false,
            head_sha: String::new(),
            stdout: String::from_utf8_lossy(&clone_output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&clone_output.stderr).to_string(),
        });
    }

    let mut rev_parse_cmd = Command::new("git");
    rev_parse_cmd.kill_on_drop(true);
    rev_parse_cmd.current_dir(&clone_dest);
    rev_parse_cmd.args(["rev-parse", "HEAD"]);

    let rev_parse_res =
        tokio::time::timeout(Duration::from_secs(timeout_secs), rev_parse_cmd.output()).await;

    let rev_parse_output = match rev_parse_res {
        Ok(Ok(out)) => out,
        Ok(Err(err)) => anyhow::bail!("Failed to execute git rev-parse: {err}"),
        Err(_) => anyhow::bail!("Git rev-parse timed out"),
    };

    let head_sha = String::from_utf8_lossy(&rev_parse_output.stdout)
        .trim()
        .to_string();

    let mut probe = Command::new(probe_command);
    probe.kill_on_drop(true);
    probe.current_dir(&clone_dest).args(args);

    let probe_res = tokio::time::timeout(Duration::from_secs(timeout_secs), probe.output()).await;

    let probe_output = match probe_res {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => {
            return Ok(GitProbeOutcome {
                passed: false,
                head_sha,
                stdout: String::new(),
                stderr: err.to_string(),
            });
        }
        Err(_) => anyhow::bail!("Probe command '{probe_command}' timed out after {timeout_secs}s"),
    };

    Ok(GitProbeOutcome {
        passed: probe_output.status.success(),
        head_sha,
        stdout: String::from_utf8_lossy(&probe_output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&probe_output.stderr).to_string(),
    })
}
