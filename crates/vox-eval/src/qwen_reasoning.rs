//! Qwen3 thinking-mode helpers: non-thinking SFT format, and grading of
//! thinking-mode completions (GSM8K numbers and short final answers).
//!
//! Direct-answer training follows Qwen3 Thinking Mode Fusion: the user turn
//! ends in `/no_think`, and the assistant prompt prefills an empty
//! `<think>\n\n</think>\n\n` block. That block is part of the prompt, not the
//! loss, so the thinking-mode entry point (`<|im_start|>assistant\n` with no
//! flag) is not trained to skip reasoning. A response that already contains a
//! non-empty think block is left as a thinking-mode example.

/// Empty think block the Qwen3 chat template prefills when thinking is off.
pub const QWEN_EMPTY_THINK: &str = "<think>\n\n</think>\n\n";

/// True when `text` contains a `<think>` block with non-whitespace inside it.
#[must_use]
pub fn has_reasoning_trace(text: &str) -> bool {
    let Some(start) = text.find("<think>") else {
        return false;
    };
    let rest = &text[start + "<think>".len()..];
    let Some(end) = rest.find("</think>") else {
        return false;
    };
    !rest[..end].trim().is_empty()
}

/// Thinking-mode initiation: a non-empty `<think>` body, including a trace
/// that was cut off before `</think>`. An empty prefilled block does not count.
#[must_use]
pub fn opened_nonempty_think(completion: &str) -> bool {
    let text = completion.trim_start();
    let Some(rest) = text.strip_prefix("<think>") else {
        return false;
    };
    let body = rest.split("</think>").next().unwrap_or(rest);
    !body.trim().is_empty()
}

/// User-turn text for one row. Direct answers gain ` /no_think` unless the
/// user already set `/think` or `/no_think`.
#[must_use]
pub fn user_for_mode(user: &str, reasoning: bool) -> String {
    if reasoning || user.contains("/no_think") || user.contains("/think") {
        user.to_string()
    } else {
        format!("{} /no_think", user.trim_end())
    }
}

/// Assistant text as stored in the supervised string. Direct answers are
/// prefixed with the empty think block; reasoning traces are unchanged.
#[must_use]
pub fn assistant_body(assistant: &str) -> String {
    if has_reasoning_trace(assistant) || assistant.starts_with(QWEN_EMPTY_THINK) {
        assistant.to_string()
    } else {
        format!("{QWEN_EMPTY_THINK}{assistant}")
    }
}

/// Serving lane: which assistant-turn prefix a request is shaped for.
///
/// - `codegen` (default): direct-answer format. The user turn ends in
///   `/no_think` and the assistant turn is prefilled with
///   [`QWEN_EMPTY_THINK`], matching QLoRA training.
/// - `hub`: thinking-mode entry point. `/no_think` is stripped and the
///   assistant turn is left open (no prefilled think block).
/// - `agent`: same prefix as `hub`; kept distinct so callers can route by
///   intent without a wire change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lane {
    /// Direct-answer lane (default).
    #[default]
    Codegen,
    /// Thinking-mode lane.
    Hub,
    /// Thinking-mode lane for agent loops.
    Agent,
}

impl Lane {
    /// Accepted wire values, for error text.
    pub const ALLOWED: &'static str = "codegen, hub, agent";

    /// Parse a wire value. Exact lowercase match: no trimming, no case folding.
    /// The error quotes the offending value cut to 32 chars.
    pub fn parse(s: &str) -> Result<Lane, String> {
        match s {
            "codegen" => Ok(Lane::Codegen),
            "hub" => Ok(Lane::Hub),
            "agent" => Ok(Lane::Agent),
            other => {
                let shown: String = other.chars().take(32).collect();
                Err(format!(
                    "unknown lane \"{shown}\": expected one of {}",
                    Lane::ALLOWED
                ))
            }
        }
    }

    /// Wire value; round-trips through [`Lane::parse`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Lane::Codegen => "codegen",
            Lane::Hub => "hub",
            Lane::Agent => "agent",
        }
    }

    /// True for the lanes that leave the assistant turn open (hub, agent).
    #[must_use]
    pub fn is_thinking(self) -> bool {
        !matches!(self, Lane::Codegen)
    }
}

