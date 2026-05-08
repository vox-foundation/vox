//! SVG sprite generation for gamification companions and agents.
//!
//! Produces inline SVG fragments driven by pose/mood with no external runtime.
//! Every path is deterministic; AI generation is an optional enhancement layer
//! added in `ai.rs`.

use crate::companion::Mood;

// ─── Pose ─────────────────────────────────────────────────

/// Visual pose for an agent sprite, mapped from activity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPose {
    /// Agent is stationary with no active task.
    Idle,
    /// Agent is actively writing or editing code.
    Working,
    /// Agent is analyzing or planning.
    Thinking,
    /// Agent just completed a task successfully.
    Celebrating,
    /// Agent has used up its energy budget.
    Exhausted,
    /// Agent needs user input or encountered an error.
    Alert,
}

impl AgentPose {
    /// Derive a pose from an activity string coming from the orchestrator.
    pub fn from_activity_str(activity: &str) -> Self {
        match activity {
            "writing" | "coding" | "editing" => AgentPose::Working,
            "thinking" | "planning" | "reviewing" => AgentPose::Thinking,
            "celebrating" | "success" | "task_completed" => AgentPose::Celebrating,
            "exhausted" | "tired" => AgentPose::Exhausted,
            "alert" | "error" | "needs_input" => AgentPose::Alert,
            _ => AgentPose::Idle,
        }
    }

    /// CSS class name for webview styling.
    pub fn css_class(&self) -> &'static str {
        match self {
            AgentPose::Idle => "pose-idle",
            AgentPose::Working => "pose-working",
            AgentPose::Thinking => "pose-thinking",
            AgentPose::Celebrating => "pose-celebrating",
            AgentPose::Exhausted => "pose-exhausted",
            AgentPose::Alert => "pose-alert",
        }
    }

    /// Derive pose from a `Mood` value.
    pub fn from_mood(mood: Mood) -> Self {
        match mood {
            Mood::Happy => AgentPose::Celebrating,
            Mood::Excited => AgentPose::Working,
            Mood::Neutral => AgentPose::Idle,
            Mood::Sad => AgentPose::Exhausted,
            Mood::Tired => AgentPose::Exhausted,
        }
    }
}

// ─── SvgSprite ────────────────────────────────────────────

/// A generated SVG sprite ready to embed in HTML.
#[derive(Debug, Clone)]
pub struct SvgSprite {
    /// Inline `<svg …>…</svg>` body.
    pub svg_body: String,
    /// Stable character identifier (0-7).
    pub character_id: u8,
    /// Pose used for this render.
    pub pose: AgentPose,
}

// ─── Character palette ────────────────────────────────────

