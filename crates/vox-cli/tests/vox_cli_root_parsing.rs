//! Smoke: `VoxCliRoot` parses global flags + `completions` / Latin groupings.

use clap::Parser;
use vox_cli::VoxCliRoot;

#[test]
fn parse_completions_bash() {
    VoxCliRoot::try_parse_from(["vox", "completions", "bash"]).expect("completions bash");
}

#[test]
fn parse_global_color_and_build() {
    let r = VoxCliRoot::try_parse_from(["vox", "--color", "never", "build", "foo.vox"])
        .expect("build with global color");
    assert!(r.global.color.is_some());
}

#[test]
fn parse_fabrica_build() {
    VoxCliRoot::try_parse_from(["vox", "fabrica", "build", "foo.vox"]).expect("fabrica build");
}
