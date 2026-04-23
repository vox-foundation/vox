use crate::task::TaskKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelAdvertisement {
    pub model_id: String,
    pub task_kinds: Vec<TaskKind>,
    pub vram_required_mb: u32,
    pub is_loaded: bool,
    pub quantization: Option<String>,
}
