use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    TextInfer,
    ImageGen,
    SpeechTranscribe,
    TrainQLoRA,
    Embed,
    VoxScript,
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextInfer => write!(f, "text_infer"),
            Self::ImageGen => write!(f, "image_gen"),
            Self::SpeechTranscribe => write!(f, "speech_transcribe"),
            Self::TrainQLoRA => write!(f, "train_qlora"),
            Self::Embed => write!(f, "embed"),
            Self::VoxScript => write!(f, "vox_script"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub kind: TaskKind,
    pub model_id: Option<String>,
    pub min_vram_mb: Option<u32>,
    pub priority: u8,
    pub timeout_secs: u64,
    pub payload_b64: String,
    pub source_blake3_hex: Option<String>,
    pub required_labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub node_id: String,
    pub success: bool,
    pub output_b64: String,
    pub duration_ms: u64,
    pub payload_blake3_hex: Option<String>,
    pub worker_ed25519_sig_b64: Option<String>,
}
