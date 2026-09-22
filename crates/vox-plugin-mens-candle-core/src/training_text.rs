//! Shared text construction for training pairs (ChatML encoding).
//!
//! Ported verbatim from `vox-populi/src/mens/tensor/training_text.rs` (SP3 sub-batch C).

use crate::config::ChatmlConfig;

#[must_use]
pub fn chatml_prefix_open_assistant(system: &str, user: &str, cfg: &ChatmlConfig) -> String {
    format!(
        "{start}{sys}\n{system}{end}\n\
         {start}{usr}\n{user}{end}\n\
         {start}{asst}\n",
        start = cfg.im_start,
        end = cfg.im_end,
        sys = cfg.role_system,
        usr = cfg.role_user,
        asst = cfg.role_assistant
    )
}

#[must_use]
pub fn chatml_turns_text(turns: &[vox_tensor::data::ChatmlTurn], cfg: &ChatmlConfig) -> String {
    let mut out = String::new();
    for turn in turns {
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

#[must_use]
pub fn chatml_turns_prefix_open_assistant(
    turns: &[vox_tensor::data::ChatmlTurn],
    cfg: &ChatmlConfig,
) -> String {
    let mut out = String::new();
    for (i, turn) in turns.iter().enumerate() {
        if i == turns.len() - 1 && turn.role == "assistant" {
            out.push_str(&format!(
                "{start}{role}\n",
                start = cfg.im_start,
                role = cfg.role_assistant
            ));
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

#[must_use]
pub fn chatml_supervised_text(
    system: &str,
    user: &str,
    assistant: &str,
    cfg: &ChatmlConfig,
) -> String {
    format!(
        "{start}{sys}\n{system}{end}\n\
         {start}{usr}\n{user}{end}\n\
         {start}{asst}\n{assistant}{end}",
        start = cfg.im_start,
        end = cfg.im_end,
        sys = cfg.role_system,
        usr = cfg.role_user,
        asst = cfg.role_assistant
    )
}

/// `messages` rows get the same system turn prompt/response rows get, unless they
/// already open with one — otherwise ~16% of rows train in a different context
/// than inference serves.
#[must_use]
pub fn with_system_turn(
    turns: &[vox_tensor::data::ChatmlTurn],
    system: &str,
    cfg: &ChatmlConfig,
) -> Vec<vox_tensor::data::ChatmlTurn> {
    let mut out = Vec::with_capacity(turns.len() + 1);
    if !system.is_empty() && turns.first().is_none_or(|t| t.role != cfg.role_system) {
        out.push(vox_tensor::data::ChatmlTurn {
            role: cfg.role_system.clone(),
            content: system.to_string(),
        });
    }
    out.extend_from_slice(turns);
    out
}

/// Clip a tokenized row to `seq_len`, keeping the **head** (system + prompt + the
/// start of the answer). Keeping the tail instead cut the prompt first, training
/// unconditioned completions. Returns `(ids, trunc_offset)`; the offset is always
/// 0 now and is kept for the mask/alignment call sites.
#[must_use]
pub fn truncate_to_seq_len(mut ids: Vec<u32>, seq_len: usize) -> (Vec<u32>, usize) {
    ids.truncate(seq_len);
    (ids, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_tensor::data::ChatmlTurn;

    fn turn(role: &str, content: &str) -> ChatmlTurn {
        ChatmlTurn {
            role: role.into(),
            content: content.into(),
        }
    }

    #[test]
    fn messages_rows_gain_system_turn_once() {
        let cfg = ChatmlConfig::default();
        let t = vec![turn("user", "u"), turn("assistant", "a")];
        let out = with_system_turn(&t, "SYS", &cfg);
        assert_eq!(
            (out[0].role.as_str(), out[0].content.as_str()),
            ("system", "SYS")
        );
        assert_eq!(out.len(), 3);
        let own = vec![turn("system", "mine"), turn("user", "u")];
        assert_eq!(with_system_turn(&own, "SYS", &cfg)[0].content, "mine");
    }

    #[test]
    fn truncation_keeps_the_prompt_head() {
        assert_eq!(
            truncate_to_seq_len(vec![1, 2, 3, 4, 5], 3),
            (vec![1, 2, 3], 0)
        );
        assert_eq!(truncate_to_seq_len(vec![1, 2], 3), (vec![1, 2], 0));
    }
}
