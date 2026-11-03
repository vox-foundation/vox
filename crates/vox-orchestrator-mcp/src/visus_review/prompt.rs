//! System prompt for adversarial GUI screenshot review.

/// Cache-busting prompt versions.
pub const DEFECT_PROMPT_VERSION: &str = "2026-09-30.defect1";
pub const UX_PROMPT_VERSION: &str = "2026-09-30.ux1";
pub const PROMPT_VERSION: &str = UX_PROMPT_VERSION;

pub const RUBRIC: &str = r#"
Review this desktop-app surface SCREENSHOT adversarially against these principles. Hunt for real defects; do not flatter.
1 Visual hierarchy: exactly one primary action; scale/weight/contrast rank elements (#65-73).
2 Tokens/consistency: consistent color meaning, spacing rhythm, iconography; no ad-hoc visual noise (#20-24,#247-255).
3 Typography & spacing: readable measure/line-height; aligned to a spacing scale (#74-98).
4 Loading/empty/error: deliberate states, not silent blanks; errors actionable (#1.1,#47-52,#163-168).
5 Accessibility (visual): text contrast >=4.5:1, UI/focus >=3:1; targets >=24px; icon-only controls look labeled (#178-211).
6 Affordance & feedback: interactive elements look interactive; current location obvious (#132-162).
7 Minimalism: progressive disclosure; remove clutter (#42-46).
8 Error prevention: destructive actions visually distinct/guarded (#25-31).
"#;

pub fn system_prompt() -> String {
    format!(
        "You are a senior product-design reviewer performing an ADVERSARIAL critique of a desktop \
GUI surface screenshot. Be specific and skeptical: every finding must cite a visible region and a \
principle number.\n\nRUBRIC:\n{RUBRIC}\n\nOUTPUT CONTRACT: Respond with ONLY a single JSON object, no \
prose, no markdown fence:\n{{\n  \"score\": <integer 0-100, lower = more/worse defects>,\n  \"verdict\": \
\"pass\" | \"pass_with_notes\" | \"fail\",\n  \"findings\": [ {{ \"principle\": \"#NN short-name\", \
\"severity\": \"low\"|\"medium\"|\"high\", \"region\": \"<where on screen>\", \"critique\": \"<what is wrong and why>\" }} ]\n}}\n\
If the surface is clean, return an empty findings array, verdict \"pass\", score >=90."
    )
}

pub fn user_prompt(view_key: &str) -> String {
    format!(
        "Surface: '{view_key}'. Critique the attached screenshot per the rubric and output the JSON verdict."
    )
}

/// Defect-hunting rubric for review-bundle analysis (vs RUBRIC's general
/// design quality): concrete rendering DEFECTS the capture matrix exists
/// to catch.
pub const DEFECT_RUBRIC: &str = r#"
Hunt for concrete rendering DEFECTS in this screenshot. Report only what is visibly wrong:
A occlusion: elements overlapping/covering each other (menus over content they shouldn't cover, HUD over controls, z- order fights, tooltips/popovers clipped by containers).
B clipping/truncation: text or controls cut off mid-glyph, ellipsis where full text matters, content escaping its card/panel, horizontal scrollbars on the page body.
C icons: blank or missing icon slots, zero-size glyphs, misaligned or mismatched icon sizes.
D error leakage: raw error text, exception stack traces, 'undefined'/'NaN'/'[object Object]' visible in UI copy.
E blank regions: panels that render empty where the dense mock data should appear.
F layout breakage: overlapping columns, collapsed rows, controls pushed off-screen — especially at the compact viewport.
G contrast/legibility: text below readable contrast against its actual background.
"#;

pub fn defect_system_prompt() -> String {
    format!(
        "You are a rendering-defect detector for a desktop GUI screenshot. Programmatic scan \
results (axe-core, console errors, icon audit, overflow measurements) are provided — correlate \
with them, then find what they CANNOT see (visual occlusion, clipping, blank panels, error-text \
leakage).\n\nDEFECT RUBRIC:\n{DEFECT_RUBRIC}\n\nOUTPUT CONTRACT: Respond with ONLY a single JSON \
object, no prose, no markdown fence:\n{{\n  \"score\": <integer 0-100, 100 = defect-free>,\n  \
\"verdict\": \"pass\" | \"pass_with_notes\" | \"fail\",\n  \"defects\": [ {{ \"severity\": \
\"critical\"|\"major\"|\"minor\", \"kind\": \"occlusion\"|\"clipping\"|\"icon\"|\"error-leak\"|\
\"blank\"|\"layout\"|\"contrast\"|\"other\", \"description\": \"<what is wrong>\", \"location\": \
\"<where on screen>\" }} ]\n}}\nIf clean, return an empty defects array, verdict \"pass\", score >= 95."
    )
}

pub fn defect_user_prompt(e: &crate::visus_review::bundle::BundleEntry) -> String {
    // Noise policy: only serious/critical axe violations reach the model;
    // moderate ones stay in the JSONL for Phase D triage.
    let axe_hot: Vec<&serde_json::Value> = e
        .axe_violations
        .iter()
        .filter(|v| matches!(v["impact"].as_str(), Some("serious") | Some("critical")))
        .collect();
    format!(
        "Capture: surface '{surface}', state '{state}', viewport '{viewport}', browser '{browser}', theme '{theme}'.\n\
Programmatic findings for THIS capture (correlate, do not merely repeat):\n\
- axe (serious/critical): {axe}\n- console errors: {console:?}\n- page errors: {page:?}\n\
- icon issues: {icons}\n- overflow: {overflow}\n- state setup ok: {ok} {err}\n\
Analyze the attached screenshot per the defect rubric and output the JSON verdict.",
        surface = e.surface,
        state = e.state,
        viewport = e.viewport,
        browser = e.browser,
        theme = e.theme,
        axe = serde_json::to_string(&axe_hot).unwrap_or_default(),
        console = e.console_errors,
        page = e.page_errors,
        icons = serde_json::to_string(&e.icon_issues).unwrap_or_default(),
        overflow = e.overflow,
        ok = e.state_ok,
        err = if e.state_error.is_empty() {
            String::new()
        } else {
            format!("(setup error: {})", e.state_error)
        },
    )
}

/// UX clarity, accessibility, and capability audit rubric.
pub const UX_CLARITY_RUBRIC: &str = r#"
Audit this desktop GUI surface screenshot for UX clarity, terminology, affordances, and capability gaps across four pillars:

1. Jargon & Acronym Retirement:
Replace internal codenames, developer jargon, and unexpanded acronyms with plain, self-evident user-facing language.
Enforce Vox internal glossary replacements:
- Ludus -> Activity Alerts
- Mercatus -> Marketplace / Compute Store
- DEI -> Oversight / Approvals
- Socrates -> Architecture Reasoner
- CAS -> Cache / Content Store
- AXTree -> Accessibility Tree

2. Readability & Visual Hierarchy:
Evaluate scannability, primary action clarity, and cognitive load. Ensure the surface purpose is immediately understandable, primary actions are unmistakable, visual hierarchy is well-structured, typography and spacing are clean, and visual noise is minimized.

3. Controllability & Affordance:
Evaluate interactive controls and affordances. Interactive controls must look clickable with obvious states (selected, disabled, hover, active). Surfaces with collections or lists must provide obvious filters/states, search, and sorting controls with predictable feedback.

4. Missing Features & Capabilities:
Identify high-value tools, actions, shortcuts, batch operations, or contextual controls that are noticeably absent and would make the surface significantly more powerful, complete, and useful for users.
"#;

pub fn ux_system_prompt() -> String {
    format!(
        "You are an expert product design and user experience auditor performing a UX clarity \
and capability audit of a desktop GUI surface screenshot.\n\n\
UX CLARITY RUBRIC:\n{UX_CLARITY_RUBRIC}\n\n\
OUTPUT CONTRACT: Respond with ONLY a single JSON object, no prose, no markdown fence:\n\
{{\n  \
  \"score\": <integer 0-100, 100 = optimal UX clarity and completeness>,\n  \
  \"verdict\": \"pass\" | \"pass_with_notes\" | \"fail\",\n  \
  \"jargon_retirements\": [\n    \
    {{\n      \
      \"current_term\": \"<cryptic term or acronym in UI>\",\n      \
      \"replacement\": \"<plain English replacement per glossary>\",\n      \
      \"rationale\": \"<why current term confuses users>\"\n    \
    }}\n  \
  ],\n  \
  \"readability_issues\": [\n    \
    {{\n      \
      \"region\": \"<where on screen>\",\n      \
      \"issue\": \"<what hurts scannability, primary action clarity, or cognitive load>\",\n      \
      \"recommendation\": \"<concrete design recommendation>\"\n    \
    }}\n  \
  ],\n  \
  \"controllability_gaps\": [\n    \
    {{\n      \
      \"control\": \"<affected control, filter, or interaction>\",\n      \
      \"issue\": \"<unclear affordance, missing filter/sort, or weak feedback>\",\n      \
      \"fix\": \"<actionable fix to improve interactivity and control>\"\n    \
    }}\n  \
  ],\n  \
  \"missing_capabilities\": [\n    \
    {{\n      \
      \"feature_name\": \"<name of missing feature or capability>\",\n      \
      \"user_benefit\": \"<how this makes surface more powerful or productive>\",\n      \
      \"implementation_hint\": \"<suggested UI placement or mechanism>\"\n    \
    }}\n  \
  ]\n\
}}\n\
If the surface is clean, clear, and complete, return empty arrays, verdict \"pass\", score >= 90."
    )
}

pub fn ux_user_prompt(e: &crate::visus_review::bundle::BundleEntry) -> String {
    let err = if e.state_error.is_empty() {
        String::new()
    } else {
        format!(" (setup error: {})", e.state_error)
    };
    format!(
        "Capture: surface '{surface}', state '{state}', viewport '{viewport}', browser '{browser}', theme '{theme}'.{err}\n\
Audit the attached screenshot per the UX clarity rubric and output the JSON report.",
        surface = e.surface,
        state = e.state,
        viewport = e.viewport,
        browser = e.browser,
        theme = e.theme,
        err = err,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_version_is_set() {
        assert!(!PROMPT_VERSION.trim().is_empty());
    }
    #[test]
    fn prompt_states_json_only_contract() {
        let p = system_prompt();
        assert!(p.contains("ONLY a single JSON object"));
        assert!(p.contains("\"findings\""));
        assert!(RUBRIC.contains("Visual hierarchy"));
    }
    #[test]
    fn defect_rubric_names_the_hunted_classes() {
        for needle in ["occlusion", "clipp", "icon", "error text", "blank", "z-"] {
            assert!(
                DEFECT_RUBRIC.to_lowercase().contains(needle),
                "missing: {needle}"
            );
        }
    }
    #[test]
    fn defect_prompts_carry_context_and_json_contract() {
        let e = crate::visus_review::bundle::BundleEntry {
            id: "chat--model-picker-open--compact--firefox".into(),
            surface: "chat".into(),
            state: "model-picker-open".into(),
            viewport: "compact".into(),
            browser: "firefox".into(),
            theme: "default".into(),
            file: "f.png".into(),
            sha256: "s".into(),
            state_ok: true,
            state_error: String::new(),
            axe_violations: vec![serde_json::json!({"id":"color-contrast","impact":"serious"})],
            console_errors: vec!["error: boom".into()],
            console_warnings: vec![],
            page_errors: vec![],
            icon_issues: vec![],
            overflow: serde_json::json!({"bodyHorizontalOverflowPx": 40}),
            capture_ms: 0,
            captured_at: "t".into(),
        };
        let up = defect_user_prompt(&e);
        assert!(
            up.contains("chat")
                && up.contains("model-picker-open")
                && up.contains("compact")
                && up.contains("firefox")
        );
        assert!(up.contains("color-contrast") && up.contains("boom") && up.contains("40"));
        let sp = defect_system_prompt();
        assert!(sp.contains("ONLY a single JSON object"));
        assert!(sp.contains("\"defects\""));
    }
    #[test]
    fn prompt_version_bumped_for_defect_rubric() {
        assert!(PROMPT_VERSION >= "2026-07-18.1");
    }
    #[test]
    fn defect_user_prompt_forwards_only_serious_and_critical_axe() {
        let mut e = crate::visus_review::bundle::BundleEntry {
            id: "x".into(),
            surface: "x".into(),
            state: "default".into(),
            viewport: "wide".into(),
            browser: "chromium".into(),
            theme: "default".into(),
            file: "x.png".into(),
            sha256: "s".into(),
            state_ok: true,
            state_error: String::new(),
            axe_violations: vec![
                serde_json::json!({"id":"region","impact":"moderate"}),
                serde_json::json!({"id":"color-contrast","impact":"serious"}),
            ],
            console_errors: vec![],
            console_warnings: vec![],
            page_errors: vec![],
            icon_issues: vec![],
            overflow: serde_json::Value::Null,
            capture_ms: 0,
            captured_at: "t".into(),
        };
        let up = defect_user_prompt(&e);
        assert!(up.contains("color-contrast"));
        assert!(
            !up.contains("\"region\""),
            "moderate violations stay in the JSONL, out of the prompt"
        );
        e.axe_violations.clear();
        let _ = defect_user_prompt(&e); // no panic on empty
    }
    #[test]
    fn prompt_version_is_current_ux_bump() {
        assert_eq!(PROMPT_VERSION, "2026-09-30.ux1");
    }
    #[test]
    fn ux_system_prompt_contains_rubric_glossary_and_schema() {
        let sp = ux_system_prompt();
        // 4 pillars & rubric keywords
        assert!(sp.contains("Jargon & Acronym Retirement"));
        assert!(sp.contains("Readability & Visual Hierarchy"));
        assert!(sp.contains("Controllability & Affordance"));
        assert!(sp.contains("Missing Features & Capabilities"));
        assert!(sp.contains("scannability"));
        assert!(sp.contains("primary action clarity"));
        assert!(sp.contains("cognitive load"));
        assert!(sp.contains("interactive controls"));
        assert!(sp.contains("obvious filters/states"));

        // Glossary terms
        assert!(sp.contains("Ludus") && sp.contains("Activity Alerts"));
        assert!(sp.contains("Mercatus") && sp.contains("Marketplace / Compute Store"));
        assert!(sp.contains("DEI") && sp.contains("Oversight / Approvals"));
        assert!(sp.contains("Socrates") && sp.contains("Architecture Reasoner"));
        assert!(sp.contains("CAS") && sp.contains("Cache / Content Store"));
        assert!(sp.contains("AXTree") && sp.contains("Accessibility Tree"));

        // JSON schema contract matching UxAuditReport
        assert!(sp.contains("ONLY a single JSON object"));
        assert!(sp.contains("\"score\""));
        assert!(sp.contains("\"verdict\""));
        assert!(sp.contains("\"jargon_retirements\""));
        assert!(sp.contains("\"current_term\""));
        assert!(sp.contains("\"replacement\""));
        assert!(sp.contains("\"rationale\""));
        assert!(sp.contains("\"readability_issues\""));
        assert!(sp.contains("\"region\""));
        assert!(sp.contains("\"issue\""));
        assert!(sp.contains("\"recommendation\""));
        assert!(sp.contains("\"controllability_gaps\""));
        assert!(sp.contains("\"control\""));
        assert!(sp.contains("\"fix\""));
        assert!(sp.contains("\"missing_capabilities\""));
        assert!(sp.contains("\"feature_name\""));
        assert!(sp.contains("\"user_benefit\""));
        assert!(sp.contains("\"implementation_hint\""));
    }
    #[test]
    fn ux_user_prompt_formats_bundle_entry() {
        let e = crate::visus_review::bundle::BundleEntry {
            id: "settings--dark--wide--chromium".into(),
            surface: "settings".into(),
            state: "dark".into(),
            viewport: "wide".into(),
            browser: "chromium".into(),
            theme: "dark".into(),
            file: "s.png".into(),
            sha256: "hash123".into(),
            state_ok: true,
            state_error: String::new(),
            axe_violations: vec![],
            console_errors: vec![],
            console_warnings: vec![],
            page_errors: vec![],
            icon_issues: vec![],
            overflow: serde_json::Value::Null,
            capture_ms: 120,
            captured_at: "now".into(),
        };
        let up = ux_user_prompt(&e);
        assert!(up.contains("surface 'settings'"));
        assert!(up.contains("state 'dark'"));
        assert!(up.contains("viewport 'wide'"));
        assert!(up.contains("browser 'chromium'"));
        assert!(up.contains("theme 'dark'"));
        assert!(up.contains("UX clarity rubric"));
    }
}
