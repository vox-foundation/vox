//! Scientia publication lifecycle MCP tools.
//!
//! **Naming:** Request/response **Rust** types historically used a `VoxScientia*` prefix. Prefer
//! adding **`Scientia*`**-prefixed [`type`] aliases (same `Serialize`/`Deserialize` shape) for new
//! code; MCP **tool names** and JSON field names are SSOT elsewhere and are unchanged by type
//! aliases. See mdBook `docs/src/api/vox-mcp.md` § MCP tool and Rust type naming.

mod common;
mod external;
mod lifecycle;
mod media;
mod preflight;
mod scholar;
mod syndication;

pub use external::*;
pub use lifecycle::*;
pub use media::*;
pub use preflight::*;
pub use scholar::*;
pub use syndication::*;
