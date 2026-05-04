//! ScriptExecutor extension point — sandboxed execution of `.vox` and
//! similar scripts. Used by `vox run` and embedder execution APIs.

use abi_stable::{sabi_trait, std_types::*};

pub const SCRIPT_EXECUTOR_REVISION: u32 = 1;

#[sabi_trait]
pub trait ScriptExecutor: Send + Sync {
    fn revision(&self) -> u32 {
        SCRIPT_EXECUTOR_REVISION
    }
    fn execute(&self, script_path: RStr<'_>, args_json: RStr<'_>) -> RResult<RString, RBoxError>;
    fn validate(&self, script_path: RStr<'_>) -> RResult<RString, RBoxError>;
}
