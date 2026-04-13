use candle_core::Tensor;

#[derive(Debug, Clone)]
pub struct QloraTrainingResume {
    pub start_epoch: usize,
    pub global_step: u32,
    pub resume_pair_offset: usize,
    pub resume_shuffled_indices: Option<Vec<usize>>,
}

pub struct EncodedTrainStep {
    pub raw_token_len: usize,
    pub ids: Vec<u32>,
    pub prefix_len: usize,
    pub trunc_offset: usize,
    pub sample_weight: f64,
}

pub enum TryEncodeOutcome {
    Encoded(EncodedTrainStep),
    SkipCurriculum,
    SkipShortSeq,
}

pub enum MaskedCeForward {
    NoSupervision,
    NonFinite {
        kind: &'static str,
        mask_sum: f32,
    },
    Finite {
        loss: Tensor,
        loss_scalar: f32,
        supervised_tokens: u64,
        theoretical_tokens: u64,
    },
}
