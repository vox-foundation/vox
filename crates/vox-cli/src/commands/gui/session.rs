use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSession {
    pub schema_version: u32,
    pub pid: u32,
    pub port: u16,
    pub token_path: PathBuf,
    pub store_path: PathBuf,
    pub show: bool,
}

#[derive(Debug)]
pub struct DriveSessionError {
    pub exit_code: i32,
    pub message: String,
}

impl std::fmt::Display for DriveSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DriveSessionError {}

pub fn vox_home() -> PathBuf {
    if let Ok(h) = std::env::var("VOX_HOME") {
        return PathBuf::from(h);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vox")
}

pub fn session_path() -> PathBuf {
    vox_home().join("run/gui-drive.json")
}

pub fn token_path() -> PathBuf {
    vox_home().join("run/gui-drive.token")
}

pub fn lock_path() -> PathBuf {
    vox_home().join("run/gui-drive.lock")
}

pub fn generate_token() -> String {
    let mut buf = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn write_token(token: &str) -> Result<PathBuf, DriveSessionError> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| DriveSessionError {
            exit_code: 1,
            message: e.to_string(),
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&path).map_err(|e| DriveSessionError {
        exit_code: 1,
        message: e.to_string(),
    })?;
    f.write_all(token.as_bytes())
        .map_err(|e| DriveSessionError {
            exit_code: 1,
            message: e.to_string(),
        })?;
    Ok(path)
}

pub fn read_token() -> Result<String, DriveSessionError> {
    let path = token_path();
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW);
    }
    let mut f = opts.open(&path).map_err(|_| DriveSessionError {
        exit_code: 1,
        message: "no drive session; run vox gui drive start".into(),
    })?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| DriveSessionError {
        exit_code: 1,
        message: e.to_string(),
    })?;
    Ok(s.trim().to_string())
}

pub fn write_session(session: &DriveSession) -> Result<(), DriveSessionError> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| DriveSessionError {
            exit_code: 1,
            message: e.to_string(),
        })?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(session).map_err(|e| DriveSessionError {
        exit_code: 1,
        message: e.to_string(),
    })?;
    fs::write(&tmp, json).map_err(|e| DriveSessionError {
        exit_code: 1,
        message: e.to_string(),
    })?;
    fs::rename(&tmp, &path).map_err(|e| DriveSessionError {
        exit_code: 1,
        message: e.to_string(),
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn load_session() -> Result<DriveSession, DriveSessionError> {
    let path = session_path();
    let bytes = fs::read(&path).map_err(|_| DriveSessionError {
        exit_code: 1,
        message: "no drive session; run vox gui drive start".into(),
    })?;
    serde_json::from_slice(&bytes).map_err(|e| DriveSessionError {
        exit_code: 1,
        message: e.to_string(),
    })
}

/// Bearer ping — primary liveness. Does not use pid.
///
/// `/v1/ping` is authenticated and does **not** wait on the webview handler,
/// so start() can probe while AxisDriveHost is still mounting.
pub fn ping_drive(port: u16, token: &str) -> bool {
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
    if !http_status_ok(&out) || !out.contains("vox-gui-drive") {
        return false;
    }
    let Ok(mut stream) = std::net::TcpStream::connect(&addr) else {
        return false;
    };
    let auth = format!(
        "GET /v1/ping HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    if std::io::Write::write_all(&mut stream, auth.as_bytes()).is_err() {
        return false;
    }
    let mut out = String::new();
    let _ = std::io::Read::read_to_string(&mut stream, &mut out);
    http_status_ok(&out) && out.contains("vox-gui-drive")
}

fn http_status_ok(raw: &str) -> bool {
    raw.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        == Some("200")
}

pub fn acquire_lock() -> Result<fs::File, DriveSessionError> {
    let path = lock_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| DriveSessionError {
            exit_code: 1,
            message: e.to_string(),
        })?;
    }
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(false);
    // Windows: share_mode(0) = exclusive while this handle is held (flock parity).
    // A second start fails to open rather than silently sharing the lock file.
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0);
    }
    let file = match opts.open(&path) {
        Ok(f) => f,
        Err(e) => {
            #[cfg(windows)]
            {
                // ERROR_SHARING_VIOLATION — another Axis Drive start holds the lock.
                if e.raw_os_error() == Some(32) {
                    return Err(DriveSessionError {
                        exit_code: 2,
                        message: format!("drive lock exists path={}", path.display()),
                    });
                }
            }
            return Err(DriveSessionError {
                exit_code: 1,
                message: e.to_string(),
            });
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        // flock is the session mutex; LOCK_NB so a second start fails closed.
        #[allow(unsafe_code)]
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            return Err(DriveSessionError {
                exit_code: 2,
                message: format!("drive lock exists path={}", path.display()),
            });
        }
    }
    Ok(file)
}

pub fn release_lock() {
    let _ = fs::remove_file(lock_path());
}

pub fn preflight_start() -> Result<(), DriveSessionError> {
    let Ok(existing) = load_session() else {
        // Do not unlink the lock file: another start may hold flock on that inode.
        return Ok(());
    };
    let token = read_token().unwrap_or_default();
    if !token.is_empty() && ping_drive(existing.port, &token) {
        return Err(DriveSessionError {
            exit_code: 2,
            message: format!(
                "drive session already running port={} path={}",
                existing.port,
                session_path().display()
            ),
        });
    }
    let _ = fs::remove_file(session_path());
    let _ = fs::remove_file(token_path());
    Ok(())
}

