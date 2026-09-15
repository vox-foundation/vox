use crate::VoxDb;
use crate::store::StoreError;
use turso::params;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CachedWebArtifact {
    pub url_hash: String,
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub status_code: u16,
    pub content_type: String,
    pub raw_body: Vec<u8>,
    pub extracted_markdown: String,
    pub fetched_at_ms: i64,
}

pub fn hash_url(url: &str) -> String {
    let normalized = url.trim().trim_end_matches('/');
    blake3::hash(normalized.as_bytes()).to_hex().to_string()
}

impl VoxDb {
    pub async fn get_cached_web_artifact(
        &self,
        url: &str,
    ) -> Result<Option<CachedWebArtifact>, StoreError> {
        let hash = hash_url(url);
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();

        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT url_hash, url, etag, last_modified, status_code, content_type, raw_body, extracted_markdown, fetched_at_ms
                     FROM web_cache WHERE url_hash = ?1",
                        params![hash.as_str()],
                    )
                    .await?;

                if let Some(row) = rows.next().await? {
                    let status_code: i64 = row.get(4)?;
                    let raw_body: Vec<u8> = row.get(6)?;
                    Ok(Some(CachedWebArtifact {
                        url_hash: row.get(0)?,
                        url: row.get(1)?,
                        etag: row.get(2)?,
                        last_modified: row.get(3)?,
                        status_code: status_code as u16,
                        content_type: row.get(5)?,
                        raw_body,
                        extracted_markdown: row.get(7)?,
                        fetched_at_ms: row.get(8)?,
                    }))
                } else {
                    Ok(None)
                }
            })
            .await
    }

    pub async fn put_cached_web_artifact(
        &self,
        artifact: &CachedWebArtifact,
    ) -> Result<(), StoreError> {
        let hash = hash_url(&artifact.url);
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();
        let art = artifact.clone();

        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR REPLACE INTO web_cache
                 (url_hash, url, etag, last_modified, status_code, content_type, raw_body, extracted_markdown, fetched_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        hash.as_str(),
                        art.url.as_str(),
                        art.etag.as_deref(),
                        art.last_modified.as_deref(),
                        art.status_code as i64,
                        art.content_type.as_str(),
                        art.raw_body.as_slice(),
                        art.extracted_markdown.as_str(),
                        art.fetched_at_ms,
                    ],
                )
                .await?;
                Ok(())
            })
            .await
    }

    pub async fn touch_cached_web_artifact(
        &self,
        url: &str,
        touched_at_ms: i64,
    ) -> Result<(), StoreError> {
        let hash = hash_url(url);
        let conn = self.conn.clone();
        let breaker = self.breaker.clone();

        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE web_cache SET fetched_at_ms = ?1 WHERE url_hash = ?2",
                    params![touched_at_ms, hash.as_str()],
                )
                .await?;
                Ok(())
            })
            .await
    }
}
