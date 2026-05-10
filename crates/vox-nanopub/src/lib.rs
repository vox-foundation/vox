//! SCIENTIA nanopublication builder: TriG emission and Ed25519 signing.

pub mod network;
pub mod signing;
pub mod trig;

pub use network::{NanopubNetworkConfig, PublishResult, publish_stub};
pub use signing::{SignedNanopub, sign_nanopub, verify_nanopub};
pub use trig::{NanopubDocument, NanopubGraphs, build_nanopub};