pub fn clear_session_files() {
    let _ = fs::remove_file(session_path());
    let _ = fs::remove_file(token_path());
    release_lock();
}

pub fn sanitize_drive_profile(profile: &str) -> Result<&str, DriveSessionError> {
    if profile.is_empty() || profile.len() > 64 {
        return Err(DriveSessionError {
            exit_code: 1,
            message: format!("invalid drive profile {profile:?}"),
        });
    }
    if profile == "." || profile == ".." || profile.contains("..") {
        return Err(DriveSessionError {
            exit_code: 1,
            message: format!("invalid drive profile {profile:?}"),
        });
    }
    if !profile
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(DriveSessionError {
            exit_code: 1,
            message: format!("invalid drive profile {profile:?}"),
        });
    }
    Ok(profile)
}

pub fn drive_store_root(profile: &str) -> Result<PathBuf, DriveSessionError> {
    let profile = sanitize_drive_profile(profile)?;
    Ok(vox_home().join("gui-drive").join(profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_home() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[allow(unsafe_code)] // edition 2024: set_var/remove_var are unsafe
    fn with_vox_home<T>(home: &std::path::Path, f: impl FnOnce() -> T) -> T {
        let _guard = HOME_LOCK.lock().expect("vox home lock");
        unsafe { std::env::set_var("VOX_HOME", home) };
        let out = f();
        unsafe { std::env::remove_var("VOX_HOME") };
        out
    }

    #[test]
    fn start_refuses_when_ping_succeeds() {
        let home = isolated_home();
        with_vox_home(home.path(), || {
            let h = session_test_bind();
            let token = "secret-token";
            write_token(token).unwrap();
            write_session(&DriveSession {
                schema_version: 1,
                pid: 1,
                port: h.port(),
                token_path: token_path(),
                store_path: home.path().join("store.db"),
                show: false,
            })
            .unwrap();
            let err = preflight_start().expect_err("must refuse");
            assert_eq!(err.exit_code, 2);
            h.shutdown();
        });
    }

    #[test]
    fn start_replaces_dead_session() {
        let home = isolated_home();
        with_vox_home(home.path(), || {
            write_session(&DriveSession {
                schema_version: 1,
                pid: 1,
                port: 1,
                token_path: token_path(),
                store_path: home.path().join("d"),
                show: false,
            })
            .unwrap();
            preflight_start().expect("dead ping is replaceable");
        });
    }

    #[test]
    fn token_file_is_0600_and_not_in_session_json_value() {
        let home = isolated_home();
        with_vox_home(home.path(), || {
            let token = generate_token();
            let path = write_token(&token).unwrap();
            let meta = fs::metadata(&path).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(meta.permissions().mode() & 0o777, 0o600);
            }
            let session = DriveSession {
                schema_version: 1,
                pid: 1,
                port: 1,
                token_path: path,
                store_path: home.path().join("d"),
                show: false,
            };
            let json = serde_json::to_string(&session).unwrap();
            assert!(!json.contains(&token));
        });
    }

    #[test]
    fn preflight_without_session_does_not_unlink_lock() {
        let home = isolated_home();
        with_vox_home(home.path(), || {
            let lock = acquire_lock().unwrap();
            assert!(lock_path().exists());
            preflight_start().expect("no session is ok");
            assert!(
                lock_path().exists(),
                "preflight must not unlink a held flock"
            );
            drop(lock);
        });
    }

    #[test]
    fn sanitize_drive_profile_rejects_traversal() {
        assert!(sanitize_drive_profile("../../.ssh").is_err());
        assert!(sanitize_drive_profile("a/b").is_err());
        assert!(sanitize_drive_profile("..").is_err());
        assert!(sanitize_drive_profile("agent-1").is_ok());
        assert!(sanitize_drive_profile("default").is_ok());
    }
}

/// Test-only loopback health+state server so session tests do not import vox-gui.
#[cfg(test)]
pub(crate) struct TestBind {
    port: u16,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(test)]
impl TestBind {
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn shutdown(self) {
        use std::sync::atomic::Ordering;
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(format!("127.0.0.1:{}", self.port));
    }
}

#[cfg(test)]
pub(crate) fn session_test_bind() -> TestBind {
    use std::io::{Read, Write};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&shutdown);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if flag.load(Ordering::SeqCst) {
                break;
            }
            if let Ok(mut s) = stream {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                let raw = String::from_utf8_lossy(&buf);
                let body = if raw.starts_with("GET /health") {
                    r#"{"service":"vox-gui-drive","ready":true}"#
                } else if raw.contains("GET /v1/ping")
                    && raw.contains("Authorization: Bearer secret-token")
                {
                    r#"{"ok":true,"ready":true,"service":"vox-gui-drive"}"#
                } else {
                    r#"{"error":"unauthorized"}"#
                };
                let status = if raw.starts_with("GET /health")
                    || (raw.contains("GET /v1/ping") && raw.contains("Bearer secret-token"))
                {
                    "200"
                } else {
                    "401"
                };
                let _ = write!(
                    s,
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
            }
        }
    });
    TestBind { port, shutdown }
}
