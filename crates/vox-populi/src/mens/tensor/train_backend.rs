//! Mens **training backend** selection (`lora` vs `qlora`).
//!
//! SSOT: `vox schola train --backend`. [`PopuliTrainBackend::BurnLora`] → Burn + wgpu LoRA on Vox JSONL.
//! [`PopuliTrainBackend::CandleQlora`] → Candle + **qlora-rs** NF4 on LM head + mmap `f32` embeds + HF tokenizer.
//!
//! **Execution kernel** (contract/planner vocabulary): type alias only — behavior is still this enum.

use std::fmt;
use std::str::FromStr;

/// Type alias: planner “execution kernel” == CLI backend selection (temporary convergence).
pub type ExecutionKernel = PopuliTrainBackend;

/// Which trainer implementation to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopuliTrainBackend {
    /// Burn + wgpu LoRA on [`vox_tensor::data::VoxTokenizer`] JSONL.
    #[default]
    BurnLora,
    /// Candle + qlora-rs: NF4 LM head + LoRA; frozen HF embedding table in `f32` (mmap).
    CandleQlora,
}

impl fmt::Display for PopuliTrainBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BurnLora => write!(f, "lora"),
            Self::CandleQlora => write!(f, "qlora"),
        }
    }
}

impl FromStr for PopuliTrainBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "lora" | "burn" | "burn-lora" => Ok(Self::BurnLora),
            "qlora" | "candle" | "candle-qlora" => Ok(Self::CandleQlora),
            _ => Err(format!(
                "unknown training backend '{s}': expected `lora` or `qlora`"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PopuliTrainBackend;
    use std::str::FromStr;

    #[test]
    fn parse_lora_aliases() {
        assert_eq!(
            PopuliTrainBackend::from_str("lora").unwrap(),
            PopuliTrainBackend::BurnLora
        );
        assert_eq!(
            PopuliTrainBackend::from_str("burn-lora").unwrap(),
            PopuliTrainBackend::BurnLora
        );
    }

    #[test]
    fn parse_qlora_aliases() {
        assert_eq!(
            PopuliTrainBackend::from_str("qlora").unwrap(),
            PopuliTrainBackend::CandleQlora
        );
    }
}
