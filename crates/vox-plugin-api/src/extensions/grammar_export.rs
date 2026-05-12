//! GrammarExport extension point — export grammar via plugin.
//! Used by vox-plugin-grammar-export.

use abi_stable::{sabi_trait, std_types::*};

pub const GRAMMAR_EXPORT_REVISION: u32 = 1;

#[sabi_trait]
pub trait GrammarExport: Send + Sync {
    fn revision(&self) -> u32 {
        GRAMMAR_EXPORT_REVISION
    }
    fn export(&self, config_json: RStr<'_>) -> RResult<RString, RBoxError>;
    fn grammar_version_matches_compiler(&self) -> bool;
}
