//! Shared text construction for training pairs (ChatML encoding).
//!
//! Direct-answer rows use Qwen3 non-thinking fusion (`vox_eval::qwen_reasoning`):
//! `/no_think` on the user turn and an empty `<think>` block that is part of the
//! prompt, so loss starts at the answer. Rows that already contain a non-empty
//! think block stay thinking-mode examples and are not rewritten.

use crate::config::ChatmlConfig;
use vox_eval::qwen_reasoning::{self, Lane, QWEN_EMPTY_THINK};

#[must_use]
pub fn chatml_prefix_open_assistant(
    system: &str,
    user: &str,
    assistant: &str,
    cfg: &ChatmlConfig,
) -> String {
    let reasoning = qwen_reasoning::has_reasoning_trace(assistant);
    let user = qwen_reasoning::user_for_mode(user, reasoning);
    let tail = if reasoning { "" } else { QWEN_EMPTY_THINK };
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

/// Raw `/generate` prefix for one serving lane.
///
/// Codegen delegates to [`chatml_prefix_open_assistant`] with an empty
/// assistant, so it stays byte-identical to the QLoRA prompt (`/no_think` on the
/// user turn, empty think block prefilled). Hub and agent use the same template
/// with the lane's user text and assistant opener from
/// [`qwen_reasoning`]: no `/no_think`, assistant turn left open.
#[must_use]
pub fn chatml_prefix_for_lane(system: &str, user: &str, lane: Lane, cfg: &ChatmlConfig) -> String {
    if lane == Lane::Codegen {
        return chatml_prefix_open_assistant(system, user, "", cfg);
    }
    let user = qwen_reasoning::user_for_lane(user, lane);
    let tail = qwen_reasoning::assistant_opener_for_lane(lane);
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

#[must_use]
pub fn chatml_turns_text(turns: &[vox_tensor::data::ChatmlTurn], cfg: &ChatmlConfig) -> String {
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

#[must_use]
pub fn chatml_turns_prefix_open_assistant(
    turns: &[vox_tensor::data::ChatmlTurn],
    cfg: &ChatmlConfig,
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
            // Empty think block is prompt, not loss. Reasoning traces are not.
            if !qwen_reasoning::has_reasoning_trace(&turn.content) {
                out.push_str(QWEN_EMPTY_THINK);
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

#[must_use]
pub fn chatml_supervised_text(
    system: &str,
    user: &str,
    assistant: &str,
    cfg: &ChatmlConfig,
) -> String {
    let reasoning = qwen_reasoning::has_reasoning_trace(assistant);
    let user = qwen_reasoning::user_for_mode(user, reasoning);
    let assistant = qwen_reasoning::assistant_body(assistant);
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

/// Rewrite direct-answer turns into Qwen3 non-thinking form. Thinking-mode
/// assistant turns (non-empty `<think>` body) are left alone, as is the user
/// turn that produced them.
fn fuse_turns(
    turns: &[vox_tensor::data::ChatmlTurn],
    cfg: &ChatmlConfig,
) -> Vec<vox_tensor::data::ChatmlTurn> {
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

/// Fit a tokenized row (`prefix_len` prompt tokens, then the answer) into
/// `seq_len`, returning `(ids, trunc_offset)` where `trunc_offset` is how many
/// tokens were removed *before* the answer (the mask uses
/// `prefix_len - trunc_offset` as the new prompt length).
///
/// The answer is what trains, so it is guaranteed up to half the window. An
/// over-long prompt is trimmed from its middle — keeping the start of the system
/// turn and the end of the user turn plus the assistant opener — and only then is
/// the answer's tail cut. Keeping the tail of the whole row (the old rule) dropped
/// the prompt first; keeping the head dropped the whole answer whenever the system
/// prompt alone exceeded `seq_len`.
#[must_use]
pub fn fit_to_seq_len(ids: Vec<u32>, prefix_len: usize, seq_len: usize) -> (Vec<u32>, usize) {
    if ids.len() <= seq_len {
        return (ids, 0);
    }
    let prefix_len = prefix_len.min(ids.len());
    let answer_len = ids.len() - prefix_len;
    let prefix_budget = seq_len - answer_len.min(seq_len / 2);
    if prefix_len <= prefix_budget {
        let mut ids = ids;
        ids.truncate(seq_len);
        return (ids, 0);
    }
    let head = prefix_budget / 4;
    let tail = prefix_budget - head;
    let removed = prefix_len - prefix_budget;
    let mut out = Vec::with_capacity(seq_len);
    out.extend_from_slice(&ids[..head]);
    out.extend_from_slice(&ids[prefix_len - tail..]);
    out.truncate(seq_len);
    (out, removed)
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
    fn short_rows_are_untouched() {
        assert_eq!(fit_to_seq_len(vec![1, 2, 3], 2, 8), (vec![1, 2, 3], 0));
    }

    #[test]
    fn long_answer_is_cut_at_its_tail_when_the_prompt_fits() {
        // prompt = 0..4, answer = 100..120; window 10.
        let ids: Vec<u32> = (0..4).chain(100..120).collect();
        let (out, off) = fit_to_seq_len(ids, 4, 10);
        assert_eq!(off, 0);
        assert_eq!(out, vec![0, 1, 2, 3, 100, 101, 102, 103, 104, 105]);
    }

    #[test]
    fn oversized_prompt_is_trimmed_in_the_middle_and_the_answer_survives() {
        // A 20-token prompt (system prompt larger than the window) + 3-token answer.
        let ids: Vec<u32> = (0..20).chain(100..103).collect();
        let (out, off) = fit_to_seq_len(ids, 20, 10);
        assert_eq!(out.len(), 10);
        assert_eq!(&out[out.len() - 3..], &[100, 101, 102], "whole answer kept");
        assert_eq!(out[0], 0, "start of the system turn kept");
        assert_eq!(out[out.len() - 4], 19, "end of the user turn kept");
        // The mask's prompt length is prefix_len - trunc_offset = 7 = answer start.
        assert_eq!(20 - off, 7);
    }

    #[test]
    fn huge_answer_gets_half_the_window() {
        let ids: Vec<u32> = (0..20).chain(100..200).collect();
        let (out, off) = fit_to_seq_len(ids, 20, 10);
        assert_eq!(20 - off, 5, "prompt squeezed to half");
        assert_eq!(&out[5..], &[100, 101, 102, 103, 104]);
    }

    #[test]
    fn direct_answer_puts_empty_think_in_the_prompt_not_the_loss() {
        let cfg = ChatmlConfig::default();
        let full = chatml_supervised_text("S", "U", "code", &cfg);
        let prefix = chatml_prefix_open_assistant("S", "U", "code", &cfg);
        assert!(
            full.starts_with(&prefix),
            "{full} does not start with {prefix}"
        );
        assert!(prefix.contains(" /no_think"));
        assert!(prefix.ends_with(QWEN_EMPTY_THINK));
        assert_eq!(
            &full[prefix.len()..],
            format!("code{}", cfg.im_end),
            "loss span is the answer only"
        );
    }

    #[test]
    fn reasoning_trace_keeps_the_thinking_mode_entry_point() {
        let cfg = ChatmlConfig::default();
        let answer = "<think>\nplan the construct\n</think>\n\ncode";
        let full = chatml_supervised_text("S", "U", answer, &cfg);
        let prefix = chatml_prefix_open_assistant("S", "U", answer, &cfg);
        assert!(full.starts_with(&prefix));
        assert!(!prefix.contains("/no_think"));
        assert!(prefix.ends_with("<|im_start|>assistant\n"));
        assert!(full.contains("<think>\nplan the construct\n</think>"));
    }

    #[test]
    fn message_rows_mask_the_empty_think_block_the_same_way() {
        let cfg = ChatmlConfig::default();
        let turns = vec![turn("user", "U"), turn("assistant", "code")];
        let full = chatml_turns_text(&turns, &cfg);
        let prefix = chatml_turns_prefix_open_assistant(&turns, &cfg);
        assert!(
            full.starts_with(&prefix),
            "{full} does not start with {prefix}"
        );
        assert!(prefix.contains(" /no_think"));
        assert!(prefix.ends_with(QWEN_EMPTY_THINK));
        assert_eq!(&full[prefix.len()..], format!("code{}", cfg.im_end));
    }

    #[test]
    fn chatml_prefix_for_lane_codegen_matches_open_assistant() {
        let cfg = ChatmlConfig::default();
        for (sys, user) in [("S", "hi"), ("S", "hi /think"), ("", "hi /no_think")] {
            assert_eq!(
                chatml_prefix_for_lane(sys, user, Lane::Codegen, &cfg),
                chatml_prefix_open_assistant(sys, user, "", &cfg),
                "codegen drifted for ({sys:?}, {user:?})"
            );
        }
    }

    #[test]
    fn chatml_prefix_for_lane_hub_and_agent_are_bare_thinking() {
        let cfg = ChatmlConfig::default();
        let want = "<|im_start|>system\nS<|im_end|>\n<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n";
        assert_eq!(chatml_prefix_for_lane("S", "hi", Lane::Hub, &cfg), want);
        assert_eq!(chatml_prefix_for_lane("S", "hi", Lane::Agent, &cfg), want);
        let stripped = chatml_prefix_for_lane("S", "hi /no_think", Lane::Hub, &cfg);
        assert_eq!(
            stripped, want,
            "/no_think is stripped on the thinking lanes"
        );
        assert!(!stripped.contains("<think>"));
    }
}