/// User-turn text for a lane. Codegen is [`user_for_mode`] with
/// `reasoning = false`; hub and agent drop every `/no_think` and trailing
/// whitespace, leaving `/think` untouched.
#[must_use]
pub fn user_for_lane(user: &str, lane: Lane) -> String {
    if lane.is_thinking() {
        user.replace("/no_think", "").trim_end().to_string()
    } else {
        user_for_mode(user, false)
    }
}

/// Text that follows `assistant\n` in the prompt: the empty think block for
/// codegen, nothing for hub and agent.
#[must_use]
pub fn assistant_opener_for_lane(lane: Lane) -> &'static str {
    if lane.is_thinking() {
        ""
    } else {
        QWEN_EMPTY_THINK
    }
}

/// Thinking-mode generation prompt. No `/no_think`, no prefilled think block:
/// this is the entry point a base Qwen3 model uses by default.
#[must_use]
pub fn thinking_mode_prompt(question: &str) -> String {
    format!("<|im_start|>user\n{question}<|im_end|>\n<|im_start|>assistant\n")
}

/// Text used to grade a final answer.
///
/// - After the last `</think>` when the think block was closed (normal path).
/// - The open think body when `<think>` was opened but never closed (truncated
///   or unfinished reasoning). Models often already write `\boxed{…}`,
///   `#### N`, or a concluding number inside the trace; treating the body as
///   empty made every unclosed completion an automatic fail even when the
///   answer was present.
/// - The full trimmed completion when no think tag is opened.
#[must_use]
pub fn answer_region(completion: &str) -> &str {
    if let Some(i) = completion.rfind("</think>") {
        return completion[i + "</think>".len()..].trim();
    }
    let trimmed = completion.trim_start();
    if let Some(rest) = trimmed.strip_prefix("<think>") {
        return rest.trim();
    }
    completion.trim()
}

/// Grade a completion against a gold final answer.
///
/// Prefer explicit answer markers over a bare trailing number:
/// 1. `\boxed{...}` body (handles "…\\boxed{8}…5:00 PM" without picking `00`)
/// 2. GSM8K-style `#### <answer>` line
/// 3. Numeric gold → last number in the answer region
/// 4. Last non-empty line (normalized)
#[must_use]
pub fn grade_final_answer(completion: &str, gold: &str) -> bool {
    let region = answer_region(completion);
    let gold_norm = normalize_answer(gold);
    if gold_norm.is_empty() || region.is_empty() {
        return false;
    }
    if let Some(boxed) = extract_boxed(region) {
        if answers_equal(&boxed, &gold_norm) {
            return true;
        }
    }
    if let Some(hash_ans) = extract_hash_answer(region) {
        if answers_equal(&hash_ans, &gold_norm) {
            return true;
        }
    }
    if let Some(gold_n) = parse_number(&gold_norm) {
        return last_number(region).is_some_and(|n| numbers_equal(gold_n, n));
    }
    last_nonempty_line(region).is_some_and(|line| normalize_answer(line) == gold_norm)
}

/// True when a candidate answer string matches normalized gold (numeric or text).
fn answers_equal(candidate: &str, gold_norm: &str) -> bool {
    let cand_norm = normalize_answer(candidate);
    if cand_norm == gold_norm {
        return true;
    }
    match (parse_number(&cand_norm), parse_number(gold_norm)) {
        (Some(a), Some(b)) => numbers_equal(a, b),
        _ => false,
    }
}

