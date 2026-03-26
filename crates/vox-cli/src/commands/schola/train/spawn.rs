//! Background training process spawn helpers.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Stdio;

/// Strip background / log-redirection flags so the child runs training in the foreground.
fn argv_for_background_child(args: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--background" {
            i += 1;
            continue;
        }
        if args[i].starts_with("--background=") {
            i += 1;
            continue;
        }
        if args[i] == "--log-dir" {
            i += 1;
            if i < args.len() {
                i += 1;
            }
            continue;
        }
        if args[i].starts_with("--log-dir=") {
            i += 1;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
    }
    out
}

/// Spawn `vox mens train` in a background process with stdout/stderr redirected to a log file.
/// Parent returns immediately so the IDE or agent tool does not hit wall-clock timeouts; tail the log
/// file to monitor progress. The child inherits the current environment (`VOX_*`, `RUST_LOG`, etc.).
pub fn spawn_train_with_log(log_dir: PathBuf) -> Result<()> {
    use owo_colors::OwoColorize;
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| anyhow::anyhow!("create log dir {}: {}", log_dir.display(), e))?;
    let timestamp = vox_corpus::training::timestamp_string();
    let log_path = log_dir.join(format!("train_{}.log", timestamp));
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| anyhow::anyhow!("create log file {}: {}", log_path.display(), e))?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let child_args = argv_for_background_child(args);
    let exe = std::env::current_exe().map_err(|e| anyhow::anyhow!("current exe: {}", e))?;

    let mut cmd = std::process::Command::new(&exe);
    for a in &child_args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(log_file.try_clone()?));
    cmd.stderr(Stdio::from(log_file));

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);
    }

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn training process: {}", e))?;
    let pid = child.id();

    eprintln!(
        "{} Training started in background. PID: {}. Log: {}",
        "✓".green(),
        pid,
        log_path.display()
    );
    eprintln!(
        "  Tail with: tail -f {}  (or Get-Content {} -Wait)",
        log_path.display(),
        log_path.display()
    );
    Ok(())
}
