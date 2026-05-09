//! ArxivAssist ScholarlyAdapter: stages submission bundle and hands off to operator.
//!
//! arXiv does not expose an open automated deposit API. This adapter packages the
//! publication as an operator-assist staging bundle (handoff JSON + body.md +
//! main.tex stub) and returns `status = "staged"`. The operator uploads the bundle.

use async_trait::async_trait;

use super::{ScholarlyAdapter, ScholarlyError, ScholarlyRemoteStatus, ScholarlySubmissionReceipt};
use crate::publication::PublicationManifest;
use crate::submission::{arxiv_assist_main_tex, arxiv_operator_handoff_value};

#[derive(Debug, Default, Clone)]
pub struct ArxivAssistAdapter;

#[async_trait]
impl ScholarlyAdapter for ArxivAssistAdapter {
    fn adapter_name(&self) -> &'static str {
        "arxiv_assist"
    }

    async fn submit(
        &self,
        manifest: &PublicationManifest,
    ) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
        let digest = manifest.content_sha3_256();
        let external_submission_id = format!("arxiv-{}", &digest[..16.min(digest.len())]);
        let handoff = arxiv_operator_handoff_value(manifest);
        let main_tex = arxiv_assist_main_tex(manifest);
        let metadata = serde_json::json!({
            "workflow": handoff["workflow"],
            "main_tex_preview": &main_tex[..main_tex.len().min(200)],
            "content_sha3_256": digest,
            "note": "Operator-assisted arXiv submission. Upload arxiv_bundle.tar.gz via arXiv web interface.",
        });
        Ok(ScholarlySubmissionReceipt {
            adapter: self.adapter_name().to_string(),
            external_submission_id,
            status: "staged".to_string(),
            response_fingerprint: Some(digest),
            metadata_json: Some(metadata.to_string()),
        })
    }

    async fn fetch_status(
        &self,
        external_submission_id: &str,
    ) -> Result<ScholarlyRemoteStatus, ScholarlyError> {
        Ok(ScholarlyRemoteStatus {
            status: "pending_operator".to_string(),
            detail_json: Some(
                serde_json::json!({
                    "external_submission_id": external_submission_id,
                    "note": "arXiv submission requires operator upload via web interface."
                })
                .to_string(),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publication::PublicationManifest;

    fn sample_manifest() -> PublicationManifest {
        PublicationManifest {
            publication_id: "pub-arxiv-1".into(),
            content_type: "paper".into(),
            source_ref: None,
            title: "Provider Latency Drift: A Longitudinal Study".into(),
            author: "Vox Research Team".into(),
            abstract_text: Some("We measure provider latency drift.".into()),
            body_markdown: "# Introduction\n\nLatency matters.".into(),
            citations_json: None,
            metadata_json: None,
        }
    }

    #[tokio::test]
    async fn arxiv_assist_adapter_submit_returns_staged() {
        let adapter = ArxivAssistAdapter;
        let manifest = sample_manifest();
        let receipt = adapter.submit(&manifest).await.expect("submit");
        assert_eq!(receipt.adapter, "arxiv_assist");
        assert_eq!(receipt.status, "staged");
        assert!(receipt.external_submission_id.starts_with("arxiv-"));
        let meta: serde_json::Value =
            serde_json::from_str(receipt.metadata_json.as_deref().unwrap_or("{}"))
                .expect("meta json");
        assert_eq!(meta["workflow"], "arxiv_operator_assist");
    }

    #[tokio::test]
    async fn arxiv_assist_fetch_status_returns_pending_operator() {
        let adapter = ArxivAssistAdapter;
        let status = adapter.fetch_status("arxiv-abc123").await.expect("status");
        assert_eq!(status.status, "pending_operator");
    }
}
