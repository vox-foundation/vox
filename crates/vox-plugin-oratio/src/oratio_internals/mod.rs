//! Oratio internal types copied verbatim from vox-oratio for use in vox-plugin-oratio.
//! These are separate from the backends so that the plugin cdylib does not depend on the full
//! vox-oratio crate.

pub mod acoustic_preprocess;
pub mod contextual_bias;
pub mod domain_mode;
pub mod runtime_config;
pub mod speech_lexicon;
