//! Stable repository identity (blake3).

use std::path::Path;

/// 16-character lowercase hex id: blake3(origin_url + NUL + canonical root), or root-only if no origin.
pub fn compute_repository_id(canonical_root: &Path, origin_url: Option<&str>) -> String {
    let mut h = blake3::Hasher::new();
    if let Some(url) = origin_url {
        let u = url.trim();
        if !u.is_empty() {
            h.update(u.as_bytes());
            h.update(&[0]);
        }
    }
    h.update(canonical_root.to_string_lossy().as_bytes());
    let out = h.finalize();
    let b = out.as_bytes();
    (0..8).map(|i| format!("{:02x}", b[i])).collect()
}
