//! Phase 3 UAT 1, automated: the REAL vox-plugin-webhook cdylib, installed the way
//! `vox plugin install --path` lays it out, is dlopen'd by the production poller that
//! `ServerState::new_full` starts for an `[orchestrator.webhook]` section. An HTTP POST
//! with `Authorization: Bearer <token>` becomes a hopper item with `IntakeSource::Webhook`;
//! the same POST without the header gets 401 and reaches nothing.
//!
//! Needs the built artifact (`cargo build -p vox-plugin-webhook`) next to this test
//! binary's profile dir; without it the test prints why and passes vacuously.
//! Network: 127.0.0.1 only.
//!
//! One test per binary on purpose: it sets process env (plugin root and ingress token)
//! before any runtime or thread exists, which is why `unsafe_code` is allowed here.
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::time::Duration;
use vox_orchestrator::hopper::IntakeSource;
use vox_orchestrator_mcp::{ServerState, load_config};

const TOKEN: &str = "uat-webhook-ingress-token";

fn dylib_name() -> &'static str {
    if cfg!(target_os = "windows") {
        // vox-arch-check: allow dynlib-ext
        "vox_plugin_webhook.dll"
    } else if cfg!(target_os = "macos") {
        // vox-arch-check: allow dynlib-ext
        "libvox_plugin_webhook.dylib"
    } else {
        // vox-arch-check: allow dynlib-ext
        "libvox_plugin_webhook.so"
    }
}

/// `<target>/<profile>/` — this binary lives in `<target>/<profile>/deps/`.
fn built_dylib() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.parent()?.join(dylib_name());
    p.is_file().then_some(p)
}

fn free_local_port() -> u16 {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    probe.local_addr().expect("probe addr").port()
}

#[test]
fn bearer_post_reaches_hopper_through_the_real_webhook_plugin() {
    let Some(dylib) = built_dylib() else {
        eprintln!(
            "SKIP webhook_plugin_e2e: {} not built next to this test binary; \
             run `cargo build -p vox-plugin-webhook` first",
            dylib_name()
        );
        return;
    };

    // Install layout of `vox plugin install --path`: <root>/<id>/<version>/{Plugin.toml, dylib}.
    let root = tempfile::tempdir().expect("plugins tempdir");
    let manifest =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vox-plugin-webhook/Plugin.toml");
    let raw = std::fs::read_to_string(&manifest).expect("read webhook Plugin.toml");
    let version: toml::Value = raw.parse().expect("Plugin.toml is TOML");
    let version = version["plugin"]["version"]
        .as_str()
        .expect("[plugin] version");
    let install = root.path().join("webhook").join(version);
    std::fs::create_dir_all(&install).expect("mkdir install dir");
    std::fs::write(install.join("Plugin.toml"), &raw).expect("copy manifest");
    std::fs::copy(&dylib, install.join(dylib_name())).expect("copy dylib");

    // SAFETY: single-test binary; no other thread exists yet.
    unsafe {
        std::env::set_var("VOX_PLUGINS_DIR", root.path());
        std::env::set_var("VOX_WEBHOOK_INGRESS_TOKEN", TOKEN);
    }

    let port = free_local_port();
    let section: vox_orchestrator::OrchestratorConfig = toml::from_str(&format!(
        "[webhook]\nbind_addr = \"127.0.0.1:{port}\"\npoll_interval_ms = 250\n"
    ))
    .expect("[orchestrator.webhook] section parses");
    let mut config = load_config();
    config.webhook = section.webhook;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async move {
        let state = ServerState::new_full(config);
        let hopper = state.orchestrator.hopper();
        let base = format!("http://127.0.0.1:{port}");
        let client = vox_http_client::client();
        // The poller dlopens the plugin on a blocking thread; wait for its listener.
        let load_diag = || match vox_plugin_host::cached_code_plugin("webhook") {
            Ok(_) => "plugin loads; listener never answered".to_string(),
            Err(e) => format!("plugin load failed: {e}"),
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
        loop {
            let up = client.get(format!("{base}/webhooks/health")).send().await;
            if up.is_ok_and(|r| r.status().as_u16() == 200) {
                break;
            }
            assert!(tokio::time::Instant::now() < deadline, "{}", load_diag());
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let webhook_items = || async {
            hopper
                .inbox()
                .await
                .into_iter()
                .filter(|i| i.source == IntakeSource::Webhook)
                .count()
        };
        let before = webhook_items().await;
        let post = || {
            client
                .post(format!("{base}/webhooks/github"))
                .header("x-github-event", "push")
                .body(r#"{"ref":"refs/heads/main"}"#)
        };

        let unauth = post().send().await.expect("unauthenticated POST");
        assert_eq!(
            unauth.status().as_u16(),
            401,
            "no bearer header must be refused"
        );

        let accepted = post().bearer_auth(TOKEN).send().await.expect("bearer POST");
        assert_eq!(accepted.status().as_u16(), 202);
        let body: serde_json::Value = accepted.json().await.expect("accept body");
        let event_id = body["event_id"].as_str().expect("event_id").to_string();

        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let item = loop {
            let found = hopper.inbox().await.into_iter().find(|i| {
                i.source == IntakeSource::Webhook && i.intent.contains(event_id.as_str())
            });
            if let Some(item) = found {
                break item;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "accepted event {event_id} never reached the hopper"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        assert!(item.intent.contains("git_push"), "{}", item.intent);
        assert!(item.intent.contains("github/push"), "{}", item.intent);
        assert!(!item.intent.contains("refs/heads/main"), "payload leaked");
        // Let one more poll pass: the 401 request must not have queued anything.
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(
            webhook_items().await,
            before + 1,
            "exactly one webhook item"
        );
        eprintln!(
            "webhook_plugin_e2e: hopper item {:?}: {}",
            item.source, item.intent
        );
    });
}
