//! CloudSync extension point — cloud provider integrations (HF Hub,
//! S3, etc.) for syncing models and artifacts.

use abi_stable::{sabi_trait, std_types::*};

pub const CLOUD_SYNC_REVISION: u32 = 1;

#[sabi_trait]
pub trait CloudSync: Send + Sync {
    fn revision(&self) -> u32 {
        CLOUD_SYNC_REVISION
    }
    fn provider_id(&self) -> RString;
    fn upload(&self, local_path: RStr<'_>, remote_uri: RStr<'_>) -> RResult<(), RBoxError>;
    fn download(&self, remote_uri: RStr<'_>, local_path: RStr<'_>) -> RResult<(), RBoxError>;
    fn list_remote_json(&self, remote_prefix: RStr<'_>) -> RResult<RString, RBoxError>;
}
