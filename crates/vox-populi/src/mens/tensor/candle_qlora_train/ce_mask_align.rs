//! Token-aligned supervised-prefix length for masked CE (avoids BPE boundary skew).

use anyhow::Result;
use tokenizers::Tokenizer;

/// Longest initial run where standalone `prefix_text` tokenization matches `full_text` tokenization.
///
/// `encode(prefix)` and `encode(full)` can diverge at the boundary even when `full` starts with
/// the same Unicode prefix; masked CE must index into **`full` token ids** (including after
/// truncation), so we use this LCP length as `prefix_len` in full-token coordinates.
pub(super) fn aligned_prefix_token_len(
    tokenizer: &Tokenizer,
    prefix_text: &str,
    full_text: &str,
) -> Result<usize> {
    let prefix_ids = tokenizer
        .encode(prefix_text, true)
        .map_err(|e| anyhow::anyhow!("tokenizer encode prefix: {e}"))?
        .get_ids()
        .to_vec();
    let full_ids = tokenizer
        .encode(full_text, true)
        .map_err(|e| anyhow::anyhow!("tokenizer encode full: {e}"))?
        .get_ids()
        .to_vec();

    let mut matched = 0usize;
    let upper = prefix_ids.len().min(full_ids.len());
    for i in 0..upper {
        if prefix_ids[i] == full_ids[i] {
            matched = i + 1;
        } else {
            break;
        }
    }

    if matched < prefix_ids.len() {
        tracing::warn!(
            target: "vox_mens_train",
            standalone_prefix_tokens = prefix_ids.len(),
            lcp_with_full_tokens = matched,
            "ChatML prefix tokenization diverged from full-text tokenization; CE mask uses longest matching initial token run"
        );
    }

    Ok(matched)
}