/// Body after the last GSM8K `####` marker, if any.
fn extract_hash_answer(s: &str) -> Option<String> {
    let key = "####";
    let i = s.rfind(key)?;
    let rest = s[i + key.len()..].trim();
    let line = rest.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Accuracy and think-initiation rate over `(completion, gold)` pairs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReasoningScore {
    /// Fraction of items whose final answer matched.
    pub accuracy: f64,
    /// Fraction of completions that opened a non-empty think block.
    pub think_initiation_rate: f64,
    /// Number of items scored.
    pub n: usize,
}

/// Score a batch. An empty batch scores 0.
#[must_use]
pub fn score_completions<'a, I>(pairs: I) -> ReasoningScore
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut n = 0usize;
    let mut correct = 0usize;
    let mut thinking = 0usize;
    for (completion, gold) in pairs {
        n += 1;
        if grade_final_answer(completion, gold) {
            correct += 1;
        }
        if opened_nonempty_think(completion) {
            thinking += 1;
        }
    }
    if n == 0 {
        return ReasoningScore {
            accuracy: 0.0,
            think_initiation_rate: 0.0,
            n: 0,
        };
    }
    ReasoningScore {
        accuracy: correct as f64 / n as f64,
        think_initiation_rate: thinking as f64 / n as f64,
        n,
    }
}

fn normalize_answer(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != '$' && *c != ',')
        .collect::<String>()
        .trim_matches(|c| c == '.')
        .to_string()
}

fn last_nonempty_line(s: &str) -> Option<&str> {
    s.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(str::trim)
}

fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty()
        || t.chars()
            .any(|c| !(c.is_ascii_digit() || c == '.' || c == '-'))
    {
        return None;
    }
    t.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// True when the comma at `comma` is a thousands separator: exactly three
/// digits follow it and no further digit after those.
fn is_thousands_group(bytes: &[u8], comma: usize) -> bool {
    let group = bytes.get(comma + 1..comma + 4);
    group.is_some_and(|g| g.iter().all(u8::is_ascii_digit))
        && !bytes.get(comma + 4).is_some_and(u8::is_ascii_digit)
}

fn last_number(s: &str) -> Option<f64> {
    let mut last: Option<f64> = None;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let signed = bytes[i] == b'-'
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_digit()
            && !(i > 0 && bytes[i - 1].is_ascii_alphanumeric());
        if signed {
            i += 1;
        }
        if i < bytes.len() && bytes[i].is_ascii_digit() {
            while i < bytes.len()
                && (bytes[i].is_ascii_digit() || (bytes[i] == b',' && is_thousands_group(bytes, i)))
            {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                let dot = i;
                i += 1;
                if i < bytes.len() && bytes[i].is_ascii_digit() {
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                } else {
                    i = dot;
                }
            }
            let token: String = s[start..i].chars().filter(|c| *c != ',').collect();
            if let Ok(n) = token.parse::<f64>() {
                last = Some(n);
            }
            continue;
        }
        i += 1;
    }
    last
}

fn numbers_equal(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-3
}

