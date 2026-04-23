//! Embedded SPA asset handler.
//! With feature `embedded-assets`: serves from compile-time bytes.
//! Without: falls back to disk at `VOX_DASHBOARD_ASSET_DIR`.
use axum::response::{IntoResponse, Response};
use axum::http::{StatusCode, header};
use axum::extract::Path;

#[cfg(feature = "embedded-assets")]
static DIST: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/dist");

pub async fn serve_asset(path: Option<Path<String>>) -> Response {
    let mut asset_path = path
        .map(|p| p.0)
        .unwrap_or_else(|| "index.html".into());
        
    if asset_path.is_empty() || asset_path.ends_with('/') {
        asset_path.push_str("index.html");
    }

    #[cfg(feature = "embedded-assets")]
    {
        match DIST.get_file(&asset_path) {
            Some(file) => {
                let mime_type = mime_guess::from_path(&asset_path).first_or_octet_stream();
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, mime_type.as_ref())],
                    file.contents(),
                ).into_response()
            }
            None => {
                if let Some(index) = DIST.get_file("index.html") {
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "text/html")],
                        index.contents(),
                    ).into_response()
                } else {
                    (StatusCode::NOT_FOUND, "Not found").into_response()
                }
            }
        }
    }
    
    #[cfg(not(feature = "embedded-assets"))]
    {
        let base_dir = std::env::var("VOX_DASHBOARD_ASSET_DIR")
            .unwrap_or_else(|_| "crates/vox-dashboard/dist".into());
        let path = std::path::Path::new(&base_dir).join(&asset_path);
        
        let file_path = if path.exists() && path.is_file() {
            path
        } else {
            std::path::Path::new(&base_dir).join("index.html")
        };
        
        match std::fs::read(&file_path) {
            Ok(contents) => {
                let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, mime_type.as_ref())],
                    contents,
                ).into_response()
            }
            Err(_) => {
                (StatusCode::NOT_FOUND, "Not found").into_response()
            }
        }
    }
}
