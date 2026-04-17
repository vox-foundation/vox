use std::io::Write;
use anyhow::Result;
#[cfg(feature = "database")]
use vox_db::VoxDb;

use serde_json::json;

#[cfg(feature = "database")]
pub struct DogfoodExporter<'a> {
    db: &'a VoxDb,
}

#[cfg(feature = "database")]
impl<'a> DogfoodExporter<'a> {
    pub fn new(db: &'a VoxDb) -> Self {
        Self { db }
    }

    /// Export agent traces with feedback to JSONL.
    pub async fn export_agent_traces(&self, limit: i64, out: &mut impl Write) -> Result<usize> {
        let pairs = self.db.get_training_data(limit).await?;
        let mut count = 0;

        for pair in pairs {
            // Priority 1: Use correction text if available (RLHF)
            // Priority 2: Use original response if rating is high (SFT)
            let effective_response = if pair.correction.as_ref().map_or(false, |c: &String| !c.is_empty()) {
                pair.correction.clone()
            } else if pair.rating.unwrap_or(0) >= 4 {
                Some(pair.response.clone())
            } else {
                None
            };

            if let Some(resp) = effective_response {
                let tp = json!({
                    "prompt": pair.prompt,
                    "response": resp,
                    "rating": pair.rating,
                    "feedback_type": pair.feedback_type,
                    "lane": "vox_dogfood_agent",
                    "task_family": "agent_trace"
                });
                writeln!(out, "{}", serde_json::to_string(&tp)?)?;
                count += 1;
            }
        }

        Ok(count)
    }
}
