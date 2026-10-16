//! Goldens for the canonical formatter `fmt`.

use vox_diagram::fmt;

const CANONICAL: &str = include_str!("fixtures/valid/canonical.vdg");

#[test]
fn canonical_example_is_fixed_point() {
    assert_eq!(fmt(CANONICAL).as_deref(), Some(CANONICAL));
}
