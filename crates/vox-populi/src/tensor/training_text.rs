//! Shared **text construction** for training pairs (Candle plain triple, Burn ChatML supervision).
//!
//! **SSOT:** objective differences between Burn and Candle QLoRA are documented in
//! `docs/src/architecture/populi-training-ssot.md` (“Training objective mismatch”).
//! Do not assume loss curves match across kernels.
//!
//! Candle QLoRA uses [`plain_system_prompt_response`]. Burn + HF tokenizer uses
//! [`hf_tokenize_chatml_supervised`] (requires `gpu`) to align masking with
//! [`vox_tensor::data::VoxTokenizer::tokenize_for_training`].

/// Plain `system \\n prompt \\n response` — matches historical Candle qlora encoding.
#[must_use]
pub fn plain_system_prompt_response(system: &str, prompt: &str, response: &str) -> String {
    format!("{system}\n{prompt}\n{response}")
}

/// ChatML prefix through the user turn (open assistant slot) — same structure as `VoxTokenizer`.
#[must_use]
pub fn chatml_prefix_open_assistant(system: &str, user: &str) -> String {
    format!(
        "<|im_start|>system\n{system}<|im_end|>\n\
         <|im_start|>user\n{user}<|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

/// Full ChatML string for SFT (system, user, assistant content).
#[must_use]
pub fn chatml_supervised_text(system: &str, user: &str, assistant: &str) -> String {
    format!(
        "<|im_start|>system\n{system}<|im_end|>\n\
         <|im_start|>user\n{user}<|im_end|>\n\
         <|im_start|>assistant\n{assistant}<|im_end|>"
    )
}

/// Tokenize ChatML SFT with HF tokenizer: prompt tokens masked with `-100`, assistant + EOS supervised.
///
/// Mirrors `VoxTokenizer::tokenize_for_training` layout (pad id 0, eos appended, length `max_len`).
#[cfg(feature = "gpu")]
const HF_MASK_IGNORE: i64 = -100;

#[cfg(feature = "gpu")]
pub fn hf_tokenize_chatml_supervised(
    tokenizer: &tokenizers::Tokenizer,
    system: &str,
    user: &str,
    assistant: &str,
    max_len: usize,
) -> anyhow::Result<(Vec<i64>, Vec<i64>)> {
    let full_text = chatml_supervised_text(system, user, assistant);
    let prompt_text = chatml_prefix_open_assistant(system, user);

    let full_enc = tokenizer
        .encode(full_text.as_str(), true)
        .map_err(|e| anyhow::anyhow!("tokenizer encode full: {e}"))?;
    let prompt_enc = tokenizer
        .encode(prompt_text.as_str(), true)
        .map_err(|e| anyhow::anyhow!("tokenizer encode prompt: {e}"))?;

    let mut full_ids: Vec<i64> = full_enc.get_ids().iter().map(|&x| x as i64).collect();
    let prompt_len = prompt_enc.len();

    if full_ids.len() > max_len.saturating_sub(1) {
        let drop = full_ids.len() - (max_len.saturating_sub(1));
        full_ids.drain(0..drop);
    }

    let eos_id = tokenizer
        .token_to_id("<|im_end|>")
        .or_else(|| tokenizer.token_to_id("</s>"))
        .or_else(|| tokenizer.token_to_id("<|endoftext|>"))
        .unwrap_or(2) as i64;

    let mut truncated: Vec<i64> = full_ids
        .into_iter()
        .take(max_len.saturating_sub(1))
        .chain(std::iter::once(eos_id))
        .collect();
    let actual_len = truncated.len();
    truncated.resize(max_len, 0);

    let prompt_len_clamped = prompt_len.min(actual_len);
    let labels: Vec<i64> = truncated
        .iter()
        .enumerate()
        .map(|(i, &tok)| {
            if i < prompt_len_clamped || i >= actual_len || tok == 0 {
                HF_MASK_IGNORE
            } else {
                tok
            }
        })
        .collect();

    Ok((truncated, labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_format_has_newlines() {
        let s = plain_system_prompt_response("S", "P", "R");
        assert_eq!(s, "S\nP\nR");
    }

    #[test]
    fn chatml_contains_markers() {
        let t = chatml_supervised_text("sys", "usr", "asst");
        assert!(t.contains("<|im_start|>"));
        assert!(t.contains("user"));
        assert!(t.contains("asst"));
    }

    /// Burn (ChatML HF) and Candle (plain triple) both consume the same ChatML SFT layout via this module.
    #[test]
    fn chatml_supervised_starts_with_open_assistant_prefix() {
        let full = chatml_supervised_text("S", "U", "A");
        let prefix = chatml_prefix_open_assistant("S", "U");
        assert!(
            full.starts_with(prefix.as_str()),
            "supervised text must begin with the open-assistant prefix"
        );
        assert!(full.ends_with("A<|im_end|>"));
    }
}
