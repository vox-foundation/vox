//! Ranking helper for docs retrieval.

use crate::types::FileEntry;

/// Docs retrieval ranking helper (WS10): combines doc-line density, hotspot tier, and mild size signal.
#[must_use]
pub fn relevance_score(entry: &FileEntry) -> f64 {
    let doc_lines =
        (entry.lines_triple_slash + entry.lines_inner_doc + entry.lines_other_doc_signal) as f64;
    let plain = entry.lines_plain_comment as f64;
    let hotspot_mul = match entry.hotspot_tier {
        1 => 2.75_f64,
        2 => 1.55_f64,
        _ => 1.0_f64,
    };
    let size_signal = ((entry.lines_total as f64) + 1.0).ln();
    (doc_lines + plain * 0.08_f64) * hotspot_mul + size_signal * 0.12_f64
}
