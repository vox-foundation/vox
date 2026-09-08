use crate::cli_args::{
    DriveArgs, DriveCmd, DriveHeadlessCmd, DriveSendArgs, DriveSetArgs, DriveStartArgs,
};
use anyhow::{Context, Result, bail};

use super::client;
use super::launch::{gui_command, spawn_gui_detached};
use super::session::{
    acquire_lock, clear_session_files, drive_store_root, generate_token, load_session, ping_drive,
    preflight_start, read_token, session_path, write_token,
};

pub async fn run(args: DriveArgs) -> Result<()> {
    match args.cmd {
        DriveCmd::Start(s) => start(s).await,
        DriveCmd::Stop => stop(),
        DriveCmd::Show => show(),
        DriveCmd::Set(s) => set(s),
        DriveCmd::Send(s) => send(s),
        DriveCmd::State => state(),
        DriveCmd::Wait(w) => client::wait_until(&w.until, &w.timeout),
        DriveCmd::Headless(h) => headless(h.cmd),
    }
}

fn drive_child_env(
    show: bool,
    store_root: &std::path::Path,
    token_path: &std::path::Path,
) -> Vec<(String, String)> {
    vec![
        ("VOX_GUI_DRIVE".into(), "1".into()),
        (
            "VOX_GUI_DRIVE_SHOW".into(),
            if show { "1" } else { "0" }.into(),
        ),
        (
            "VOX_GUI_DRIVE_SESSION_PATH".into(),
            session_path().display().to_string(),
        ),
        (
            "VOX_GUI_DRIVE_STORE_ROOT".into(),
            store_root.display().to_string(),
        ),
        (
            "VOX_GUI_DRIVE_TOKEN_PATH".into(),
            token_path.display().to_string(),
        ),
    ]
}

