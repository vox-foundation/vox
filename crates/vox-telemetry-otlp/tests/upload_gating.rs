//! Guards the invariant: remote upload must be gated by is_remote_allowed().
//! When the master switch is off or consent is Denied/Unset, upload returns 0.
#![allow(unsafe_code)]

use std::sync::Mutex;
use tempfile::TempDir;
use vox_telemetry::config::{ConsentState, set_remote_consent};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_config<F: FnOnce()>(f: F) -> TempDir {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().expect("tempdir");
    #[cfg(windows)]
    unsafe {
        std::env::set_var("APPDATA", dir.path());
    }
    #[cfg(not(windows))]
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
    }
    f();
    dir
}

#[test]
fn upload_gated_when_consent_unset() {
    let _dir = with_temp_config(|| {
        unsafe {
            std::env::remove_var("VOX_TELEMETRY");
        }
        assert!(
            !vox_telemetry::config::is_remote_allowed(),
            "upload must be gated when consent is Unset"
        );
    });
}

#[test]
fn upload_allowed_when_consent_granted_and_master_on() {
    let _dir = with_temp_config(|| {
        unsafe {
            std::env::remove_var("VOX_TELEMETRY");
        }
        set_remote_consent(ConsentState::Granted);
        assert!(
            vox_telemetry::config::is_remote_allowed(),
            "upload must be allowed when consent is Granted and master is on"
        );
    });
}

#[test]
fn upload_gated_when_master_off_even_if_granted() {
    let _dir = with_temp_config(|| {
        unsafe {
            std::env::set_var("VOX_TELEMETRY", "off");
        }
        set_remote_consent(ConsentState::Granted);
        assert!(
            !vox_telemetry::config::is_remote_allowed(),
            "master off must override consent=Granted"
        );
        unsafe {
            std::env::remove_var("VOX_TELEMETRY");
        }
    });
}

#[test]
fn upload_gated_when_consent_denied() {
    let _dir = with_temp_config(|| {
        unsafe {
            std::env::remove_var("VOX_TELEMETRY");
        }
        set_remote_consent(ConsentState::Denied);
        assert!(
            !vox_telemetry::config::is_remote_allowed(),
            "upload must be gated when consent is Denied"
        );
    });
}

#[test]
fn merge_otlp_log_batch_combines_multiple_envelopes() {
    use vox_telemetry_otlp::otlp_json::{merge_otlp_log_batch, to_otlp_log};
    use vox_telemetry_otlp::redact::RedactedRecord;

    let r1 = to_otlp_log(
        &RedactedRecord {
            event_name: "vox.command".into(),
            attrs: serde_json::Map::new(),
        },
        "inst-123",
    );
    let r2 = to_otlp_log(
        &RedactedRecord {
            event_name: "vox.model_call".into(),
            attrs: serde_json::Map::new(),
        },
        "inst-123",
    );
    let merged = merge_otlp_log_batch(&[r1, r2]);
    let records = merged
        .pointer("/resourceLogs/0/scopeLogs/0/logRecords")
        .and_then(|v| v.as_array())
        .expect("logRecords array");
    assert_eq!(records.len(), 2);
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn upload_pending_otlp_batches_and_acks_when_granted() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use vox_telemetry_otlp::otlp_json::to_otlp_log;
    use vox_telemetry_otlp::redact::RedactedRecord;
    use vox_telemetry_otlp::upload::upload_pending_otlp_with_auth;

    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().expect("tempdir");
    #[cfg(windows)]
    unsafe {
        std::env::set_var("APPDATA", dir.path());
    }
    #[cfg(not(windows))]
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        std::env::remove_var("VOX_TELEMETRY");
    }
    set_remote_consent(ConsentState::Granted);

    let spool = dir.path().join("spool");
    let pending = spool.join("pending");
    std::fs::create_dir_all(&pending).expect("pending dir");
    for idx in 0..3 {
        let env = to_otlp_log(
            &RedactedRecord {
                event_name: format!("vox.command.{idx}"),
                attrs: serde_json::Map::new(),
            },
            "inst-batch",
        );
        std::fs::write(
            pending.join(format!("rec-{idx}.json")),
            serde_json::to_vec(&env).unwrap(),
        )
        .unwrap();
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local listener");
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.expect("read");
        let req_str = String::from_utf8_lossy(&buf[..n]).to_string();
        stream
            .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\n\r\n{}")
            .await
            .expect("write response");
        req_str
    });

    let endpoint = format!("http://{addr}/v1/logs");
    let uploaded = upload_pending_otlp_with_auth(&spool, &endpoint, Some("test-token"))
        .await
        .expect("upload_pending_otlp_with_auth");
    assert_eq!(uploaded, 3);
    assert_eq!(std::fs::read_dir(&pending).unwrap().count(), 0);

    let raw_req = server.await.expect("server join");
    assert!(raw_req.contains("Bearer test-token"));
    assert!(raw_req.contains("vox.command.0"));
    assert!(raw_req.contains("vox.command.2"));
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn batched_upload_merges_pending_files_and_deletes_on_200() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use vox_telemetry_otlp::otlp_json::to_otlp_log;
    use vox_telemetry_otlp::redact::RedactedRecord;
    use vox_telemetry_otlp::upload::upload_pending_otlp_with_auth;

    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cfg_dir = TempDir::new().expect("cfg tempdir");
    #[cfg(windows)]
    unsafe {
        std::env::set_var("APPDATA", cfg_dir.path());
    }
    #[cfg(not(windows))]
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", cfg_dir.path());
    }
    unsafe {
        std::env::remove_var("VOX_TELEMETRY");
    }
    set_remote_consent(ConsentState::Granted);

    let spool_dir = TempDir::new().expect("spool tempdir");
    let pending = spool_dir.path().join("pending");
    std::fs::create_dir_all(&pending).unwrap();

    for i in 0..3 {
        let mut attrs = serde_json::Map::new();
        attrs.insert("verb".into(), serde_json::Value::String("check".into()));
        let rec = RedactedRecord {
            event_name: "vox.command".into(),
            attrs,
        };
        let env = to_otlp_log(&rec, "install-test-123");
        std::fs::write(
            pending.join(format!("00{i}.json")),
            serde_json::to_vec(&env).unwrap(),
        )
        .unwrap();
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 16384];
        let n = stream.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]).to_string();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            .await
            .unwrap();
        req_str
    });

    let endpoint = format!("http://{addr}/v1/logs");
    let uploaded = upload_pending_otlp_with_auth(spool_dir.path(), &endpoint, Some("tok-123"))
        .await
        .expect("upload_pending_otlp_with_auth");
    assert_eq!(uploaded, 3);
    let remaining: Vec<_> = std::fs::read_dir(&pending)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        remaining.is_empty(),
        "all 3 files must be deleted after 200 OK"
    );

    let received = server.await.unwrap();
    assert!(received.contains("\"logRecords\""));
}
