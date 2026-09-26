//! Local Oratio transcription worker (`vox oratio serve`): `POST /transcribe` on 127.0.0.1.

#[cfg(feature = "serve")]
use axum::{Json, Router, extract::Multipart, routing::post};
#[cfg(feature = "serve")]
use std::net::SocketAddr;

/// Serve `POST /transcribe` (multipart `file` of f32 LE PCM, optional `sample_rate` and
/// `language`) on `127.0.0.1:port`, transcribing through the Whisper backend selection.
#[cfg(feature = "serve")]
pub async fn run_serve_worker(port: u16) -> anyhow::Result<()> {
    tracing::info!("Starting local Oratio worker on port {}", port);

    let app = Router::new().route("/transcribe", post(transcribe_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(feature = "serve")]
async fn transcribe_handler(
    mut multipart: Multipart,
) -> Result<Json<crate::backends::asr_backend::AsrOutput>, axum::http::StatusCode> {
    let mut file_data = Vec::new();
    let mut sample_rate = 16000;
    let mut language = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
            file_data.extend_from_slice(&data);
        } else if name == "sample_rate" {
            let text = field
                .text()
                .await
                .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
            if let Ok(sr) = text.parse::<u32>() {
                sample_rate = sr;
            }
        } else if name == "language" {
            let text = field
                .text()
                .await
                .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
            if !text.is_empty() {
                language = Some(text);
            }
        }
    }

    if file_data.is_empty() {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let pcm = pcm_from_le_bytes(&file_data);

    let out = tokio::task::spawn_blocking(move || {
        crate::backend_dispatch::whisper_backend()?.transcribe_pcm(
            &pcm,
            sample_rate,
            language.as_deref(),
        )
    })
    .await
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(out))
}

/// Decode raw little-endian f32 bytes to PCM; a trailing partial chunk is ignored.
#[cfg(feature = "serve")]
fn pcm_from_le_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

#[cfg(all(test, feature = "serve"))]
mod tests {
    use super::*;

    #[test]
    fn pcm_from_le_bytes_roundtrip() {
        let mut bytes: Vec<u8> = [0.5f32, -0.25]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        bytes.extend_from_slice(&[1, 2, 3]);
        assert_eq!(pcm_from_le_bytes(&bytes), vec![0.5, -0.25]);
    }
}
