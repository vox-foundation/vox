//! Shared **text construction** for training pairs (Candle plain triple, Burn ChatML supervision).
//!
//! **SSOT:** objective differences between Burn and Candle QLoRA are documented in
//! `docs/src/architecture/mens-training-ssot.md` (“Training objective mismatch”).
//! Do not assume loss curves match across kernels.
//!
//! Both trainers encode ChatML through the functions below. Direct answers use
//! Qwen3 non-thinking fusion (see [`chatml_prefix_open_assistant`]).
//! [`hf_tokenize_chatml_supervised`] (feature `mens-candle-qlora`) masks the prompt.

/// Plain `system \\n prompt \\n response` — matches historical Candle qlora encoding.
#[must_use]
pub fn plain_system_prompt_response(system: &str, prompt: &str, response: &str) -> String {
    format!("{system}\n{prompt}\n{response}")
}

/// ChatML prefix through the open assistant slot.
///
/// `assistant` selects the mode: a non-empty think block keeps the thinking-mode
/// entry point; anything else is a direct answer and the returned prefix includes
/// `/no_think` plus the empty think block (masked out of the loss).
#[must_use]
pub fn chatml_prefix_open_assistant(
    system: &str,
    user: &str,
    assistant: &str,
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> String {
    let reasoning = vox_eval::qwen_reasoning::has_reasoning_trace(assistant);
    let user = vox_eval::qwen_reasoning::user_for_mode(user, reasoning);
    let tail = if reasoning {
        ""
    } else {
        vox_eval::qwen_reasoning::QWEN_EMPTY_THINK
    };
    format!(
        "{start}{sys}\n{system}{end}\n\
         {start}{usr}\n{user}{end}\n\
         {start}{asst}\n{tail}",
        start = cfg.im_start,
        end = cfg.im_end,
        sys = cfg.role_system,
        usr = cfg.role_user,
        asst = cfg.role_assistant,
        tail = tail,
    )
}

/// Format multi-turn turns as full ChatML text.
#[must_use]
pub fn chatml_turns_text(
    turns: &[vox_tensor::data::ChatmlTurn],
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> String {
    let turns = fuse_turns(turns, cfg);
    let mut out = String::new();
    for turn in &turns {
        out.push_str(&format!(
            "{start}{role}\n{content}{end}\n",
            start = cfg.im_start,
            end = cfg.im_end,
            role = turn.role,
            content = turn.content
        ));
    }
    out.trim_end().to_string()
}

/// Open assistant slot after all turns (or after the prefix of the last assistant turn).
#[must_use]
pub fn chatml_turns_prefix_open_assistant(
    turns: &[vox_tensor::data::ChatmlTurn],
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> String {
    let turns = fuse_turns(turns, cfg);
    let mut out = String::new();
    for (i, turn) in turns.iter().enumerate() {
        if i == turns.len() - 1 && turn.role == cfg.role_assistant {
            out.push_str(&format!(
                "{start}{role}\n",
                start = cfg.im_start,
                role = cfg.role_assistant
            ));
            if !vox_eval::qwen_reasoning::has_reasoning_trace(&turn.content) {
                out.push_str(vox_eval::qwen_reasoning::QWEN_EMPTY_THINK);
            }
            break;
        }
        out.push_str(&format!(
            "{start}{role}\n{content}{end}\n",
            start = cfg.im_start,
            end = cfg.im_end,
            role = turn.role,
            content = turn.content
        ));
    }
    out
}

/// Full ChatML string for SFT (system, user, assistant content).
#[must_use]
pub fn chatml_supervised_text(
    system: &str,
    user: &str,
    assistant: &str,
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> String {
    let reasoning = vox_eval::qwen_reasoning::has_reasoning_trace(assistant);
    let user = vox_eval::qwen_reasoning::user_for_mode(user, reasoning);
    let assistant = vox_eval::qwen_reasoning::assistant_body(assistant);
    format!(
        "{start}{sys}\n{system}{end}\n\
         {start}{usr}\n{user}{end}\n\
         {start}{asst}\n{assistant}{end}",
        start = cfg.im_start,
        end = cfg.im_end,
        sys = cfg.role_system,
        usr = cfg.role_user,
        asst = cfg.role_assistant,
        assistant = assistant,
    )
}

fn fuse_turns(
    turns: &[vox_tensor::data::ChatmlTurn],
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> Vec<vox_tensor::data::ChatmlTurn> {
    use vox_eval::qwen_reasoning::{self, QWEN_EMPTY_THINK};
    let mut out = turns.to_vec();
    for i in 0..out.len() {
        if out[i].role != cfg.role_assistant || qwen_reasoning::has_reasoning_trace(&out[i].content)
        {
            continue;
        }
        if !out[i].content.starts_with(QWEN_EMPTY_THINK) {
            out[i].content = format!("{QWEN_EMPTY_THINK}{}", out[i].content);
        }
        if let Some(u) = out[..i].iter().rposition(|t| t.role == cfg.role_user) {
            out[u].content = qwen_reasoning::user_for_mode(&out[u].content, false);
        }
    }
    out
}

/// Tokenize ChatML SFT with HF tokenizer: prompt tokens masked with `-100`, assistant + EOS supervised.
///
/// Mirrors `VoxTokenizer::tokenize_for_training` layout (pad id 0, eos appended, length `max_len`).
#[cfg(feature = "mens-candle-qlora")]
const HF_MASK_IGNORE: i64 = -100;

#[cfg(feature = "mens-candle-qlora")]
pub fn hf_tokenize_chatml_supervised(
    tokenizer: &tokenizers::Tokenizer,
    system: &str,
    user: &str,
    assistant: &str,
    max_len: usize,
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> anyhow::Result<(Vec<i64>, Vec<i64>)> {
    let full_text = chatml_supervised_text(system, user, assistant, cfg);
    let prompt_text = chatml_prefix_open_assistant(system, user, assistant, cfg);

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
        .token_to_id(&cfg.im_end)
        .or_else(|| tokenizer.token_to_id("<|im_end|>"))
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
        let cfg = crate::mens::tensor::training_config::ChatmlConfig::default();
        let t = chatml_supervised_text("sys", "usr", "asst", &cfg);
        assert!(t.contains(&cfg.im_start));
        assert!(t.contains("user"));
        assert!(t.contains("asst"));
    }

    /// Burn (ChatML HF) and Candle (plain triple) both consume the same ChatML SFT layout via this module.
    #[test]
    fn chatml_supervised_starts_with_open_assistant_prefix() {
        let cfg = crate::mens::tensor::training_config::ChatmlConfig::default();
        let full = chatml_supervised_text("S", "U", "A", &cfg);
        let prefix = chatml_prefix_open_assistant("S", "U", "A", &cfg);
        assert!(
            full.starts_with(prefix.as_str()),
            "supervised text must begin with the open-assistant prefix"
        );
        assert!(full.ends_with(&format!("A{}", cfg.im_end)));
    }

    #[test]
    fn direct_answer_puts_empty_think_in_the_prompt_not_the_loss() {
        let cfg = crate::mens::tensor::training_config::ChatmlConfig::default();
        let full = chatml_supervised_text("S", "U", "code", &cfg);
        let prefix = chatml_prefix_open_assistant("S", "U", "code", &cfg);
        assert!(
            full.starts_with(&prefix),
            "{full} does not start with {prefix}"
        );
        assert!(prefix.contains(" /no_think"));
        assert!(prefix.ends_with(vox_eval::qwen_reasoning::QWEN_EMPTY_THINK));
        assert_eq!(
            &full[prefix.len()..],
            format!("code{}", cfg.im_end),
            "loss span is the answer only"
        );
    }

    #[test]
    fn reasoning_trace_keeps_the_thinking_mode_entry_point() {
        let cfg = crate::mens::tensor::training_config::ChatmlConfig::default();
        let answer = "<think>\nplan\n</think>\n\ncode";
        let full = chatml_supervised_text("S", "U", answer, &cfg);
        let prefix = chatml_prefix_open_assistant("S", "U", answer, &cfg);
        assert!(full.starts_with(&prefix));
        assert!(!prefix.contains("/no_think"));
        assert!(prefix.ends_with("<|im_start|>assistant\n"));
        assert!(full.contains("<think>\nplan\n</think>"));
    }
}
