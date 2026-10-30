//! Message construction for the diagram-authoring eval: one frozen template serves both arms, which
//! differ only in the `{LANG}`, `{FENCE}` and `{VALIDATOR}` slots and in the system message (the
//! arm's grammar card). The task prompt and every repair feedback are inserted verbatim, never
//! truncated, and never re-scanned for slots.

use anyhow::{Result, bail, ensure};
use vox_actor_runtime::llm::LlmChatMessage;

use super::Arm;

/// Slot names a template may use.
const SLOTS: [&str; 5] = ["LANG", "FENCE", "VALIDATOR", "PROMPT", "FEEDBACK"];

/// The two sections of `prompt-template.txt`: the first user message and the repair user message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub user: String,
    pub repair: String,
}

/// The per-arm slot values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmSlots {
    pub lang: &'static str,
    pub fence: &'static str,
    pub validator: &'static str,
}

/// Slot values of one arm.
#[must_use]
pub fn arm_slots(arm: Arm) -> ArmSlots {
    match arm {
        Arm::Voxdiagram => ArmSlots {
            lang: "VoxDiagram",
            fence: "vox-diagram",
            validator: "validator",
        },
        Arm::Mermaid => ArmSlots {
            lang: "Mermaid",
            fence: "mermaid",
            validator: "parser",
        },
    }
}

/// The `{NAME}` tokens of `text` (uppercase letters and `_`), in order.
fn slot_names(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        if let Some(close) = rest.find('}') {
            let name = &rest[..close];
            if !name.is_empty() && name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
                out.push(name);
            }
        }
    }
    out
}

/// Split the template into its `[user]` and `[repair]` sections; refuse a missing, repeated or
/// unknown section, an unknown `{SLOT}`, and a section without its content slot.
pub fn parse_template(text: &str) -> Result<Template> {
    let mut sections: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in text.lines() {
        match line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            Some(name) => {
                ensure!(
                    matches!(name, "user" | "repair"),
                    "unknown template section [{name}]"
                );
                ensure!(
                    !sections.iter().any(|(n, _)| *n == name),
                    "template section [{name}] repeated"
                );
                sections.push((name, Vec::new()));
            }
            None => match sections.last_mut() {
                Some((_, body)) => body.push(line),
                None => ensure!(
                    line.trim().is_empty(),
                    "text before the first template section"
                ),
            },
        }
    }
    let section = |name: &str| {
        sections
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, body)| body.join("\n").trim_matches('\n').to_string())
    };
    let (Some(user), Some(repair)) = (section("user"), section("repair")) else {
        bail!("template needs both a [user] and a [repair] section");
    };
    for (section, text, needed) in [("user", &user, "PROMPT"), ("repair", &repair, "FEEDBACK")] {
        let names = slot_names(text);
        if let Some(bad) = names.iter().find(|n| !SLOTS.contains(n)) {
            bail!("unknown slot {{{bad}}} in [{section}]");
        }
        ensure!(names.contains(&needed), "[{section}] lacks {{{needed}}}");
    }
    Ok(Template { user, repair })
}

