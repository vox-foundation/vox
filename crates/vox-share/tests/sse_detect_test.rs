use serde_json::json;
use vox_share::sse_detect::detect_sse_in_openapi;

#[test]
fn detects_sse_route_in_spec() {
    let spec = json!({
        "paths": {
            "/chat/stream": {
                "get": {
                    "responses": {
                        "200": {
                            "content": {
                                "text/event-stream": {}
                            }
                        }
                    }
                }
            }
        }
    });
    assert!(detect_sse_in_openapi(&spec));
}

#[test]
fn no_false_positive_for_json_only_spec() {
    let spec = json!({
        "paths": {
            "/api/data": {
                "get": {
                    "responses": {
                        "200": { "content": { "application/json": {} } }
                    }
                }
            }
        }
    });
    assert!(!detect_sse_in_openapi(&spec));
}

#[tokio::test]
async fn has_sse_routes_returns_false_on_no_server() {
    // Port 1 will refuse connections — should return false, not panic.
    let result = vox_share::sse_detect::has_sse_routes(1).await;
    assert!(!result, "should return false when upstream is unreachable");
}

#[tokio::test]
async fn has_sse_routes_detects_sse_from_live_server() {
    // Spawn a tiny Axum server that serves an OpenAPI spec with an SSE route.
    let spec = json!({
        "openapi": "3.0.0",
        "paths": {
            "/stream": {
                "get": {
                    "responses": {
                        "200": {
                            "content": { "text/event-stream": {} }
                        }
                    }
                }
            }
        }
    });
    let spec_str = spec.to_string();
    let app =
        axum::Router::new().route(
            "/openapi.json",
            axum::routing::get(move || {
                let s = spec_str.clone();
                async move {
                    axum::response::Json(serde_json::from_str::<serde_json::Value>(&s).unwrap())
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    assert!(vox_share::sse_detect::has_sse_routes(port).await);
}