async fn start(args: DriveStartArgs) -> Result<()> {
    if let Err(e) = preflight_start() {
        std::process::exit(e.exit_code);
    }
    let _lock = acquire_lock().map_err(|e| {
        eprintln!("{}", e.message);
        std::process::exit(e.exit_code);
    });
    let token = generate_token();
    let token_path = write_token(&token).map_err(|e| anyhow::anyhow!(e.message))?;
    let profile = args.profile.as_deref().unwrap_or("default");
    let store_root = drive_store_root(profile).map_err(|e| anyhow::anyhow!(e.message))?;
    std::fs::create_dir_all(&store_root)?;
    let mut cmd = gui_command()?;
    cmd.arg("--drive");
    if args.show {
        cmd.arg("--show");
    }
    cmd.env("VOX_GUI_DRIVE_PROFILE", profile);
    for (k, v) in drive_child_env(args.show, &store_root, &token_path) {
        cmd.env(k, v);
    }
    let child = spawn_gui_detached(cmd)?;
    let pid = child.id();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if let Ok(session) = load_session() {
            if session.port != 0 && ping_drive(session.port, &token) {
                if health_ready(session.port) {
                    println!(
                        "started pid={pid} port={} session={}",
                        session.port,
                        session_path().display()
                    );
                    return Ok(());
                }
            }
        }
        if std::time::Instant::now() > deadline {
            let _ = nix_kill(pid);
            clear_session_files();
            bail!("drive listener not ready within 15s");
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn health_ready(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let Ok(mut stream) = std::net::TcpStream::connect(&addr) else {
        return false;
    };
    let req = "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if std::io::Write::write_all(&mut stream, req.as_bytes()).is_err() {
        return false;
    }
    let mut out = String::new();
    let _ = std::io::Read::read_to_string(&mut stream, &mut out);
    out.contains("\"ready\":true")
}

fn nix_kill(pid: u32) -> Result<()> {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .context("run taskkill")?;
        if !status.success() {
            bail!("taskkill failed for pid {pid}");
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let status = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .context("run kill")?;
        if !status.success() {
            bail!("kill failed for pid {pid}");
        }
        Ok(())
    }
}

fn stop() -> Result<()> {
    let session = load_session().map_err(|e| anyhow::anyhow!(e.message))?;
    let token = read_token().unwrap_or_default();
    if !token.is_empty() && ping_drive(session.port, &token) {
        let _ = nix_kill(session.pid);
    }
    clear_session_files();
    println!("stopped");
    Ok(())
}

fn show() -> Result<()> {
    let (status, body) = client::post("show", "{}")?;
    if status != 200 {
        bail!("show failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

fn set(args: DriveSetArgs) -> Result<()> {
    let mut map = serde_json::Map::new();
    for pair in &args.knob {
        let (k, v) = pair.split_once('=').context("knob must be key=value")?;
        map.insert(k.to_string(), parse_knob_value(v));
    }
    let body = serde_json::Value::Object(map).to_string();
    let (status, resp) = client::post("set", &body)?;
    println!("{resp}");
    if status >= 400 {
        bail!("set failed ({status})");
    }
    Ok(())
}

fn parse_knob_value(v: &str) -> serde_json::Value {
    if v == "true" {
        return serde_json::Value::Bool(true);
    }
    if v == "false" {
        return serde_json::Value::Bool(false);
    }
    serde_json::Value::String(v.to_string())
}

fn send(args: DriveSendArgs) -> Result<()> {
    let body = serde_json::json!({ "text": args.text }).to_string();
    let (status, resp) = client::post("send", &body)?;
    println!("{resp}");
    if status >= 400 {
        bail!("send failed ({status})");
    }
    Ok(())
}

fn state() -> Result<()> {
    let (status, resp) = client::post("state", "{}")?;
    println!("{resp}");
    if status >= 400 {
        bail!("state failed ({status})");
    }
    Ok(())
}

fn headless(cmd: DriveHeadlessCmd) -> Result<()> {
    let req = match &cmd {
        DriveHeadlessCmd::State => serde_json::json!({ "verb": "state" }),
        DriveHeadlessCmd::Set(s) => {
            let mut map = serde_json::Map::new();
            for pair in &s.knob {
                let (k, v) = pair.split_once('=').context("knob must be key=value")?;
                map.insert(k.to_string(), parse_knob_value(v));
            }
            serde_json::json!({ "verb": "set", "set": map })
        }
        DriveHeadlessCmd::Send(s) => serde_json::json!({ "verb": "send", "text": s.text }),
    };
    let mut command = gui_command()?;
    command.arg("--drive-headless");
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    let mut child = command.spawn().context("spawn --drive-headless")?;
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .context("stdin")?
        .write_all(req.to_string().as_bytes())?;
    let out = child.wait_with_output()?;
    print!("{}", String::from_utf8_lossy(&out.stdout));
    if !out.status.success() {
        bail!("headless failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn drive_start_parses_show_and_profile() {
        let args = DriveArgs::try_parse_from(["drive", "start", "--show", "--profile", "agent-1"])
            .expect("parse");
        match args.cmd {
            DriveCmd::Start(s) => {
                assert!(s.show);
                assert_eq!(s.profile.as_deref(), Some("agent-1"));
            }
            other => panic!("expected Start, got {other:?}"),
        }
    }

    #[test]
    fn drive_set_accepts_repeated_knob() {
        let args = DriveArgs::try_parse_from([
            "drive",
            "set",
            "--knob",
            "model_override=mens/e2e-smoke",
            "--knob",
            "execution=sync",
        ])
        .expect("parse");
        match args.cmd {
            DriveCmd::Set(s) => {
                assert_eq!(s.knob.len(), 2);
                assert_eq!(s.knob[0], "model_override=mens/e2e-smoke");
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn drive_send_requires_text() {
        assert!(DriveArgs::try_parse_from(["drive", "send"]).is_err());
    }

    #[test]
    fn drive_env_does_not_put_token_in_child_env() {
        let env = drive_child_env(
            false,
            std::path::Path::new("/tmp/gui-drive/agent-1"),
            std::path::Path::new("/tmp/run/gui-drive.token"),
        );
        assert!(env.iter().all(|(k, _)| k != "VOX_GUI_DRIVE_TOKEN"));
        assert!(
            env.iter()
                .any(|(k, v)| k == "VOX_GUI_DRIVE_TOKEN_PATH" && v.ends_with("gui-drive.token"))
        );
        assert!(
            env.iter()
                .any(|(k, v)| k == "VOX_GUI_DRIVE_STORE_ROOT" && v.ends_with("gui-drive/agent-1"))
        );
    }
}