/// Body of the last `\boxed{...}`, respecting nested braces. `None` if absent.
fn extract_boxed(s: &str) -> Option<String> {
    let key = "\\boxed{";
    let start = s.rfind(key)? + key.len();
    let mut depth = 1i32;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..start + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_think_block_is_not_a_reasoning_trace() {
        assert!(!has_reasoning_trace("<think>\n\n</think>\n\nfn main() {}"));
        assert!(!opened_nonempty_think("<think>\n\n</think>\n\n42"));
        assert!(has_reasoning_trace("<think>\nplan\n</think>\n\n42"));
        assert!(opened_nonempty_think("<think>\nstill going"));
    }

    #[test]
    fn direct_answers_gain_no_think_and_an_empty_block() {
        assert_eq!(user_for_mode("write it", false), "write it /no_think");
        assert_eq!(user_for_mode("write it /think", false), "write it /think");
        assert_eq!(user_for_mode("q", true), "q");
        assert_eq!(assistant_body("code"), format!("{QWEN_EMPTY_THINK}code"));
        assert_eq!(
            assistant_body("<think>\nplan\n</think>\n\ncode"),
            "<think>\nplan\n</think>\n\ncode"
        );
    }

    #[test]
    fn lane_parse_accepts_exactly_codegen_hub_agent() {
        assert_eq!(Lane::parse("codegen"), Ok(Lane::Codegen));
        assert_eq!(Lane::parse("hub"), Ok(Lane::Hub));
        assert_eq!(Lane::parse("agent"), Ok(Lane::Agent));
        assert_eq!(Lane::default(), Lane::Codegen);
        for bad in ["Hub", "HUB", " hub", "hub ", "", "bogus", "codegen\n"] {
            let err = Lane::parse(bad).unwrap_err();
            for name in ["codegen", "hub", "agent"] {
                assert!(err.contains(name), "{err:?} must name {name}");
            }
        }
        assert_eq!(
            Lane::parse("bogus").unwrap_err(),
            "unknown lane \"bogus\": expected one of codegen, hub, agent"
        );
    }

    #[test]
    fn lane_parse_error_truncates_the_offending_value_on_a_char_boundary() {
        let long = "é".repeat(100);
        let err = Lane::parse(&long).unwrap_err();
        let quoted = "é".repeat(32);
        assert!(
            err.starts_with(&format!("unknown lane \"{quoted}\": ")),
            "{err}"
        );
        assert!(!err.contains(&"é".repeat(33)));
    }

    #[test]
    fn lane_as_str_round_trips_and_only_hub_and_agent_think() {
        for lane in [Lane::Codegen, Lane::Hub, Lane::Agent] {
            assert_eq!(Lane::parse(lane.as_str()), Ok(lane));
        }
        assert!(!Lane::Codegen.is_thinking());
        assert!(Lane::Hub.is_thinking());
        assert!(Lane::Agent.is_thinking());
        assert_eq!(Lane::ALLOWED, "codegen, hub, agent");
    }

    #[test]
    fn user_for_lane_codegen_matches_user_for_mode() {
        assert_eq!(user_for_lane("hi", Lane::Codegen), "hi /no_think");
        assert_eq!(user_for_lane("hi /think", Lane::Codegen), "hi /think");
        assert_eq!(
            user_for_lane("hi /no_think", Lane::Codegen),
            user_for_mode("hi /no_think", false)
        );
    }

    #[test]
    fn user_for_lane_hub_and_agent_strip_no_think_and_keep_think() {
        assert_eq!(user_for_lane("hi /no_think", Lane::Hub), "hi");
        assert_eq!(user_for_lane("a /no_think b /no_think", Lane::Hub), "a  b");
        assert_eq!(user_for_lane("hi /think", Lane::Agent), "hi /think");
        assert_eq!(user_for_lane("hi  ", Lane::Hub), "hi");
        for u in ["hi", "hi /no_think", "hi /think", "x\n"] {
            assert_eq!(user_for_lane(u, Lane::Hub), user_for_lane(u, Lane::Agent));
        }
    }

    #[test]
    fn assistant_opener_for_lane_prefills_only_for_codegen() {
        assert_eq!(assistant_opener_for_lane(Lane::Codegen), QWEN_EMPTY_THINK);
        assert_eq!(assistant_opener_for_lane(Lane::Hub), "");
        assert_eq!(assistant_opener_for_lane(Lane::Agent), "");
    }

    #[test]
    fn gsm8k_grades_the_last_number_after_the_trace() {
        let completion = "<think>\n16-3-4=9, 9*2=18\n</think>\n\nShe makes $18.\n#### 18";
        assert!(grade_final_answer(completion, "18"));
        assert!(!grade_final_answer(completion, "16"));
        assert!(grade_final_answer("total is 1,000", "1000"));
    }

    #[test]
    fn boxed_and_last_line_grade_non_numeric_answers() {
        let completion = "<think>\nr=3\n</think>\n\n\\boxed{\\left(3, \\frac{\\pi}{2}\\right)}";
        assert!(grade_final_answer(
            completion,
            r"\left(3, \frac{\pi}{2}\right)"
        ));
        assert!(grade_final_answer("therefore\n\np - q", "p - q"));
        assert!(!grade_final_answer(
            "<think>\nnope\n</think>\n\n\\boxed{1}",
            "2"
        ));
    }

    #[test]
    fn score_counts_accuracy_and_think_initiation() {
        let pairs = [("<think>\n1\n</think>\n\n5", "5"), ("just 5", "5")];
        let s = score_completions(pairs.iter().copied());
        assert_eq!(s.n, 2);
        assert!((s.accuracy - 1.0).abs() < f64::EPSILON);
        assert!((s.think_initiation_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn assistant_body_does_not_double_the_empty_think_block() {
        let body = assistant_body(&format!("{QWEN_EMPTY_THINK}x"));
        assert!(body.starts_with(QWEN_EMPTY_THINK));
        assert!(!body[QWEN_EMPTY_THINK.len()..].starts_with(QWEN_EMPTY_THINK));
        assert_eq!(body.matches("<think>").count(), 1, "{body:?}");
    }

    #[test]
    fn codegen_user_with_an_existing_flag_gets_no_second_flag() {
        for user in ["a /no_think", "a /think"] {
            assert_eq!(user_for_mode(user, false), user);
        }
    }

    #[test]
    fn unterminated_think_grades_from_body_when_answer_present() {
        // Unclosed think: region is the body, not empty.
        assert_eq!(answer_region("<think>\n... so 42"), "... so 42");
        assert_eq!(
            answer_region("  \n<think>\nstill going 42"),
            "still going 42"
        );
        assert!(grade_final_answer("<think>\n... so 42", "42"));
        assert!(grade_final_answer("<think>\nplan\n\\boxed{42}", "42"));
        assert!(grade_final_answer("<think>\ncalc\n#### 18\n", "18"));
        // Still fail when the body has no recoverable answer for the gold.
        assert!(!grade_final_answer("<think>\nplan only, no digits", "42"));
        assert!(!grade_final_answer(
            "<think>\nplan\nnot the gold string",
            "answer"
        ));
    }

    #[test]
    fn closed_think_and_plain_completions_keep_their_answer_region() {
        assert_eq!(answer_region("<think>\nplan\n</think>\n\n 42 \n"), "42");
        assert_eq!(answer_region("<think>a</think> x <think>b</think> 7"), "7");
        assert_eq!(answer_region("  just 42  "), "just 42");
        assert!(grade_final_answer("<think>\nplan\n</think>\n\n42", "42"));
        assert!(grade_final_answer("so 42", "42"));
    }

    #[test]
    fn boxed_wins_over_trailing_time_like_numbers() {
        // Last-number alone would pick `00` from `5:00`; boxed must win for gold 8.
        let completion =
            "<think>\n8cm\n</think>\nThe candle is \\boxed{8} cm shorter after 1:00 PM to 5:00 PM.";
        assert!(grade_final_answer(completion, "8"));
        // Wrong golds must not match the boxed answer.
        assert!(!grade_final_answer(completion, "5"));
        assert!(!grade_final_answer(completion, "1"));
        assert!(!grade_final_answer(completion, "16"));
    }

    #[test]
    fn last_number_does_not_glue_comma_lists() {
        assert_eq!(last_number("the answers are 1,2,3"), Some(3.0));
        assert_eq!(last_number("5,6"), Some(6.0));
        assert_eq!(last_number("1,2345"), Some(2345.0));
        assert!(!grade_final_answer("the answers are 1,2,3", "123"));
        assert!(grade_final_answer("the answers are 1,2,3", "3"));
    }

    #[test]
    fn last_number_keeps_thousands_separators() {
        assert_eq!(last_number("1,234"), Some(1234.0));
        assert_eq!(last_number("total 1,234,567 units"), Some(1_234_567.0));
        assert_eq!(last_number("1,234.5"), Some(1234.5));
        assert!(grade_final_answer("it costs $1,234", "1234"));
    }

    #[test]
    fn last_number_hyphen_inside_a_token_is_not_a_sign() {
        assert_eq!(last_number("2024-05-01"), Some(1.0));
        assert_eq!(last_number("3-4"), Some(4.0));
        assert_eq!(last_number("x-5"), Some(5.0));
        assert!(!grade_final_answer("3-4", "-4"));
        assert!(grade_final_answer("3-4", "4"));
    }

    #[test]
    fn last_number_leading_minus_is_still_a_sign() {
        assert_eq!(last_number("-5"), Some(-5.0));
        assert_eq!(last_number("temp is -5"), Some(-5.0));
        assert_eq!(last_number("(-5)"), Some(-5.0));
        assert!(grade_final_answer("the result is -12", "-12"));
    }
}
