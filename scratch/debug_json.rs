use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatmlTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainingPair {
    #[serde(alias = "instruction")]
    pub prompt: Option<String>,
    #[serde(alias = "output")]
    pub response: Option<String>,
    pub turns: Option<Vec<ChatmlTurn>>,
    pub rating: Option<u8>,
    pub category: Option<String>,
    pub difficulty: Option<u8>,
    pub lane: Option<String>,
    pub response_mode: Option<String>,
    pub task_family: Option<String>,
    pub interruption_decision: Option<String>,
    pub agent_trust_score: Option<f64>,
}

fn main() {
    let json = r#"{"category":"import","difficulty":1,"instruction":"Write Vox code demonstrating example","lane":"vox_codegen","origin":"human","output":"// Minimal notify demo — same handler shape as `examples/golden/mobile_camera.vox`.\n\nimport std.mobile\n\ncomponent App() {\n    view:\n        <button onclick={fn() {\n            mobile.notify(\"Hello\", \"From Vox!\")\n        }}>\"Notify Me\"</button>\n}\n","prompt":"Write Vox code demonstrating example","rating":5,"response":"// Minimal notify demo — same handler shape as `examples/golden/mobile_camera.vox`.\n\nimport std.mobile\n\ncomponent App() {\n    view:\n        <button onclick={fn() {\n            mobile.notify(\"Hello\", \"From Vox!\")\n        }}>\"Notify Me\"</button>\n}\n","response_mode":"code_only","schema_version":"vox_dogfood_v1","source":"examples\\golden\\mobile_test.vox","task_family":"vox_codegen"}"#;
    let parsed: Result<TrainingPair, _> = serde_json::from_str(json);
    match parsed {
        Ok(p) => println!("Parsed successfully: {:?}", p),
        Err(e) => println!("Parse error: {}", e),
    }
}