/// Replace the template's own slots in one pass; inserted values are never re-scanned.
fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after.find('}').and_then(|close| {
            let name = &after[..close];
            values
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v, close))
        });
        match value {
            Some((v, close)) => {
                out.push_str(v);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn message(role: &str, content: String) -> LlmChatMessage {
    LlmChatMessage {
        role: role.to_string(),
        content,
        ..Default::default()
    }
}

/// System = the arm's card; then the first user message; then, per earlier turn, the raw reply
/// verbatim as the assistant message and the repair message carrying that turn's feedback verbatim.
#[must_use]
pub fn render_messages(
    template: &Template,
    card: &str,
    arm: Arm,
    task_prompt: &str,
    history: &[(String, String)],
) -> Vec<LlmChatMessage> {
    let s = arm_slots(arm);
    let mut out = vec![
        message("system", card.to_string()),
        message(
            "user",
            fill(
                &template.user,
                &[
                    ("LANG", s.lang),
                    ("FENCE", s.fence),
                    ("PROMPT", task_prompt),
                ],
            ),
        ),
    ];
    for (reply, feedback) in history {
        out.push(message("assistant", reply.clone()));
        let values = [
            ("LANG", s.lang),
            ("FENCE", s.fence),
            ("VALIDATOR", s.validator),
            ("FEEDBACK", feedback.as_str()),
        ];
        out.push(message("user", fill(&template.repair, &values)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "[user]\nDraw this as a {LANG} diagram. {PROMPT}\nReply with only one fenced {FENCE} block.\n\n[repair]\nThe {LANG} {VALIDATOR} rejected it:\n{FEEDBACK}\nReply with only the corrected diagram in one fenced {FENCE} block.\n";

    fn roles(m: &[LlmChatMessage]) -> Vec<&str> {
        m.iter().map(|m| m.role.as_str()).collect()
    }

    #[test]
    fn parse_template_splits_the_two_sections() {
        let t = parse_template(TEMPLATE).unwrap();
        assert_eq!(
            t.user,
            "Draw this as a {LANG} diagram. {PROMPT}\nReply with only one fenced {FENCE} block."
        );
        assert!(
            t.repair
                .starts_with("The {LANG} {VALIDATOR} rejected it:\n{FEEDBACK}\n")
        );
        assert!(!t.repair.ends_with('\n'));
    }

    #[test]
    fn parse_template_refuses_missing_sections_and_unknown_slots() {
        let no_repair = TEMPLATE.split("[repair]").next().unwrap();
        assert!(parse_template(no_repair).is_err());
        let err = parse_template(&TEMPLATE.replace("{VALIDATOR}", "{MODEL}")).unwrap_err();
        assert!(err.to_string().contains("MODEL"), "{err}");
        assert!(parse_template(&TEMPLATE.replace("{PROMPT}", "")).is_err());
        assert!(parse_template(&TEMPLATE.replace("{FEEDBACK}", "")).is_err());
        assert!(parse_template(&TEMPLATE.replace("[repair]", "[fix]")).is_err());
        assert!(parse_template(&format!("{TEMPLATE}[user]\n{{PROMPT}}\n")).is_err());
        assert!(parse_template(&format!("preamble\n{TEMPLATE}")).is_err());
    }

    #[test]
    fn render_first_pass_is_card_then_user_message() {
        let t = parse_template(TEMPLATE).unwrap();
        let m = render_messages(&t, "CARD", Arm::Mermaid, "Draw \"A\" {LANG}.", &[]);
        assert_eq!(roles(&m), ["system", "user"]);
        assert_eq!(m[0].content, "CARD");
        assert_eq!(
            m[1].content,
            "Draw this as a Mermaid diagram. Draw \"A\" {LANG}.\nReply with only one fenced mermaid block."
        );
    }

    #[test]
    fn render_repair_turns_carry_reply_and_feedback_verbatim() {
        let t = parse_template(TEMPLATE).unwrap();
        let feedback = format!("{{PROMPT}} {}", "x".repeat(20_000));
        let history = vec![
            ("```vox-diagram\n{FENCE}\n```".to_string(), feedback.clone()),
            ("second".to_string(), "[]".to_string()),
        ];
        let m = render_messages(&t, "CARD", Arm::Voxdiagram, "p", &history);
        assert_eq!(
            roles(&m),
            ["system", "user", "assistant", "user", "assistant", "user"]
        );
        assert_eq!(m[2].content, history[0].0);
        assert_eq!(
            m[3].content,
            format!(
                "The VoxDiagram validator rejected it:\n{feedback}\nReply with only the corrected diagram in one fenced vox-diagram block."
            )
        );
        assert_eq!(m[4].content, "second");
        assert!(m[5].content.contains("\n[]\n"));
    }

    #[test]
    fn arms_differ_only_in_their_slots_and_card() {
        let t = parse_template(TEMPLATE).unwrap();
        let history = vec![("reply".to_string(), "line 2: bad".to_string())];
        let masked = |arm: Arm, card: &str| -> Vec<(String, String)> {
            let s = arm_slots(arm);
            render_messages(&t, card, arm, "Draw \"x\".", &history)
                .into_iter()
                .map(|m| {
                    let c = m
                        .content
                        .replace(card, "<CARD>")
                        .replace(s.fence, "<FENCE>")
                        .replace(s.lang, "<LANG>")
                        .replace(s.validator, "<VALIDATOR>");
                    (m.role, c)
                })
                .collect()
        };
        let vd = masked(Arm::Voxdiagram, "VD CARD");
        assert_eq!(vd, masked(Arm::Mermaid, "MERMAID CARD"));
        assert!(vd.iter().any(|(_, c)| c.contains("<LANG>")));
        assert!(vd.iter().any(|(_, c)| c.contains("<VALIDATOR>")));
        assert_ne!(arm_slots(Arm::Voxdiagram), arm_slots(Arm::Mermaid));
    }
}
