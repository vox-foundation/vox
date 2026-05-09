//! Sanity test for the TunnelBackend trait shape.

use vox_share::{BackendKind, TunnelBackend};

#[test]
fn backend_kind_round_trips_via_str() {
    assert_eq!("lan".parse::<BackendKind>().unwrap(), BackendKind::Lan);
    assert_eq!("cloudflare".parse::<BackendKind>().unwrap(), BackendKind::Cloudflare);
    assert_eq!("localhost-run".parse::<BackendKind>().unwrap(), BackendKind::LocalhostRun);
    assert_eq!("tailscale".parse::<BackendKind>().unwrap(), BackendKind::Tailscale);
    assert!("frobnicate".parse::<BackendKind>().is_err());
}

#[test]
fn backend_kind_default_is_cloudflare() {
    assert_eq!(BackendKind::default(), BackendKind::Cloudflare);
}

/// Compile-time check that the trait is object-safe (we'll be Box<dyn TunnelBackend>-ing it).
fn _assert_object_safe(_: Box<dyn TunnelBackend>) {}
