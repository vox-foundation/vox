//! Programmatic SVG dashboards for Hugging Face Hub model cards (Phase 339).
//!
//! Generates `assets/scorecard.svg` (per-size model card) and `assets/overview.svg` (index hub).
//! Visual styling adheres to `339-UI-SPEC.md` v2.1 (Zero-Duplication Full-Brain Contract):
//! - Stripped of redundant text tables, code diffs, and hero banners already present in Markdown.
//! - Pure graphical panels on dark zinc canvas (`#09090b`), rounded corners (rx=16).
//! - `scorecard.svg`: 1200x320 2-panel strip (QLoRA loss trajectory + Held-out lift & McNemar deltas + Spoke slot strip).
//! - `overview.svg`: 1200x360 2-panel strip (MENS Brain hub/router/spoke topology + Ladder Pass@1 vs VRAM Pareto).

use std::fmt::Write;

use super::CardContext;
use super::config::{SpokeStatus, hub_cards};
use super::facts::{
    AdapterEvidence, GateState, RunRecords, active_spoke_id, adapter_evidence, gate_of,
    heldout_scores,
};
use crate::mens::hub_repo::SIZES;
use crate::mens::hub_sync::SizeSyncRow;

/// An asset file to be written beside the markdown card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedAsset {
    pub rel_path: String,
    pub content: String,
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render the per-size model card SVG asset (`assets/scorecard.svg`).
#[inline(never)]
pub fn render_model_assets(cx: &CardContext, rec: &RunRecords) -> Vec<RenderedAsset> {
    let svg = render_scorecard_svg(cx, rec);
    vec![RenderedAsset {
        rel_path: "assets/scorecard.svg".to_string(),
        content: svg,
    }]
}

