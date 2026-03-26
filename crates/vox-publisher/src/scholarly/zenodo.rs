//! Minimal Zenodo REST client (deposit draft creation).

use std::time::Duration;

use async_trait::async_trait;

use super::error::{classify_scholarly_http, ScholarlyError};
use super::flags;
use super::ScholarlyRemoteStatus;
use super::ScholarlySubmissionReceipt;
use crate::publication::PublicationManifest;
use crate::zenodo_api_types::{ZenodoDeposition, ZenodoDepositionCreateBody};
use crate::zenodo_metadata;

const ZENODO_API_PRODUCTION: &str = "https://zenodo.org/api";
const ZENODO_API_SANDBOX: &str = "https://sandbox.zenodo.org/api";

#[must_use]
fn zenodo_http_max_attempts() -> u32 {
    std::env::var("VOX_ZENODO_HTTP_MAX_ATTEMPTS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|&n| (1..=10).contains(&n))
        .unwrap_or(3)
}

async fn sleep_before_zenodo_retry(err: &ScholarlyError, zero_based_attempt: u32) {
    let ms: u64 = match err {
        ScholarlyError::RateLimited {
            retry_after_secs: Some(s),
            ..
        } => (*s).saturating_mul(1000).max(500),
        _ => {
            let base = 350u64.saturating_mul(2u64.pow(zero_based_attempt));
            base.min(8_000)
        }
    };
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

#[derive(Debug, Clone)]
pub(super) struct ZenodoHttpClient {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl ZenodoHttpClient {
    pub(super) fn new(sandbox: bool, token: String) -> Result<Self, ScholarlyError> {
        let t = token.trim();
        if t.is_empty() {
            return Err(ScholarlyError::Config {
                message: "Zenodo access token is empty".into(),
            });
        }
        let base = std::env::var("VOX_ZENODO_API_BASE")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if sandbox {
                    ZENODO_API_SANDBOX.to_string()
                } else {
                    ZENODO_API_PRODUCTION.to_string()
                }
            });
        Ok(Self {
            base,
            token: t.to_string(),
            http: reqwest::Client::builder()
                .user_agent("vox-publisher/scholarly-zenodo")
                .build()
                .map_err(|e| ScholarlyError::Config {
                    message: format!("http client: {e}"),
                })?,
        })
    }

    fn url_depositions(&self) -> String {
        format!("{}/deposit/depositions", self.base.trim_end_matches('/'))
    }

    fn url_deposition(&self, id: &str) -> String {
        format!("{}/deposit/depositions/{id}", self.base.trim_end_matches('/'))
    }

    async fn create_deposition_draft_once(
        &self,
        body: &ZenodoDepositionCreateBody,
    ) -> Result<ZenodoDeposition, ScholarlyError> {
        let url = self.url_depositions();
        let resp = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(classify_scholarly_http(status, &text));
        }
        serde_json::from_str(&text).map_err(|e| ScholarlyError::Fatal {
            code: "zenodo_json_parse".into(),
            message: format!("invalid JSON from Zenodo: {e}; body={text}"),
        })
    }

    pub(super) async fn create_deposition_draft(
        &self,
        body: &ZenodoDepositionCreateBody,
    ) -> Result<ZenodoDeposition, ScholarlyError> {
        let max = zenodo_http_max_attempts();
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            match self.create_deposition_draft_once(body).await {
                Ok(v) => return Ok(v),
                Err(e) if e.retryable() && attempt < max => {
                    sleep_before_zenodo_retry(&e, attempt.saturating_sub(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn get_deposition_once(
        &self,
        deposition_id: &str,
    ) -> Result<ZenodoDeposition, ScholarlyError> {
        let url = self.url_deposition(deposition_id);
        let resp = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(classify_scholarly_http(status, &text));
        }
        serde_json::from_str(&text).map_err(|e| ScholarlyError::Fatal {
            code: "zenodo_json_parse".into(),
            message: format!("invalid JSON from Zenodo: {e}; body={text}"),
        })
    }

    pub(super) async fn get_deposition(
        &self,
        deposition_id: &str,
    ) -> Result<ZenodoDeposition, ScholarlyError> {
        let max = zenodo_http_max_attempts();
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            match self.get_deposition_once(deposition_id).await {
                Ok(v) => return Ok(v),
                Err(e) if e.retryable() && attempt < max => {
                    sleep_before_zenodo_retry(&e, attempt.saturating_sub(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn bucket_url_from_deposition(v: &ZenodoDeposition) -> Result<String, ScholarlyError> {
        v.links
            .as_ref()
            .and_then(|l| l.bucket.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string)
            .ok_or_else(|| ScholarlyError::Fatal {
                code: "zenodo_missing_bucket".into(),
                message: format!("deposit response missing links.bucket: {}", serde_json::to_string(v).unwrap_or_default()),
            })
    }

    async fn put_bucket_object_once(
        &self,
        bucket_url: &str,
        object_name: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<(), ScholarlyError> {
        let name = object_name.trim_start_matches('/');
        if name.is_empty() {
            return Err(ScholarlyError::Config {
                message: "Zenodo bucket object name must not be empty".into(),
            });
        }
        let url = format!(
            "{}/{}",
            bucket_url.trim_end_matches('/'),
            name
        );
        let resp = self
            .http
            .put(url)
            .header("Content-Type", content_type)
            .bearer_auth(&self.token)
            .body(data.to_vec())
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(classify_scholarly_http(status, &text));
        }
        Ok(())
    }

    pub(super) async fn put_bucket_object(
        &self,
        bucket_url: &str,
        object_name: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<(), ScholarlyError> {
        let max = zenodo_http_max_attempts();
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            match self
                .put_bucket_object_once(bucket_url, object_name, data, content_type)
                .await
            {
                Ok(()) => return Ok(()),
                Err(e) if e.retryable() && attempt < max => {
                    sleep_before_zenodo_retry(&e, attempt.saturating_sub(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn publish_deposition_once(
        &self,
        deposition_id: &str,
    ) -> Result<ZenodoDeposition, ScholarlyError> {
        let url = format!(
            "{}/deposit/depositions/{deposition_id}/actions/publish",
            self.base.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(classify_scholarly_http(status, &text));
        }
        serde_json::from_str(&text).map_err(|e| ScholarlyError::Fatal {
            code: "zenodo_json_parse".into(),
            message: format!("publish response JSON: {e}; body={text}"),
        })
    }

    pub(super) async fn publish_deposition(
        &self,
        deposition_id: &str,
    ) -> Result<ZenodoDeposition, ScholarlyError> {
        let max = zenodo_http_max_attempts();
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            match self.publish_deposition_once(deposition_id).await {
                Ok(v) => return Ok(v),
                Err(e) if e.retryable() && attempt < max => {
                    sleep_before_zenodo_retry(&e, attempt.saturating_sub(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ZenodoAdapter {
    client: ZenodoHttpClient,
}

impl ZenodoAdapter {
    pub(super) fn from_token(sandbox: bool, token: String) -> Result<Self, ScholarlyError> {
        Ok(Self {
            client: ZenodoHttpClient::new(sandbox, token)?,
        })
    }
}

#[async_trait]
impl super::ScholarlyAdapter for ZenodoAdapter {
    fn adapter_name(&self) -> &'static str {
        "zenodo"
    }

    async fn submit(
        &self,
        manifest: &PublicationManifest,
    ) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
        let body = zenodo_metadata::zenodo_deposition_create_body(manifest);
        let mut dep = self.client.create_deposition_draft(&body).await?;
        let id = dep.id;
        let attach = flags::zenodo_attach_manifest_body();
        let publish = flags::zenodo_publish_deposition();
        if publish && !attach {
            return Err(ScholarlyError::Config {
                message: "VOX_ZENODO_PUBLISH_DEPOSITION requires VOX_ZENODO_ATTACH_MANIFEST_BODY (deposit must include at least one file before publish)"
                    .into(),
            });
        }
        if attach {
            let bucket = ZenodoHttpClient::bucket_url_from_deposition(&dep)?;
            self.client
                .put_bucket_object(
                    &bucket,
                    "body.md",
                    manifest.body_markdown.as_bytes(),
                    "text/markdown; charset=utf-8",
                )
                .await?;
        }
        if publish {
            dep = self.client.publish_deposition(&id.to_string()).await?;
        }
        let state = if dep.state.is_empty() {
            "draft".to_string()
        } else {
            dep.state.clone()
        };
        let digest = manifest.content_sha3_256();
        let meta_json = serde_json::to_string(&dep).map_err(|e| ScholarlyError::Fatal {
            code: "zenodo_receipt_encode".into(),
            message: format!("serialize deposition: {e}"),
        })?;
        Ok(ScholarlySubmissionReceipt {
            adapter: self.adapter_name().to_string(),
            external_submission_id: id.to_string(),
            status: state,
            response_fingerprint: Some(digest),
            metadata_json: Some(meta_json),
        })
    }

    async fn fetch_status(
        &self,
        external_submission_id: &str,
    ) -> Result<ScholarlyRemoteStatus, ScholarlyError> {
        let dep = self.client.get_deposition(external_submission_id).await?;
        let status = if dep.state.is_empty() {
            "unknown".to_string()
        } else {
            dep.state.clone()
        };
        let detail = serde_json::to_string(&dep).map_err(|e| ScholarlyError::Fatal {
            code: "zenodo_status_encode".into(),
            message: format!("serialize deposition: {e}"),
        })?;
        Ok(ScholarlyRemoteStatus {
            status,
            detail_json: Some(detail),
        })
    }
}

pub(super) fn zenodo_from_clavis() -> Result<ZenodoAdapter, ScholarlyError> {
    if flags::adapter_live_disabled("zenodo") {
        return Err(ScholarlyError::Disabled {
            reason: "VOX_SCHOLARLY_DISABLE_ZENODO is set".into(),
        });
    }
    let token = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxZenodoAccessToken)
        .expose()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let Some(token) = token else {
        return Err(ScholarlyError::Config {
            message: "missing Zenodo token: set ZENODO_ACCESS_TOKEN (or VOX_ZENODO_ACCESS_TOKEN) per Clavis / `vox clavis doctor`"
                .into(),
        });
    };
    ZenodoAdapter::from_token(flags::zenodo_use_sandbox(), token)
}
