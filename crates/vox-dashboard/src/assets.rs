//! Embedded SPA asset handler.
//! With feature `embedded-assets`: serves from compile-time bytes.
//! Without: falls back to disk at `VOX_DASHBOARD_ASSET_DIR`.
use axum::response::{IntoResponse, Response};
use axum::http::{StatusCode, header};
use axum::extract::Path;

#[cfg(feature = "embedded-assets")]
static DIST: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/dist");

pub async fn serve_asset(
    path: Option<Path<String>>,
    axum::extract::Extension(token): axum::extract::Extension<Option<String>>
) -> Response {
    let mut asset_path = path
        .map(|p| p.0)
        .unwrap_or_else(|| "index.html".into());
        
    if asset_path.is_empty() || asset_path.ends_with('/') {
        asset_path.push_str("index.html");
    }

    let csp = "default-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; frame-ancestors 'none'; object-src 'none'";

    let response_builder = axum::response::Response::builder()
        .header("Content-Security-Policy", csp)
        .header("X-Frame-Options", "DENY")
        .header("Referrer-Policy", "no-referrer");

    #[cfg(feature = "embedded-assets")]
    let file_result = {
        match DIST.get_file(&asset_path) {
            Some(file) => Some((mime_guess::from_path(&asset_path).first_or_octet_stream(), file.contents().to_vec())),
            None => {
                if let Some(index) = DIST.get_file("index.html") {
                    Some((mime_guess::from_path("index.html").first_or_octet_stream(), index.contents().to_vec()))
                } else {
                    None
                }
            }
        }
    };
    
    #[cfg(not(feature = "embedded-assets"))]
    let file_result = {
        let base_dir = std::env::var("VOX_DASHBOARD_ASSET_DIR")
            .unwrap_or_else(|_| "crates/vox-dashboard/dist".into());
        let path = std::path::Path::new(&base_dir).join(&asset_path);
        
        let file_path = if path.exists() && path.is_file() {
            path
        } else {
            std::path::Path::new(&base_dir).join("index.html")
        };
        
        std::fs::read(&file_path).ok().map(|contents| {
            (mime_guess::from_path(&file_path).first_or_octet_stream(), contents)
        })
    };

    match file_result {
        Some((mime_type, mut contents)) => {
            if mime_type.as_ref() == "text/html" {
                if let Some(t) = token {
                    let html = String::from_utf8_lossy(&contents);
                    let injected = html.replace(
                        "</head>",
                        &format!("<meta name=\"vox-bearer\" content=\"{}\">\n</head>", t)
                    );
                    contents = injected.into_bytes();
                }
            }
            response_builder
                .header(header::CONTENT_TYPE, mime_type.as_ref())
                .body(axum::body::Body::from(contents))
                .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response())
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}