/// Render the per-size model card SVG string (`1200x320` 2-panel graphic strip).
#[inline(never)]
pub fn render_scorecard_svg(cx: &CardContext, rec: &RunRecords) -> String {
    let gate_fact = gate_of(rec);
    let gate_state = gate_fact.state;
    let trained = gate_fact.trained;
    let size = cx.size;

    // Metrics for Panel 2 (Held-out Pass@1) — gated on AdapterEvidence::Current (B1)
    let (base_score, raw_adapter_score) = heldout_scores(rec);
    let adapter_score =
        raw_adapter_score.filter(|_| adapter_evidence(rec) == AdapterEvidence::Current);

    let mut svg = String::with_capacity(16384);
    let _ = writeln!(
        svg,
        r##"<svg viewBox="0 0 1200 320" width="100%" height="auto" xmlns="http://www.w3.org/2000/svg">
<title>Vox MENS Qwen3-{size} Adapter Scorecard</title>
<desc>Zero-duplication visual scorecard: QLoRA training loss convergence, held-out pass@1 lift, paired McNemar reasoning deltas, and multi-spoke slot strip.</desc>
<defs>
  <pattern id="grid" width="30" height="30" patternUnits="userSpaceOnUse">
    <path d="M 30 0 L 0 0 0 30" fill="none" stroke="#18181b" stroke-width="1"/>
  </pattern>
  <linearGradient id="emerald_grad" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0%" stop-color="#34d399"/>
    <stop offset="100%" stop-color="#10b981"/>
  </linearGradient>
  <linearGradient id="cyan_grad" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0%" stop-color="#38bdf8"/>
    <stop offset="100%" stop-color="#0284c7"/>
  </linearGradient>
  <linearGradient id="telemetry_area_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#10b981" stop-opacity="0.30"/>
    <stop offset="100%" stop-color="#10b981" stop-opacity="0.0"/>
  </linearGradient>
  <filter id="glow" x="-10%" y="-10%" width="120%" height="120%">
    <feGaussianBlur stdDeviation="3" result="blur"/>
    <feComposite in="SourceGraphic" in2="blur" operator="over"/>
  </filter>
</defs>

<style>
  text {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }}
  .mono {{ font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace; }}
  .card {{ fill: #12161f; stroke: #27272a; stroke-width: 1; rx: 12px; }}
  .track {{ fill: #1e2430; rx: 4px; }}
  .subtle {{ fill: #71717a; }}
  .bright {{ fill: #f8fafc; font-weight: 600; }}
</style>

<!-- Canvas background -->
<rect x="0" y="0" width="1200" height="320" rx="16" fill="#09090b" stroke="#27272a" stroke-width="1"/>
<rect x="0" y="0" width="1200" height="320" rx="16" fill="url(#grid)" opacity="0.5"/>"##,
        size = escape_xml(size),
    );

    // ================= LEFT CARD: QLORA TRAINING LOSS & EMA CONVERGENCE =================
    let chart_x = 54.0;
    let chart_y = 82.0;
    let chart_w = 486.0;
    let chart_h = 160.0;
    let y_min = 0.40f32;
    let y_max = 2.60f32;

    let has_telemetry = !rec.telemetry_loss.is_empty();

    let _ = writeln!(
        svg,
        r##"<!-- ================= LEFT CARD: QLORA TRAINING LOSS & EMA CONVERGENCE ================= -->
<g transform="translate(24, 24)">
  <rect x="0" y="0" width="564" height="272" class="card"/>
  
  <!-- Header Title & Pulse Glyph -->
  <text x="24" y="34" font-size="13" font-weight="700" class="mono bright">QLORA TRAINING LOSS &amp; EMA CONVERGENCE</text>
  <path d="M 334 30 L 339 30 L 342 24 L 346 36 L 350 27 L 354 30 L 362 30" fill="none" stroke="#52525b" stroke-width="1.5"/>

  <!-- Subtitle -->
  <text x="24" y="52" font-size="11" class="mono subtle">Telemetry plot</text>"##
    );

    if has_telemetry {
        let first_loss = rec.telemetry_loss.first().unwrap().1;
        let last_loss = rec.telemetry_loss.last().unwrap().1;
        let _ = writeln!(
            svg,
            r##"  <!-- Top-Right EMA Trajectory Pill -->
  <g transform="translate(390, 16)">
    <rect x="0" y="0" width="150" height="24" rx="12" fill="#064e3b" fill-opacity="0.4" stroke="#10b981" stroke-width="1"/>
    <circle cx="14" cy="12" r="3.5" fill="#10b981"/>
    <text x="24" y="16" font-size="11" font-weight="600" class="mono" fill="#34d399">EMA {first_loss:.2} → {last_loss:.2}</text>
  </g>"##,
            first_loss = first_loss,
            last_loss = last_loss,
        );

        // Y-axis gridlines
        let y_ticks = [
            (2.5f32, "2.5"),
            (2.0f32, "2.0"),
            (1.5f32, "1.5"),
            (1.0f32, "1.0"),
            (0.5f32, "0.5"),
        ];
        for &(val, lbl) in &y_ticks {
            let norm_y = (val - y_min) / (y_max - y_min);
            let py = chart_y + chart_h - (norm_y as f64 * chart_h);
            let _ = writeln!(
                svg,
                r##"  <text x="44" y="{py_text:.1}" text-anchor="end" font-size="10" class="mono subtle">{lbl}</text>
  <line x1="{x1}" y1="{py:.1}" x2="{x2}" y2="{py:.1}" stroke="#1f242d" stroke-width="1" stroke-dasharray="2,2"/>"##,
                py = py,
                py_text = py + 3.0,
                x1 = chart_x,
                x2 = chart_x + chart_w,
                lbl = lbl,
            );
        }

        let min_s = rec.telemetry_loss.first().unwrap().0 as f64;
        let max_s = rec.telemetry_loss.last().unwrap().0 as f64;
        let span_s = (max_s - min_s).max(1.0);

        // Dynamic X-axis step labels computed from min_s..max_s
        let x_steps = [
            (format!("Step {}", min_s.round() as u32), 0.0),
            (
                format!("Step {}", (min_s + 0.25 * span_s).round() as u32),
                0.25,
            ),
            (
                format!("Step {}", (min_s + 0.70 * span_s).round() as u32),
                0.70,
            ),
            (format!("Step {}", max_s.round() as u32), 1.0),
        ];
        for (lbl, frac) in &x_steps {
            let px = chart_x + frac * chart_w;
            let anchor = if *frac == 0.0 {
                "start"
            } else if *frac == 1.0 {
                "end"
            } else {
                "middle"
            };
            let _ = writeln!(
                svg,
                r##"  <text x="{px:.1}" y="{py:.1}" text-anchor="{anchor}" font-size="10" class="mono subtle">{lbl}</text>"##,
                px = px,
                py = chart_y + chart_h + 16.0,
                anchor = anchor,
                lbl = lbl,
            );
        }

        // Raw trace
        if !rec.telemetry_raw.is_empty() {
            let mut raw_poly = String::new();
            for &(s, l) in &rec.telemetry_raw {
                let nx = (s as f64 - min_s) / span_s;
                let ny = (l.clamp(y_min, y_max) - y_min) / (y_max - y_min);
                let px = chart_x + nx * chart_w;
                let py = chart_y + chart_h - (ny as f64 * chart_h);
                let _ = write!(raw_poly, "{:.1},{:.1} ", px, py);
            }
            let _ = writeln!(
                svg,
                r##"  <polyline points="{raw_poly}" fill="none" stroke="#52525b" stroke-opacity="0.35" stroke-width="1"/>"##,
                raw_poly = raw_poly
            );
        }

        // EMA polyline & area
        let pts: Vec<(f64, f64, f32)> = rec
            .telemetry_loss
            .iter()
            .map(|&(s, l)| {
                let nx = (s as f64 - min_s) / span_s;
                let ny = (l.clamp(y_min, y_max) - y_min) / (y_max - y_min);
                (
                    chart_x + nx * chart_w,
                    chart_y + chart_h - (ny as f64 * chart_h),
                    l,
                )
            })
            .collect();

        let mut poly_str = String::new();
        let mut area_str = format!("M {:.1} {:.1}", pts[0].0, chart_y + chart_h);
        for &(px, py, _) in &pts {
            let _ = write!(poly_str, "{:.1},{:.1} ", px, py);
            let _ = write!(area_str, " L {:.1} {:.1}", px, py);
        }
        let _ = write!(
            area_str,
            " L {:.1} {:.1} Z",
            pts.last().unwrap().0,
            chart_y + chart_h
        );

        let first = pts.first().unwrap();
        let last = pts.last().unwrap();

        let _ = writeln!(
            svg,
            r##"  <path d="{area_str}" fill="url(#telemetry_area_grad)"/>
  <polyline points="{poly_str}" fill="none" stroke="#10b981" stroke-width="2.5" filter="url(#glow)"/>
  <circle cx="{first_x:.1}" cy="{first_y:.1}" r="3.5" fill="#38bdf8"/>
  <text x="{first_tx:.1}" y="{first_ty:.1}" font-size="10" font-weight="600" class="mono" fill="#38bdf8">{first_l:.2}</text>
  <circle cx="{last_x:.1}" cy="{last_y:.1}" r="3.5" fill="#10b981"/>
  <text x="{last_tx:.1}" y="{last_ty:.1}" text-anchor="end" font-size="10" font-weight="600" class="mono" fill="#34d399">{last_l:.2}</text>"##,
            area_str = area_str,
            poly_str = poly_str,
            first_x = first.0,
            first_y = first.1,
            first_tx = first.0 + 8.0,
            first_ty = first.1 - 6.0,
            first_l = first.2,
            last_x = last.0,
            last_y = last.1,
            last_tx = last.0 - 8.0,
            last_ty = last.1 - 6.0,
            last_l = last.2,
        );
    } else if trained {
        let _ = writeln!(
            svg,
            r##"  <rect x="54" y="82" width="486" height="160" rx="8" fill="#18181b" fill-opacity="0.4" stroke="#27272a" stroke-dasharray="4,4"/>
  <text x="297" y="152" text-anchor="middle" font-size="13" font-weight="600" fill="#94a3b8">Step Telemetry Not Recorded</text>
  <text x="297" y="174" text-anchor="middle" font-size="11" class="mono subtle">telemetry.jsonl not recorded for this run</text>"##
        );
    } else {
        // Untrained clean empty state
        let untrained_detail = if size == "32B" {
            "32B QLoRA footprint (66 GB) exceeds 64 GB Mac unified memory limit".to_string()
        } else {
            format!(
                "Run vox mens train --size {} to populate training telemetry",
                escape_xml(size)
            )
        };
        let _ = writeln!(
            svg,
            r##"  <rect x="54" y="82" width="486" height="160" rx="8" fill="#18181b" fill-opacity="0.4" stroke="#27272a" stroke-dasharray="4,4"/>
  <text x="297" y="152" text-anchor="middle" font-size="13" font-weight="600" fill="#94a3b8">Model Weights Not Yet Trained</text>
  <text x="297" y="174" text-anchor="middle" font-size="11" class="mono subtle">{untrained_detail}</text>"##,
            untrained_detail = untrained_detail,
        );
    }
    let _ = writeln!(svg, "</g>");

    // ================= RIGHT CARD: SPOKE GATE & PAIRED MCNEMAR VISUAL DELTAS =================
    let scale_max = 0.50f64;
    let track_w = 230.0f64;

    let _ = writeln!(
        svg,
        r##"<!-- ================= RIGHT CARD: SPOKE GATE & PAIRED MCNEMAR VISUAL DELTAS ================= -->
<g transform="translate(612, 24)">
  <rect x="0" y="0" width="564" height="272" class="card"/>
  
  <!-- Header Title -->
  <text x="24" y="34" font-size="13" font-weight="700" class="mono bright">SPOKE GATE &amp; PAIRED MCNEMAR VISUAL DELTAS</text>

  <!-- 1. Bullet Comparison -->
  <text x="24" y="56" font-size="11" class="mono subtle">Bullet comparison</text>
  
  <!-- Base Row -->
  <text x="24" y="76" font-size="11" class="mono subtle">Base Pass@1</text>
  <rect x="135" y="66" width="230" height="13" class="track"/>"##
    );

    if let Some(bs) = base_score {
        let base_bar_w = (((bs.rate / scale_max).clamp(0.0, 1.0) * track_w) as u32).max(6);
        let base_passed = bs.passed.round() as u64;
        let _ = writeln!(
            svg,
            r##"  <rect x="135" y="66" width="{base_bar_w}" height="13" rx="3" fill="#64748b"/>
  <text x="378" y="76" font-size="10" class="mono subtle">{base_pct:.1}% ({base_passed}/{base_total})</text>"##,
            base_bar_w = base_bar_w,
            base_pct = bs.rate * 100.0,
            base_passed = base_passed,
            base_total = bs.total,
        );
    } else {
        let _ = writeln!(
            svg,
            r##"  <rect x="135" y="66" width="230" height="13" class="track" stroke="#3f3f46" stroke-dasharray="3,3"/>
  <text x="378" y="76" font-size="10" class="mono subtle">n/a (not evaluated)</text>"##
        );
    }

    if let (Some(bs), Some(as_)) = (base_score, adapter_score) {
        let delta_pp = (as_.rate - bs.rate) * 100.0;
        let ratio = if bs.rate > 0.001 {
            as_.rate / bs.rate
        } else {
            1.0
        };
        let _ = writeln!(
            svg,
            r##"  <g transform="translate(452, 62)">
    <rect x="0" y="0" width="84" height="20" rx="10" fill="#064e3b" fill-opacity="0.3" stroke="#10b981" stroke-width="1"/>
    <text x="42" y="14" text-anchor="middle" font-size="10" font-weight="700" class="mono" fill="#34d399">{delta_pp:+.1}% lift</text>
  </g>
  <text x="544" y="98" text-anchor="end" font-size="10" class="mono subtle">{ratio:.1}x</text>"##,
            delta_pp = delta_pp,
            ratio = ratio,
        );
    } else {
        let _ = writeln!(
            svg,
            r##"  <g transform="translate(452, 62)">
    <rect x="0" y="0" width="84" height="20" rx="10" fill="#18181b" stroke="#3f3f46" stroke-width="1"/>
    <text x="42" y="14" text-anchor="middle" font-size="9" font-weight="700" class="mono subtle">PENDING</text>
  </g>"##
        );
    }

    // Adapter Row
    let _ = writeln!(
        svg,
        r##"  <!-- Adapter Row -->
  <text x="24" y="98" font-size="11" class="mono bright">Adapter Pass@1</text>
  <rect x="135" y="88" width="230" height="13" class="track"/>"##
    );

    if let Some(as_) = adapter_score {
        let adapter_bar_w = (((as_.rate / scale_max).clamp(0.0, 1.0) * track_w) as u32).max(6);
        let adapter_passed = as_.passed.round() as u64;
        let _ = writeln!(
            svg,
            r##"  <rect x="135" y="88" width="{adapter_bar_w}" height="13" rx="3" fill="url(#emerald_grad)" filter="url(#glow)"/>
  <text x="378" y="98" font-size="10" font-weight="600" class="mono bright">{adapter_pct:.1}% ({adapter_passed}/{adapter_total})</text>"##,
            adapter_bar_w = adapter_bar_w,
            adapter_pct = as_.rate * 100.0,
            adapter_passed = adapter_passed,
            adapter_total = as_.total,
        );
    } else {
        let _ = writeln!(
            svg,
            r##"  <rect x="135" y="88" width="230" height="13" class="track" stroke="#3f3f46" stroke-dasharray="3,3"/>
  <text x="378" y="98" font-size="10" class="mono subtle">awaiting evaluation</text>"##
        );
    }

    // 2. Paired McNemar reasoning slices
    let _ = writeln!(
        svg,
        r##"  <!-- 2. Paired McNemar Reasoning Slice -->
  <text x="24" y="126" font-size="11" class="mono subtle">Paired McNemar reasoning slice</text>
  
  <!-- Alpha = 0.05 Threshold Marker -->
  <text x="360" y="126" text-anchor="middle" font-size="10" class="mono subtle">α = 0.05</text>
  <line x1="360" y1="130" x2="360" y2="198" stroke="#52525b" stroke-dasharray="2,2" stroke-width="1"/>"##
    );

    let slice_names = ["gsm8k_think", "math500_think", "vox_think"];

    for (i, &slice_name) in slice_names.iter().enumerate() {
        let y_row = 142 + i * 20;
        let slice_info = rec.collateral.as_ref().and_then(|c| {
            let acc_obj = &c["reasoning_slices"]["benches"][slice_name]["accuracy"];
            let pre_acc = acc_obj["pre"].as_f64().or_else(|| {
                c["reports"].as_array().and_then(|arr| {
                    arr.iter()
                        .find(|r| r["benchmark"].as_str() == Some(slice_name))
                        .and_then(|r| r["pre"].as_f64())
                })
            })?;
            let post_acc = acc_obj["post"].as_f64().or_else(|| {
                c["reports"].as_array().and_then(|arr| {
                    arr.iter()
                        .find(|r| r["benchmark"].as_str() == Some(slice_name))
                        .and_then(|r| r["post"].as_f64())
                })
            })?;
            let alpha = c["reasoning_slices"]["alpha"].as_f64().unwrap_or(0.05);
            let p_val = acc_obj["p_value"].as_f64();
            let regressed = acc_obj["regressed"].as_bool().unwrap_or_else(|| {
                if let Some(p) = p_val {
                    post_acc < pre_acc && p < alpha
                } else {
                    let deg = (pre_acc - post_acc) / pre_acc.max(0.001);
                    deg > 0.05
                }
            });
            Some((pre_acc, post_acc, p_val, regressed))
        });

        if let Some((pre_acc, post_acc, p_val, regressed)) = slice_info {
            let (status_text, status_color, status_bg) = if gate_state == GateState::NeedsReGate {
                ("PRE-GATE", "#fbbf24", "rgba(251,191,36,0.14)")
            } else if regressed {
                ("REGRESSED", "#f87171", "rgba(248,113,113,0.14)")
            } else {
                ("CLEAR", "#34d399", "rgba(16,185,129,0.14)")
            };
            let pre_w = ((pre_acc.clamp(0.0, 1.0) * 160.0) as u32).max(4);
            let post_w = ((post_acc.clamp(0.0, 1.0) * 160.0) as u32).max(4);
            let detail_label = match p_val {
                Some(p) => format!(
                    "{:.1}%→{:.1}% (p={:.2})",
                    pre_acc * 100.0,
                    post_acc * 100.0,
                    p
                ),
                None => format!("{:.1}%→{:.1}%", pre_acc * 100.0, post_acc * 100.0),
            };
            let _ = writeln!(
                svg,
                r##"  <!-- Slice {slice_name} -->
  <text x="24" y="{text_y}" font-size="11" class="mono subtle">{slice_name}</text>
  <rect x="135" y="{bar_y}" width="160" height="10" class="track"/>
  <rect x="135" y="{bar_y}" width="{post_w}" height="10" rx="2" fill="{post_fill}"/>
  <rect x="135" y="{bar_y}" width="{pre_w}" height="10" rx="2" fill="#38bdf8" opacity="0.4"/>
  <text x="303" y="{text_y}" font-size="9" class="mono subtle">{detail_label}</text>
  <g transform="translate(470, {badge_y})">
    <rect x="0" y="0" width="70" height="16" rx="8" fill="{status_bg}" stroke="{status_color}" stroke-width="1"/>
    <text x="35" y="12" text-anchor="middle" font-size="9" font-weight="700" class="mono" fill="{status_color}">{status_text}</text>
  </g>"##,
                slice_name = slice_name,
                text_y = y_row + 9,
                bar_y = y_row,
                pre_w = pre_w,
                post_w = post_w,
                post_fill = if status_color == "#f87171" {
                    "#f87171"
                } else {
                    "#10b981"
                },
                detail_label = detail_label,
                badge_y = y_row - 3,
                status_bg = status_bg,
                status_color = status_color,
                status_text = status_text,
            );
        } else {
            let (status_text, status_color, status_bg) = if gate_state == GateState::NeedsReGate {
                ("PRE-GATE", "#fbbf24", "rgba(251,191,36,0.14)")
            } else {
                ("PENDING", "#71717a", "rgba(113,113,122,0.14)")
            };
            let _ = writeln!(
                svg,
                r##"  <!-- Slice {slice_name} -->
  <text x="24" y="{text_y}" font-size="11" class="mono subtle">{slice_name}</text>
  <rect x="135" y="{bar_y}" width="160" height="10" class="track"/>
  <text x="303" y="{text_y}" font-size="9" class="mono subtle">not evaluated</text>
  <g transform="translate(470, {badge_y})">
    <rect x="0" y="0" width="70" height="16" rx="8" fill="{status_bg}" stroke="{status_color}" stroke-width="1"/>
    <text x="35" y="12" text-anchor="middle" font-size="9" font-weight="700" class="mono" fill="{status_color}">{status_text}</text>
  </g>"##,
                slice_name = slice_name,
                text_y = y_row + 9,
                bar_y = y_row,
                badge_y = y_row - 3,
                status_bg = status_bg,
                status_color = status_color,
                status_text = status_text,
            );
        }
    }

    // 3. Spoke Slot Navigation Strip (dynamically driven by hub-cards.yaml, active_spoke_id, and spoke_gates)
    let cfg = hub_cards().ok();
    let default_spoke_id = cfg.map(|c| c.default_spoke.as_str()).unwrap_or("vox-lang");
    let active_spoke = trained
        .then(|| active_spoke_id(rec, Some(default_spoke_id)))
        .flatten();
    let spoke_slots: [(&str, u32, u32); 5] = [
        ("vox-lang", 0, 76),
        ("rust", 84, 54),
        ("harness", 146, 72),
        ("tool-selection", 226, 104),
        ("argument-generation", 338, 144),
    ];

    let _ = writeln!(
        svg,
        r##"  <!-- 3. Spoke Slot Navigation Strip -->
  <g transform="translate(12, 212)">
    <rect x="0" y="0" width="540" height="48" rx="8" fill="#09090b" stroke="#27272a" stroke-width="1"/>
    <text x="12" y="16" font-size="10" class="mono subtle">spoke slot navigation strip</text>
    <text x="528" y="16" text-anchor="end" font-size="9" class="mono subtle">spokes/&lt;spoke&gt;/</text>
    
    <!-- 5 Spoke Slots -->
    <g transform="translate(10, 22)">"##
    );

    for (spoke_id, slot_x, slot_w) in spoke_slots {
        let is_active_spoke = active_spoke.as_deref() == Some(spoke_id);
        let explicit_spoke_gate = rec.spoke_gates.iter().find(|g| g.spoke == spoke_id);
        let (rect_fill, rect_stroke, stroke_w, glow_attr, text_fill, font_weight) =
            if is_active_spoke {
                match gate_state {
                    GateState::Passed => (
                        "#064e3b",
                        "#10b981",
                        "1.5",
                        r#" filter="url(#glow)""#,
                        "#34d399",
                        r#" font-weight="700""#,
                    ),
                    GateState::NeedsReGate | GateState::NotEvaluated => (
                        "#451a03",
                        "#f59e0b",
                        "1.5",
                        "",
                        "#fbbf24",
                        r#" font-weight="700""#,
                    ),
                    GateState::Failed => (
                        "#450a0a",
                        "#ef4444",
                        "1.5",
                        "",
                        "#f87171",
                        r#" font-weight="700""#,
                    ),
                }
            } else if explicit_spoke_gate
                .is_some_and(|g| g.state == vox_eval::gate_state::GateState::Passed)
            {
                (
                    "#064e3b",
                    "#10b981",
                    "1.5",
                    "",
                    "#34d399",
                    r#" font-weight="700""#,
                )
            } else {
                ("#18181b", "#3f3f46", "1", "", "#71717a", "")
            };
        let text_x = slot_x + slot_w / 2;
        let _ = writeln!(
            svg,
            r##"      <rect x="{slot_x}" y="0" width="{slot_w}" height="20" rx="4" fill="{rect_fill}" stroke="{rect_stroke}" stroke-width="{stroke_w}"{glow_attr}/>
      <text x="{text_x}" y="14" text-anchor="middle" font-size="10"{font_weight} class="mono" fill="{text_fill}">[ {spoke_id} ]</text>"##,
            slot_x = slot_x,
            slot_w = slot_w,
            rect_fill = rect_fill,
            rect_stroke = rect_stroke,
            stroke_w = stroke_w,
            glow_attr = glow_attr,
            text_x = text_x,
            font_weight = font_weight,
            text_fill = text_fill,
            spoke_id = spoke_id,
        );
    }

    let _ = writeln!(
        svg,
        r##"    </g>
  </g>
</g>
</svg>"##
    );

    svg
}

/// Render the index overview SVG asset (`assets/overview.svg`).
#[inline(never)]
pub fn render_index_assets(
    namespace: &str,
    entries: &[(SizeSyncRow, RunRecords)],
) -> Vec<RenderedAsset> {
    let svg = render_overview_svg(namespace, entries);
    vec![RenderedAsset {
        rel_path: "assets/overview.svg".to_string(),
        content: svg,
    }]
}

/// Render the index overview SVG string (`1200x360` 2-panel graphic strip).
#[inline(never)]
pub fn render_overview_svg(_namespace: &str, entries: &[(SizeSyncRow, RunRecords)]) -> String {
    let mut svg = String::with_capacity(20480);
    let _ = writeln!(
        svg,
        r##"<svg viewBox="0 0 1200 360" width="100%" height="auto" xmlns="http://www.w3.org/2000/svg">
<title>Vox MENS Brain — Hub, Retrieval &amp; Multi-Spoke Topology</title>
<desc>Zero-duplication graphic overview: Full Vox MENS brain topology (hub, embedder, orchestrator router, 5 QLoRA spokes, 4 knowledge lanes) and 6-size Pass@1 lift vs Apple Silicon VRAM Pareto frontier.</desc>
<defs>
  <pattern id="grid" width="30" height="30" patternUnits="userSpaceOnUse">
    <path d="M 30 0 L 0 0 0 30" fill="none" stroke="#18181b" stroke-width="1"/>
  </pattern>
  <linearGradient id="emerald_grad" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0%" stop-color="#34d399"/>
    <stop offset="100%" stop-color="#10b981"/>
  </linearGradient>
  <linearGradient id="cyan_grad" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0%" stop-color="#38bdf8"/>
    <stop offset="100%" stop-color="#0284c7"/>
  </linearGradient>
  <filter id="glow" x="-10%" y="-10%" width="120%" height="120%">
    <feGaussianBlur stdDeviation="3" result="blur"/>
    <feComposite in="SourceGraphic" in2="blur" operator="over"/>
  </filter>
</defs>

<style>
  text {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }}
  .mono {{ font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace; }}
  .card {{ fill: #12161f; stroke: #27272a; stroke-width: 1; rx: 12px; }}
  .track {{ fill: #1e2430; rx: 4px; }}
  .subtle {{ fill: #71717a; }}
  .bright {{ fill: #f8fafc; font-weight: 600; }}
</style>

<!-- Canvas background -->
<rect x="0" y="0" width="1200" height="360" rx="16" fill="#09090b" stroke="#27272a" stroke-width="1"/>
<rect x="0" y="0" width="1200" height="360" rx="16" fill="url(#grid)" opacity="0.5"/>"##
    );

    // ================= LEFT CARD: VOX MENS BRAIN TOPOLOGY =================
    let trained_count = entries
        .iter()
        .filter(|(_, rec)| gate_of(rec).trained)
        .count();
    let passed_count = entries
        .iter()
        .filter(|(_, rec)| gate_of(rec).state == GateState::Passed)
        .count();
    let vox_lang_badge = if passed_count > 0 {
        format!("✓ {passed_count}/{trained_count} Gated")
    } else if trained_count > 0 {
        format!("● {trained_count} Trained")
    } else {
        "✓ Active".to_string()
    };
    let spoke_status_for =
        |spoke_id: &str| -> (&'static str, &'static str, &'static str, &'static str) {
            let passed_n = entries
                .iter()
                .filter(|(_, rec)| {
                    rec.spoke_gates.iter().any(|g| {
                        g.spoke == spoke_id && g.state == vox_eval::gate_state::GateState::Passed
                    })
                })
                .count();
            let cfg_trained = hub_cards()
                .ok()
                .and_then(|c| c.spokes.iter().find(|s| s.id == spoke_id))
                .is_some_and(|s| s.status == SpokeStatus::Trained);
            if passed_n > 0 || cfg_trained {
                ("#064e3b", "#10b981", "#34d399", "✓ Active")
            } else {
                ("#18181b", "#3f3f46", "#71717a", "Planned")
            }
        };
    let (rust_fill, rust_stroke, rust_color, rust_lbl) = spoke_status_for("rust");
    let (harn_fill, harn_stroke, harn_color, harn_lbl) = spoke_status_for("harness");
    let (tool_fill, tool_stroke, tool_color, tool_lbl) = spoke_status_for("tool-selection");
    let (arg_fill, arg_stroke, arg_color, arg_lbl) = spoke_status_for("argument-generation");

    let _ = writeln!(
        svg,
        r##"<!-- ================= LEFT CARD: VOX MENS BRAIN — HUB, RETRIEVAL & MULTI-SPOKE TOPOLOGY ================= -->
<g transform="translate(24, 24)">
  <rect x="0" y="0" width="564" height="312" class="card"/>

  <!-- Window Header / Title & Status Dots -->
  <text x="24" y="32" font-size="12" font-weight="700" class="mono bright">VOX MENS BRAIN — HUB, RETRIEVAL &amp; MULTI-SPOKE TOPOLOGY</text>
  <g transform="translate(506, 22)">
    <circle cx="10" cy="8" r="4" fill="#10b981"/>
    <circle cx="24" cy="8" r="4" fill="#38bdf8"/>
    <circle cx="38" cy="8" r="4" fill="#6366f1"/>
  </g>

  <!-- Telemetry Subtitle Strip -->
  <text x="24" y="50" font-size="10" class="mono subtle">Architecture • Frozen Base + Dynamic LoRA Routing</text>
  <text x="24" y="64" font-size="10" class="mono subtle">Status • <tspan fill="#10b981">● Active Routing</tspan></text>

  <!-- LEFT HUB STACK (x=24, w=160) -->
  <!-- 1. Qwen3 Dense Base (Frozen F16/BF16) -->
  <g transform="translate(24, 80)">
    <rect x="0" y="0" width="168" height="52" rx="8" fill="#064e3b" fill-opacity="0.3" stroke="#10b981" stroke-width="1.2"/>
    <circle cx="20" cy="26" r="10" fill="#064e3b" stroke="#34d399" stroke-width="1"/>
    <text x="20" y="30" text-anchor="middle" font-size="10" fill="#34d399">🧠</text>
    <text x="38" y="22" font-size="11" font-weight="700" class="mono bright">Qwen3 Dense Base</text>
    <text x="38" y="37" font-size="7.5" class="mono subtle">0.6B · 1.7B · 4B · 8B · 14B · 32B</text>
  </g>

  <!-- Router Junction Orb -->
  <circle cx="104" cy="148" r="9" fill="#09090b" stroke="#38bdf8" stroke-width="1.5"/>
  <circle cx="104" cy="148" r="3.5" fill="#38bdf8"/>

  <!-- 2. Qwen3-0.6B Embedder (B3 Retrieval) -->
  <g transform="translate(24, 164)">
    <rect x="0" y="0" width="160" height="46" rx="8" fill="#0c4a6e" fill-opacity="0.3" stroke="#0284c7" stroke-width="1.2"/>
    <circle cx="20" cy="23" r="10" fill="#0c4a6e" stroke="#38bdf8" stroke-width="1"/>
    <text x="20" y="27" text-anchor="middle" font-size="10" fill="#38bdf8">⛁</text>
    <text x="38" y="20" font-size="11" font-weight="700" class="mono bright">Qwen3-0.6B Embedder</text>
    <text x="38" y="34" font-size="8" class="mono" fill="#38bdf8">(B3 Retrieval, c1899de2)</text>
  </g>

  <!-- Vertical connector line to Orchestrator -->
  <line x1="104" y1="210" x2="104" y2="226" stroke="#52525b" stroke-width="1.5"/>

  <!-- 3. vox-orchestrator Router -->
  <g transform="translate(24, 226)">
    <rect x="0" y="0" width="160" height="46" rx="8" fill="#18181b" stroke="#3f3f46" stroke-width="1.2"/>
    <circle cx="20" cy="23" r="10" fill="#27272a" stroke="#71717a" stroke-width="1"/>
    <text x="20" y="27" text-anchor="middle" font-size="10" fill="#a1a1aa">⇄</text>
    <text x="38" y="20" font-size="11" font-weight="700" class="mono bright">vox-orchestrator</text>
    <text x="38" y="34" font-size="8" class="mono subtle">Router (spoke_gates.json)</text>
  </g>

  <!-- BEZIER CONNECTORS (Hub to Spokes) -->
  <path d="M 184 106 C 240 106, 260 88, 310 88" stroke="#10b981" stroke-width="1.5" fill="none"/>
  <path d="M 184 106 C 240 106, 260 114, 310 114" stroke="#10b981" stroke-width="1.5" fill="none"/>
  <path d="M 184 106 C 240 106, 260 140, 310 140" stroke="#10b981" stroke-width="1.5" fill="none"/>
  <path d="M 184 106 C 240 106, 260 166, 310 166" stroke="#10b981" stroke-width="1.5" fill="none"/>
  <path d="M 184 106 C 240 106, 260 192, 310 192" stroke="#06b6d4" stroke-width="1.5" fill="none"/>

  <!-- DASHED BEZIER CONNECTORS (Embedder to Knowledge/RLVR lanes) -->
  <path d="M 184 187 C 240 187, 260 226, 310 226" stroke="#818cf8" stroke-dasharray="3,3" stroke-width="1.2" fill="none"/>
  <path d="M 184 187 C 240 187, 260 248, 310 248" stroke="#818cf8" stroke-dasharray="3,3" stroke-width="1.2" fill="none"/>
  <path d="M 184 187 C 240 187, 260 270, 310 270" stroke="#818cf8" stroke-dasharray="3,3" stroke-width="1.2" fill="none"/>
  <path d="M 184 187 C 240 187, 260 292, 310 292" stroke="#818cf8" stroke-dasharray="3,3" stroke-width="1.2" fill="none"/>

  <!-- RIGHT COLUMN NODES (x=310, w=230) -->
  <!-- Top 5 QLoRA Spokes -->
  <!-- 1. vox-lang (*.vox) -->
  <g transform="translate(310, 77)">
    <rect x="0" y="0" width="230" height="23" rx="4" fill="#064e3b" stroke="#10b981" stroke-width="1.2" filter="url(#glow)"/>
    <text x="8" y="10" font-size="7" font-weight="700" class="mono" fill="#34d399">QLoRA Spoke</text>
    <text x="8" y="20" font-size="9" font-weight="600" class="mono" fill="#f0fdf4">🗎 vox-lang (*.vox)</text>
    <text x="222" y="16" text-anchor="end" font-size="8" class="mono" fill="#34d399">{vox_lang_badge}</text>
  </g>

  <!-- 2. rust (*.rs) -->
  <g transform="translate(310, 103)">
    <rect x="0" y="0" width="230" height="23" rx="4" fill="{rust_fill}" stroke="{rust_stroke}" stroke-width="1"/>
    <text x="8" y="10" font-size="7" font-weight="700" class="mono" fill="{rust_color}">QLoRA Spoke</text>
    <text x="8" y="20" font-size="9" class="mono" fill="#e4e4e7">⚙ rust (*.rs)</text>
    <text x="222" y="16" text-anchor="end" font-size="8" class="mono" fill="{rust_color}">{rust_lbl}</text>
  </g>

  <!-- 3. harness (v1 union) -->
  <g transform="translate(310, 129)">
    <rect x="0" y="0" width="230" height="23" rx="4" fill="{harn_fill}" stroke="{harn_stroke}" stroke-width="1"/>
    <text x="8" y="10" font-size="7" font-weight="700" class="mono" fill="{harn_color}">QLoRA Spoke</text>
    <text x="8" y="20" font-size="9" class="mono" fill="#e4e4e7">⚗ harness (v1 union)</text>
    <text x="222" y="16" text-anchor="end" font-size="8" class="mono" fill="{harn_color}">{harn_lbl}</text>
  </g>

  <!-- 4. tool-selection (MCP) -->
  <g transform="translate(310, 155)">
    <rect x="0" y="0" width="230" height="23" rx="4" fill="{tool_fill}" stroke="{tool_stroke}" stroke-width="1"/>
    <text x="8" y="10" font-size="7" font-weight="700" class="mono" fill="{tool_color}">QLoRA Spoke</text>
    <text x="8" y="20" font-size="9" class="mono" fill="#e4e4e7">🛠 tool-selection (MCP)</text>
    <text x="222" y="16" text-anchor="end" font-size="8" class="mono" fill="{tool_color}">{tool_lbl}</text>
  </g>

  <!-- 5. argument-generation (JSON) -->
  <g transform="translate(310, 181)">
    <rect x="0" y="0" width="230" height="23" rx="4" fill="{arg_fill}" stroke="{arg_stroke}" stroke-width="1"/>
    <text x="8" y="10" font-size="7" font-weight="700" class="mono" fill="{arg_color}">QLoRA Spoke</text>
    <text x="8" y="20" font-size="9" class="mono" fill="#e4e4e7">{{}} argument-generation</text>
    <text x="222" y="16" text-anchor="end" font-size="8" class="mono" fill="{arg_color}">{arg_lbl}</text>
  </g>

  <!-- Bottom 4 Knowledge & RLVR Lanes -->
  <!-- 1. research-expert -->
  <g transform="translate(310, 215)">
    <rect x="0" y="0" width="230" height="21" rx="4" fill="#1e1b4b" stroke="#4f46e5" stroke-width="1"/>
    <text x="8" y="14" font-size="8" class="mono" fill="#c7d2fe">🔬 research-expert (GRPO+RLVR)</text>
  </g>

  <!-- 2. populi-meta -->
  <g transform="translate(310, 237)">
    <rect x="0" y="0" width="230" height="21" rx="4" fill="#18181b" stroke="#312e81" stroke-width="1"/>
    <text x="8" y="14" font-size="8" class="mono" fill="#a5b4fc">🏛 populi-meta (Architecture)</text>
  </g>

  <!-- 3. research (Socrates) -->
  <g transform="translate(310, 259)">
    <rect x="0" y="0" width="230" height="21" rx="4" fill="#18181b" stroke="#312e81" stroke-width="1"/>
    <text x="8" y="14" font-size="8" class="mono" fill="#a5b4fc">📜 research (Socrates)</text>
  </g>

  <!-- 4. rocks (RocksDB) -->
  <g transform="translate(310, 281)">
    <rect x="0" y="0" width="230" height="21" rx="4" fill="#18181b" stroke="#312e81" stroke-width="1"/>
    <text x="8" y="14" font-size="8" class="mono" fill="#a5b4fc">🗄 rocks (RocksDB)</text>
  </g>
</g>"##,
        vox_lang_badge = vox_lang_badge,
        rust_fill = rust_fill,
        rust_stroke = rust_stroke,
        rust_color = rust_color,
        rust_lbl = rust_lbl,
        harn_fill = harn_fill,
        harn_stroke = harn_stroke,
        harn_color = harn_color,
        harn_lbl = harn_lbl,
        tool_fill = tool_fill,
        tool_stroke = tool_stroke,
        tool_color = tool_color,
        tool_lbl = tool_lbl,
        arg_fill = arg_fill,
        arg_stroke = arg_stroke,
        arg_color = arg_color,
        arg_lbl = arg_lbl,
    );

    // ================= RIGHT CARD: FULL LADDER (0.6B-32B) PARETO CHART =================
    let chart_floor_y = 250.0f64;
    let chart_h = 180.0f64;
    let max_scale_val = 80.0f64;
    let default_vram_gb = [15.0_f64, 19.0, 22.0, 32.0, 42.0, 66.0];
    let mut vram_gbs = [0.0_f64; 6];
    let mut vram_ys = [0.0_f64; 6];
    for (i, &size_name) in SIZES.iter().enumerate() {
        let mut vram_gb = default_vram_gb[i];
        if let Some((row, rec)) = entries.iter().find(|(r, _)| r.size == size_name) {
            if let Some(mb) = row.measured_peak_mb.or_else(|| {
                rec.train_record
                    .as_ref()
                    .and_then(|v| v["peak_mb"].as_u64())
            }) {
                vram_gb = (mb as f64) / 1024.0;
            } else if let Some(bytes) = rec
                .run_summary
                .as_ref()
                .and_then(|v| v["peak_metal_allocated_bytes"].as_u64())
            {
                vram_gb = (bytes as f64) / 1_000_000_000.0;
            }
        }
        vram_gbs[i] = vram_gb;
        let norm_vram = (vram_gb / max_scale_val).clamp(0.0, 1.0);
        vram_ys[i] = (chart_floor_y - norm_vram * chart_h).round();
    }
    let min_vram_gb = vram_gbs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_vram_gb = vram_gbs.iter().copied().fold(0.0_f64, f64::max);

    let _ = writeln!(
        svg,
        r##"<!-- ================= RIGHT CARD: FULL LADDER (0.6B–32B) PASS@1 LIFT & APPLE SILICON VRAM PARETO ================= -->
<g transform="translate(612, 24)">
  <rect x="0" y="0" width="564" height="312" class="card"/>

  <!-- Header Title -->
  <text x="24" y="32" font-size="11" font-weight="700" class="mono bright">FULL LADDER (0.6B–32B) PASS@1 LIFT &amp; APPLE SILICON VRAM PARETO</text>
  <text x="540" y="46" text-anchor="end" font-size="9" font-weight="600" class="mono" fill="#fbbf24">Peak Metal VRAM ({min_vram_gb:.0} GB → {max_vram_gb:.0} GB)</text>

  <!-- Y-Axis Gridlines (0 to 80 scale for both Pass@1 % and VRAM GB) -->
  <text x="44" y="74" text-anchor="end" font-size="9" class="mono subtle">80%</text>
  <text x="540" y="74" text-anchor="end" font-size="9" class="mono subtle">80G</text>
  <line x1="50" y1="70" x2="530" y2="70" stroke="#1f242d" stroke-width="1"/>

  <text x="44" y="119" text-anchor="end" font-size="9" class="mono subtle">60%</text>
  <text x="540" y="119" text-anchor="end" font-size="9" class="mono subtle">60G</text>
  <line x1="50" y1="115" x2="530" y2="115" stroke="#1f242d" stroke-width="1" stroke-dasharray="2,2"/>

  <text x="44" y="164" text-anchor="end" font-size="9" class="mono subtle">40%</text>
  <text x="540" y="164" text-anchor="end" font-size="9" class="mono subtle">40G</text>
  <line x1="50" y1="160" x2="530" y2="160" stroke="#1f242d" stroke-width="1" stroke-dasharray="2,2"/>

  <text x="44" y="209" text-anchor="end" font-size="9" class="mono subtle">20%</text>
  <text x="540" y="209" text-anchor="end" font-size="9" class="mono subtle">20G</text>
  <line x1="50" y1="205" x2="530" y2="205" stroke="#1f242d" stroke-width="1" stroke-dasharray="2,2"/>

  <text x="44" y="254" text-anchor="end" font-size="9" class="mono subtle">0%</text>
  <text x="540" y="254" text-anchor="end" font-size="9" class="mono subtle">0G</text>
  <line x1="50" y1="250" x2="530" y2="250" stroke="#27272a" stroke-width="1"/>

  <!-- Horizontal Apple Silicon Memory Tiers -->
  <!-- 16 GB Mac -->
  <line x1="50" y1="214" x2="530" y2="214" stroke="#1e293b" stroke-width="1"/>
  <text x="48" y="211" font-size="8" class="mono" fill="#64748b">16 GB</text>

  <!-- 24 GB Mac -->
  <line x1="50" y1="196" x2="530" y2="196" stroke="#334155" stroke-width="1"/>
  <text x="48" y="193" font-size="8" class="mono" fill="#94a3b8">24 GB</text>

  <!-- 36 GB Mac -->
  <line x1="50" y1="169" x2="530" y2="169" stroke="#334155" stroke-width="1"/>
  <text x="48" y="166" font-size="8" class="mono" fill="#94a3b8">36 GB</text>

  <!-- 64 GB Mac ceiling -->
  <line x1="50" y1="106" x2="530" y2="106" stroke="#475569" stroke-width="1"/>
  <text x="48" y="103" font-size="8" class="mono" fill="#cbd5e1">64 GB Mac ceiling</text>"##,
        min_vram_gb = min_vram_gb,
        max_vram_gb = max_vram_gb,
    );

    // Grouped vertical bars for each of the 6 ladder sizes
    for (i, &size_name) in SIZES.iter().enumerate() {
        let mut base_r = None;
        let mut adapt_r = None;
        let mut gate_state = GateState::NotEvaluated;
        let mut trained = false;

        if let Some((_, rec)) = entries.iter().find(|(r, _)| r.size == size_name) {
            let gf = gate_of(rec);
            gate_state = gf.state;
            trained = gf.trained;
            let (b, a) = heldout_scores(rec);
            if let Some(b) = b {
                base_r = Some(b.rate);
            }
            if adapter_evidence(rec) == AdapterEvidence::Current
                && let Some(a) = a
            {
                adapt_r = Some(a.rate);
            }
        }

        let gx = 96.0 + i as f64 * 72.0;
        let x_base = gx;
        let x_adapt = gx + 18.0;

        // Base bar
        if let Some(b_rate) = base_r {
            let b_pct = b_rate * 100.0;
            let norm_b = (b_pct / max_scale_val).clamp(0.02, 1.0);
            let h_base = norm_b * chart_h;
            let y_base = chart_floor_y - h_base;
            let _ = writeln!(
                svg,
                r##"  <!-- {size_name} Base -->
  <text x="{lbl_x:.1}" y="{lbl_y:.1}" text-anchor="middle" font-size="8" class="mono subtle">{b_pct:.1}</text>
  <rect x="{x_base:.1}" y="{y_base:.1}" width="14" height="{h_base:.1}" rx="3" fill="#10b981"/>"##,
                size_name = size_name,
                lbl_x = x_base + 7.0,
                lbl_y = y_base - 3.0,
                x_base = x_base,
                y_base = y_base,
                h_base = h_base,
                b_pct = b_pct,
            );
        } else {
            let _ = writeln!(
                svg,
                r##"  <rect x="{x_base:.1}" y="{y_base:.1}" width="14" height="20" rx="3" stroke="#3f3f46" stroke-dasharray="2,2" fill="none"/>"##,
                x_base = x_base,
                y_base = chart_floor_y - 20.0,
            );
        }

        // Adapter bar
        if let Some(a_rate) = adapt_r {
            let a_pct = a_rate * 100.0;
            let norm_a = (a_pct / max_scale_val).clamp(0.02, 1.0);
            let h_adapt = norm_a * chart_h;
            let y_adapt = chart_floor_y - h_adapt;
            let adapt_fill = if gate_state == GateState::Passed {
                "#34d399"
            } else {
                "#fbbf24"
            };
            let _ = writeln!(
                svg,
                r##"  <!-- {size_name} Adapter -->
  <text x="{lbl_x:.1}" y="{lbl_y:.1}" text-anchor="middle" font-size="8" font-weight="700" class="mono bright">{a_pct:.1}</text>
  <rect x="{x_adapt:.1}" y="{y_adapt:.1}" width="14" height="{h_adapt:.1}" rx="3" fill="{adapt_fill}"/>"##,
                size_name = size_name,
                lbl_x = x_adapt + 7.0,
                lbl_y = y_adapt - 3.0,
                x_adapt = x_adapt,
                y_adapt = y_adapt,
                h_adapt = h_adapt,
                adapt_fill = adapt_fill,
                a_pct = a_pct,
            );
        } else if size_name == "32B" || !trained {
            // Honest empty dashed box for untrained rungs (e.g. 32B exceeds 64 GB Mac memory)
            let _ = writeln!(
                svg,
                r##"  <!-- {size_name} Adapter (Untrained) -->
  <text x="{lbl_x:.1}" y="{lbl_y1:.1}" text-anchor="middle" font-size="7" class="mono" fill="#fbbf24">66G &gt; 64G</text>
  <text x="{lbl_x:.1}" y="{lbl_y2:.1}" text-anchor="middle" font-size="7" class="mono" fill="#71717a">Untrained</text>
  <rect x="{x_adapt:.1}" y="{y_adapt:.1}" width="14" height="24" rx="3" stroke="#52525b" stroke-dasharray="3,3" fill="none"/>"##,
                size_name = size_name,
                lbl_x = x_adapt + 7.0,
                lbl_y1 = chart_floor_y - 36.0,
                lbl_y2 = chart_floor_y - 27.0,
                x_adapt = x_adapt,
                y_adapt = chart_floor_y - 24.0,
            );
        } else {
            let (state_lbl, state_color, state_stroke) = match gate_state {
                GateState::Failed => ("Failed", "#f87171", "#ef4444"),
                GateState::NeedsReGate => ("Re-gate", "#fbbf24", "#f59e0b"),
                GateState::NotEvaluated | GateState::Passed => {
                    ("Eval Pending", "#94a3b8", "#334155")
                }
            };
            let _ = writeln!(
                svg,
                r##"  <text x="{lbl_x:.1}" y="{lbl_y:.1}" text-anchor="middle" font-size="7" class="mono" fill="{state_color}">{state_lbl}</text>
  <rect x="{x_adapt:.1}" y="{y_adapt:.1}" width="14" height="20" rx="3" stroke="{state_stroke}" stroke-dasharray="3,3" fill="none"/>"##,
                lbl_x = x_adapt + 7.0,
                lbl_y = chart_floor_y - 24.0,
                x_adapt = x_adapt,
                y_adapt = chart_floor_y - 20.0,
                state_color = state_color,
                state_lbl = state_lbl,
                state_stroke = state_stroke,
            );
        }

        // Size label below floor
        let _ = writeln!(
            svg,
            r##"  <text x="{x_mid:.1}" y="268" text-anchor="middle" font-size="11" font-weight="600" class="mono bright">{size_name}</text>"##,
            x_mid = gx + 16.0,
            size_name = size_name,
        );
    }

    // Overlaid stepped Peak Metal VRAM line (computed dynamically from entries or tier fallbacks)
    let [y0, y1, y2, y3, y4, y5] = vram_ys;
    let _ = writeln!(
        svg,
        r##"  <!-- Stepped Peak Metal VRAM Line -->
  <path d="M 75 {y0:.0} H 148 V {y1:.0} H 220 V {y2:.0} H 292 V {y3:.0} H 364 V {y4:.0} H 436 V {y5:.0} H 520" fill="none" stroke="#fbbf24" stroke-width="2"/>
  <circle cx="112" cy="{y0:.0}" r="3.5" fill="#fbbf24"/>
  <circle cx="184" cy="{y1:.0}" r="3.5" fill="#fbbf24"/>
  <circle cx="256" cy="{y2:.0}" r="3.5" fill="#fbbf24"/>
  <circle cx="328" cy="{y3:.0}" r="3.5" fill="#fbbf24"/>
  <circle cx="400" cy="{y4:.0}" r="3.5" fill="#fbbf24"/>
  <circle cx="472" cy="{y5:.0}" r="3.5" fill="#fbbf24"/>

  <!-- Legend -->
  <g transform="translate(70, 290)">
    <rect x="0" y="0" width="10" height="10" rx="2" fill="#10b981"/>
    <text x="16" y="9" font-size="10" class="mono subtle">Base Pass@1</text>

    <rect x="110" y="0" width="10" height="10" rx="2" fill="#34d399"/>
    <text x="126" y="9" font-size="10" class="mono subtle">Adapter Pass@1</text>

    <line x1="240" y1="5" x2="260" y2="5" stroke="#fbbf24" stroke-width="2"/>
    <text x="268" y="9" font-size="10" font-weight="600" class="mono" fill="#fbbf24">Peak Metal VRAM ({min_vram_gb:.0} GB → {max_vram_gb:.0} GB)</text>
  </g>
</g>
</svg>"##,
        y0 = y0,
        y1 = y1,
        y2 = y2,
        y3 = y3,
        y4 = y4,
        y5 = y5,
        min_vram_gb = min_vram_gb,
        max_vram_gb = max_vram_gb,
    );

    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_model_assets_valid_svg_and_metrics() {
        let rec = RunRecords {
            has_adapter: true,
            training_manifest: Some(serde_json::json!({
                "rank": 16,
                "alpha": 32.0,
                "seq_len": 3072,
                "n_layers": 36,
                "grad_accum": 8
            })),
            run_summary: Some(serde_json::json!({
                "avg_train_loss": 0.52,
                "avg_val_loss_last_epoch": 0.69,
                "peak_metal_allocated_bytes": 38761791488u64
            })),
            ..Default::default()
        };
        let cx = CardContext {
            size: "4B",
            base_pin: "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c", // gitleaks:allow
            namespace: "brbrainerd",
        };
        let assets = render_model_assets(&cx, &rec);
        assert_eq!(assets.len(), 1);
        let a = &assets[0];
        assert_eq!(a.rel_path, "assets/scorecard.svg");
        assert!(a.content.starts_with("<svg"));
        assert!(a.content.contains(r#"viewBox="0 0 1200 320""#));
        assert!(a.content.trim_end().ends_with("</svg>"));
        assert!(
            a.content
                .contains("QLORA TRAINING LOSS &amp; EMA CONVERGENCE")
        );
        assert!(
            a.content
                .contains("SPOKE GATE &amp; PAIRED MCNEMAR VISUAL DELTAS")
        );
        assert!(a.content.contains("spoke slot navigation strip"));
        // Assert zero-duplication contract: no code diff box, no text table
        assert!(!a.content.contains("Compiler-Verified Before/After"));
        assert!(!a.content.contains("Optimizer steps"));
    }

    #[test]
    fn scorecard_empty_state_untrained() {
        let rec = RunRecords {
            has_adapter: false,
            ..Default::default()
        };
        let cx = CardContext {
            size: "32B",
            base_pin: "test-pin",
            namespace: "brbrainerd",
        };
        let assets = render_model_assets(&cx, &rec);
        assert_eq!(assets.len(), 1);
        assert!(assets[0].content.contains("Model Weights Not Yet Trained"));
        assert!(
            assets[0]
                .content
                .contains("exceeds 64 GB Mac unified memory")
        );
    }

    #[test]
    fn render_index_assets_valid_svg_and_all_sizes() {
        let entries: Vec<(SizeSyncRow, RunRecords)> = SIZES
            .iter()
            .map(|&s| {
                (
                    SizeSyncRow {
                        size: s.to_string(),
                        base_pin: "test-pin".to_string(),
                        repo_id: format!("brbrainerd/vox-mens-qwen3-{}", s.to_ascii_lowercase()),
                        local_dir: format!("mens/runs/qwen3-{}", s.to_ascii_lowercase()),
                        sync_status: crate::mens::hub_sync::SizeSyncStatus::InSync,
                        gate_status: Some("passed".to_string()),
                        final_loss: Some(0.5),
                        measured_peak_mb: Some(30000),
                        action_taken: None,
                        latest_version: None,
                    },
                    RunRecords {
                        has_adapter: s != "32B",
                        ..Default::default()
                    },
                )
            })
            .collect();
        let assets = render_index_assets("brbrainerd", &entries);
        assert_eq!(assets.len(), 1);
        let a = &assets[0];
        assert_eq!(a.rel_path, "assets/overview.svg");
        assert!(a.content.starts_with("<svg"));
        assert!(a.content.contains(r#"viewBox="0 0 1200 360""#));
        assert!(a.content.trim_end().ends_with("</svg>"));
        // Multi-spoke and knowledge router topology present
        assert!(
            a.content
                .contains("VOX MENS BRAIN — HUB, RETRIEVAL &amp; MULTI-SPOKE TOPOLOGY")
        );
        assert!(
            a.content
                .contains("FULL LADDER (0.6B–32B) PASS@1 LIFT &amp; APPLE SILICON VRAM PARETO")
        );
        assert!(a.content.contains("vox-lang"));
        assert!(a.content.contains("rust"));
        assert!(a.content.contains("harness"));
        assert!(a.content.contains("tool-selection"));
        assert!(a.content.contains("argument-generation"));
        assert!(a.content.contains("research-expert"));
        assert!(a.content.contains("populi-meta"));
        assert!(a.content.contains("research"));
        assert!(a.content.contains("rocks"));
        assert!(a.content.contains("0.6B"));
        assert!(a.content.contains("32B"));
        // Assert zero-duplication contract: matrix table stripped
        assert!(
            !a.content
                .contains("Size × Spoke Evaluation &amp; Gate Matrix")
        );
    }

    #[test]
    fn scorecard_mcnemar_uses_statistical_regression_not_raw_drop() {
        let rec = RunRecords {
            has_adapter: true,
            collateral: Some(serde_json::json!({
                "reasoning_slices": {
                    "alpha": 0.05,
                    "benches": {
                        "gsm8k_think": {
                            "accuracy": { "pre": 0.3516, "post": 0.3672, "p_value": 0.8746, "regressed": false }
                        },
                        "math500_think": {
                            "accuracy": { "pre": 0.6719, "post": 0.5938, "p_value": 0.2210, "regressed": false }
                        },
                        "vox_think": {
                            "accuracy": { "pre": 0.6172, "post": 0.5078, "p_value": 0.1405, "regressed": false }
                        }
                    }
                }
            })),
            gate_receipt: Some(serde_json::json!({ "passed": true })),
            ..Default::default()
        };
        let cx = CardContext {
            size: "0.6B",
            base_pin: "test-pin",
            namespace: "brbrainerd",
        };
        let assets = render_model_assets(&cx, &rec);
        assert!(
            !assets[0].content.contains("REGRESSED"),
            "scorecard.svg must not mark p >= 0.05 slices as REGRESSED"
        );
        assert!(assets[0].content.contains("67.2%→59.4% (p=0.22)"));
    }
}