/// Eight deterministic character archetypes (Roman theme).
/// Fields: (name, body_color, accent_color, helmet_color, emblem_svg)
/// emblem_svg is appended inside the character's body group for visual differentiation.
const CHARACTERS: &[(&str, &str, &str, &str, &str)] = &[
    // 0 — Centurion: carries a gladius (sword) on the belt
    (
        "Centurion",
        "#C0A060",
        "#8B0000",
        "#D4AF37",
        "<line x1='-2' y1='4' x2='-2' y2='16' stroke='#D4AF37' stroke-width='2'/><polygon points='-4,4 0,4 -2,1' fill='#D4AF37'/>",
    ),
    // 1 — Architectus: holds a blueprint scroll
    (
        "Architectus",
        "#6080C0",
        "#00008B",
        "#A0B4D4",
        "<rect x='3' y='2' width='8' height='10' rx='1' fill='white' opacity='0.7'/><line x1='5' y1='5' x2='9' y2='5' stroke='#00008B' stroke-width='0.8'/><line x1='5' y1='7' x2='9' y2='7' stroke='#00008B' stroke-width='0.8'/><line x1='5' y1='9' x2='9' y2='9' stroke='#00008B' stroke-width='0.8'/>",
    ),
    // 2 — Scriba: carries a wax tablet and stylus
    (
        "Scriba",
        "#60A060",
        "#006400",
        "#90D490",
        "<rect x='-10' y='2' width='7' height='9' rx='1' fill='#8B6914' opacity='0.85'/><line x1='-9' y1='5' x2='-5' y2='5' stroke='#90D490' stroke-width='0.7'/><line x1='-9' y1='7' x2='-5' y2='7' stroke='#90D490' stroke-width='0.7'/>",
    ),
    // 3 — Legatus: bears a shield (scutum) on the left
    (
        "Legatus",
        "#C06060",
        "#800000",
        "#D49090",
        "<rect x='-16' y='0' width='7' height='11' rx='2' fill='#8B0000' opacity='0.85'/><line x1='-12' y1='0' x2='-12' y2='11' stroke='#D49090' stroke-width='1'/><line x1='-16' y1='5' x2='-9' y2='5' stroke='#D49090' stroke-width='1'/>",
    ),
    // 4 — Tribunus: wears a laurel wreath indicator above head
    (
        "Tribunus",
        "#9060C0",
        "#4B0082",
        "#C490D4",
        "<path d='M-8 -30 Q0 -38 8 -30' stroke='#C8B400' stroke-width='2' fill='none'/><circle cx='-8' cy='-30' r='2' fill='#C8B400'/><circle cx='8' cy='-30' r='2' fill='#C8B400'/>",
    ),
    // 5 — Optio: holds a telescope/scrying lens to the eye
    (
        "Optio",
        "#60C0C0",
        "#006060",
        "#90D4D4",
        "<circle cx='7' cy='-16' r='5' stroke='#90D4D4' stroke-width='1.5' fill='none'/><circle cx='7' cy='-16' r='2' fill='#90D4D4' opacity='0.4'/><line x1='7' y1='-11' x2='7' y2='0' stroke='#90D4D4' stroke-width='1.5'/>",
    ),
    // 6 — Signifer: carries a signum standard (pole with disc)
    (
        "Signifer",
        "#C09060",
        "#804000",
        "#D4B490",
        "<line x1='8' y1='-30' x2='8' y2='18' stroke='#D4B490' stroke-width='2'/><circle cx='8' cy='-30' r='4' fill='#D4B490'/><line x1='4' y1='-24' x2='12' y2='-24' stroke='#D4B490' stroke-width='1.5'/>",
    ),
    // 7 — Praetor: wears a cape (curved accent behind body)
    (
        "Praetor",
        "#A0A0A0",
        "#404040",
        "#D0D0D0",
        "<path d='M-10 -4 Q-20 10 -14 20' stroke='#D0D0D0' stroke-width='3' fill='none' stroke-linecap='round'/>",
    ),
];

/// Map an agent_id (u64) to a stable character_id (0-7).
pub fn character_for_agent(agent_id: u64) -> u8 {
    (agent_id % CHARACTERS.len() as u64) as u8
}

/// Map a mood to a stable character_id offset (blended with agent slot).
pub fn character_for_mood(mood: Mood) -> u8 {
    match mood {
        Mood::Happy => 0,
        Mood::Excited => 1,
        Mood::Neutral => 2,
        Mood::Sad => 3,
        Mood::Tired => 4,
    }
}

// ─── SVG generation ───────────────────────────────────────

/// Generate an SVG sprite for a given character and pose.
///
/// Returns a complete inline `<svg>` element (64×64 px) with no external deps.
pub fn generate_svg(character_id: u8, pose: AgentPose) -> SvgSprite {
    let idx = (character_id as usize) % CHARACTERS.len();
    let (_, body, accent, helmet, emblem) = CHARACTERS[idx];

    let svg_body = render_character_svg(body, accent, helmet, emblem, pose);
    SvgSprite {
        svg_body,
        character_id,
        pose,
    }
}

/// Generate an SVG sprite driven by mood (for companion cards, not agent forum).
pub fn generate_svg_from_mood(mood: Mood, character_id: Option<u8>) -> SvgSprite {
    let cid = character_id.unwrap_or_else(|| character_for_mood(mood));
    let pose = AgentPose::from_mood(mood);
    generate_svg(cid, pose)
}

