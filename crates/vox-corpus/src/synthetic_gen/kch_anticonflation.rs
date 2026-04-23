use super::{SyntheticGenConfig, emit_line};
use serde_json::json;
use std::io::Write;

pub fn generate_kch_anticonflation_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    if !cfg.emit_kch_anticonflation {
        return Ok(0);
    }

    let mut count = 0;

    // Stub example based on the proximity problem we detected
    let prompt = "How do I process a combat round in Dystopia MUD?";

    // Negative (hallucinated conflation)
    let bad_response = json!({
        "status": "rejected",
        "reason": "The model hallucinated `combatRoundResolver` instead of using the canonical `resolveArenaRound`."
    });

    emit_line(
        out,
        prompt,
        &bad_response,
        "lane_kch_anticonflation",
        "negative_preference",
    )?;
    count += 1;

    Ok(count)
}
