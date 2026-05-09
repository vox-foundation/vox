use axum::{Router, routing::get};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use vox_share::auth::AuthMode;
use vox_share::proxy::{ProxyConfig, build_app};

#[test]
fn auth_mode_token_decorate_url() {
    let mode = AuthMode::UrlToken("abc123".to_string());
    let url = mode.decorate_url("https://test.trycloudflare.com");
    assert_eq!(url, "https://test.trycloudflare.com?vox_share_token=abc123");
}

#[test]
fn auth_mode_none_decorate_url_unchanged() {
    let mode = AuthMode::None;
    let url = mode.decorate_url("https://test.trycloudflare.com");
    assert_eq!(url, "https://test.trycloudflare.com");
}

#[test]
fn auth_mode_basic_decorate_url_unchanged() {
    let mode = AuthMode::Basic {
        user: "u".into(),
        pass: "p".into(),
    };
    let url = mode.decorate_url("https://test.trycloudflare.com");
    assert_eq!(url, "https://test.trycloudflare.com");
}

#[test]
fn auth_mode_token_decorate_url_with_existing_query() {
    let mode = AuthMode::UrlToken("abc123".to_string());
    let url = mode.decorate_url("https://test.trycloudflare.com?foo=bar");
    assert_eq!(
        url,
        "https://test.trycloudflare.com?foo=bar&vox_share_token=abc123"
    );
}

#[test]
fn auth_mode_parse_none() {
    let mode: AuthMode = "none".parse().unwrap();
    assert_eq!(mode, AuthMode::None);
}

#[test]
fn auth_mode_parse_basic() {
    let mode: AuthMode = "basic:alice:secret".parse().unwrap();
    assert_eq!(
        mode,
        AuthMode::Basic {
            user: "alice".into(),
            pass: "secret".into()
        }
    );
}

#[test]
fn auth_mode_parse_invalid() {
    let err = "foobar".parse::<AuthMode>().unwrap_err();
    assert!(err.contains("unknown auth mode"));
}

#[test]
fn auth_mode_parse_basic_missing_pass() {
    let err = "basic:useronly".parse::<AuthMode>().unwrap_err();
    assert!(err.contains("basic auth format"));
}

#[test]
fn auth_mode_random_token_is_16_chars() {
    let mode = AuthMode::random_token();
    if let AuthMode::UrlToken(token) = mode {
        assert_eq!(token.len(), 16);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    } else {
        panic!("expected UrlToken");
    }
}

#[tokio::test]
async fn proxy_with_token_rejects_missing_token() {
    let upstream = Router::new().route("/", get(|| async { "secret" }));
    let ul = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_port = ul.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(ul, upstream).await.unwrap();
    });

    let token = "deadbeefdeadbeef".to_string();
    let cfg = ProxyConfig {
        upstream_addr: SocketAddr::from(([127, 0, 0, 1], up_port)),
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        auth_mode: AuthMode::UrlToken(token.clone()),
    };
    let pl = TcpListener::bind(cfg.bind_addr).await.unwrap();
    let proxy_port = pl.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(pl, build_app(cfg)).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    // Without token → 401
    let resp = client
        .get(format!("http://127.0.0.1:{}/", proxy_port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With token in query param → 200
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/?vox_share_token={}",
            proxy_port, token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn proxy_with_token_accepts_cookie() {
    let upstream = Router::new().route("/", get(|| async { "secret" }));
    let ul = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_port = ul.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(ul, upstream).await.unwrap();
    });

    let token = "cafebabecafebabe".to_string();
    let cfg = ProxyConfig {
        upstream_addr: SocketAddr::from(([127, 0, 0, 1], up_port)),
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        auth_mode: AuthMode::UrlToken(token.clone()),
    };
    let pl = TcpListener::bind(cfg.bind_addr).await.unwrap();
    let proxy_port = pl.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(pl, build_app(cfg)).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    // With token in Cookie → 200
    let resp = client
        .get(format!("http://127.0.0.1:{}/", proxy_port))
        .header("Cookie", format!("vox_share_token={}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn proxy_with_none_auth_allows_all() {
    let upstream = Router::new().route("/", get(|| async { "public" }));
    let ul = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_port = ul.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(ul, upstream).await.unwrap();
    });

    let cfg = ProxyConfig {
        upstream_addr: SocketAddr::from(([127, 0, 0, 1], up_port)),
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        auth_mode: AuthMode::None,
    };
    let pl = TcpListener::bind(cfg.bind_addr).await.unwrap();
    let proxy_port = pl.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(pl, build_app(cfg)).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/", proxy_port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn proxy_with_basic_auth_rejects_missing_creds() {
    let upstream = Router::new().route("/", get(|| async { "secret" }));
    let ul = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_port = ul.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(ul, upstream).await.unwrap();
    });

    let cfg = ProxyConfig {
        upstream_addr: SocketAddr::from(([127, 0, 0, 1], up_port)),
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        auth_mode: AuthMode::Basic {
            user: "alice".into(),
            pass: "hunter2".into(),
        },
    };
    let pl = TcpListener::bind(cfg.bind_addr).await.unwrap();
    let proxy_port = pl.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(pl, build_app(cfg)).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    // Without credentials → 401
    let resp = client
        .get(format!("http://127.0.0.1:{}/", proxy_port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With correct credentials → 200
    let resp = client
        .get(format!("http://127.0.0.1:{}/", proxy_port))
        .basic_auth("alice", Some("hunter2"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