/// Render the raw SVG string.
fn render_character_svg(
    body: &str,
    accent: &str,
    helmet: &str,
    emblem: &str,
    pose: AgentPose,
) -> String {
    let (arm_l, arm_r, leg_l, leg_r, head_tilt) = match pose {
        AgentPose::Working => (-20, 20, 5, -5, 0),
        AgentPose::Thinking => (0, -30, 0, 0, -10),
        AgentPose::Celebrating => (-30, 30, -10, 10, 0),
        AgentPose::Exhausted => (10, 10, 5, 5, 15),
        AgentPose::Alert => (-10, 10, -5, 5, 0),
        AgentPose::Idle => (0, 0, 0, 0, 0),
    };

    let eye_expr = match pose {
        AgentPose::Thinking => {
            r#"<circle cx="25" cy="22" r="2.5" fill="white"/><circle cx="39" cy="22" r="2.5" fill="white"/>"#
        }
        AgentPose::Celebrating => {
            r#"<path d="M22 22 Q25 20 28 22" stroke="white" stroke-width="1.5" fill="none"/><path d="M36 22 Q39 20 42 22" stroke="white" stroke-width="1.5" fill="none"/>"#
        }
        AgentPose::Alert => {
            r#"<circle cx="25" cy="22" r="3.5" fill="red"/><circle cx="39" cy="22" r="3.5" fill="red"/>"#
        }
        AgentPose::Exhausted => {
            r#"<path d="M22 23 Q25 25 28 23" stroke="white" stroke-width="1.5" fill="none"/><path d="M36 23 Q39 25 42 23" stroke="white" stroke-width="1.5" fill="none"/>"#
        }
        _ => {
            r#"<circle cx="25" cy="22" r="2.5" fill="white"/><circle cx="39" cy="22" r="2.5" fill="white"/>"#
        }
    };

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <g transform="translate(32,32)">
    <!-- Body -->
    <rect x="-10" y="-4" width="20" height="24" rx="4" fill="{body}"/>
    <!-- Accent stripe -->
    <rect x="-10" y="4" width="20" height="4" fill="{accent}" opacity="0.6"/>
    <!-- Archetype emblem -->
    {emblem}
    <!-- Head with tilt -->
    <g transform="rotate({head_tilt})">
      <circle cx="0" cy="-18" r="12" fill="{body}"/>
      <!-- Helmet -->
      <path d="M-12 -20 Q0 -36 12 -20" fill="{helmet}"/>
      <!-- Eyes -->
      <g transform="translate(-32,-32)">{eye_expr}</g>
    </g>
    <!-- Arm left -->
    <g transform="rotate({arm_l}, -10, 4)">
      <rect x="-16" y="0" width="6" height="14" rx="3" fill="{body}"/>
    </g>
    <!-- Arm right -->
    <g transform="rotate({arm_r}, 10, 4)">
      <rect x="10" y="0" width="6" height="14" rx="3" fill="{body}"/>
    </g>
    <!-- Leg left -->
    <g transform="rotate({leg_l}, -5, 20)">
      <rect x="-8" y="18" width="6" height="14" rx="3" fill="{accent}"/>
    </g>
    <!-- Leg right -->
    <g transform="rotate({leg_r}, 5, 20)">
      <rect x="2" y="18" width="6" height="14" rx="3" fill="{accent}"/>
    </g>
  </g>
</svg>"#,
        body = body,
        accent = accent,
        helmet = helmet,
        emblem = emblem,
        head_tilt = head_tilt,
        eye_expr = eye_expr,
        arm_l = arm_l,
        arm_r = arm_r,
        leg_l = leg_l,
        leg_r = leg_r,
    )
}

/// Render ASCII fallback (used by terminal/HUD paths).
pub fn render_ascii(mood: Mood) -> String {
    crate::sprite::generate_deterministic("vox", mood)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_non_empty_for_all_poses() {
        for pose in [
            AgentPose::Idle,
            AgentPose::Working,
            AgentPose::Thinking,
            AgentPose::Celebrating,
            AgentPose::Exhausted,
            AgentPose::Alert,
        ] {
            let s = generate_svg(0, pose);
            assert!(
                s.svg_body.contains("<svg"),
                "pose {:?} produced no SVG",
                pose
            );
        }
    }

    #[test]
    fn character_for_agent_is_stable() {
        assert_eq!(character_for_agent(0), 0);
        assert_eq!(character_for_agent(8), 0);
        assert_eq!(character_for_agent(3), 3);
    }

    #[test]
    fn pose_from_activity() {
        assert_eq!(AgentPose::from_activity_str("writing"), AgentPose::Working);
        assert_eq!(AgentPose::from_activity_str("idle"), AgentPose::Idle);
        assert_eq!(
            AgentPose::from_activity_str("thinking"),
            AgentPose::Thinking
        );
        assert_eq!(AgentPose::from_activity_str("unknown"), AgentPose::Idle);
    }
}
