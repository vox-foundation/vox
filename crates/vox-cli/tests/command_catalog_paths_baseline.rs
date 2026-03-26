//! Stable sorted list of `vox …` command paths from [`vox_cli::command_catalog::build_catalog`].
//!
//! Intentional command-surface changes: refresh the fixture:
//! `UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline`

use std::fs;
use std::path::Path;

#[test]
fn command_catalog_paths_match_baseline() {
    let cat = vox_cli::command_catalog::build_catalog();
    let mut paths: Vec<String> = cat.entries.iter().map(|e| e.path.join("/")).collect();
    paths.sort();
    let current = paths.join("\n") + "\n";

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/command_catalog_paths_baseline.txt");

    if std::env::var("UPDATE_CLI_CATALOG_BASELINE").ok().as_deref() == Some("1") {
        if let Some(parent) = fixture.parent() {
            fs::create_dir_all(parent).expect("create fixtures dir");
        }
        fs::write(&fixture, &current).expect("write baseline fixture");
        panic!("wrote {}; commit this file", fixture.display());
    }

    let expected = fs::read_to_string(&fixture)
        .unwrap_or_else(|e| panic!("missing {}: {e}", fixture.display()));
    assert_eq!(
        current, expected,
        "command catalog paths changed; review diff and run UPDATE_CLI_CATALOG_BASELINE=1 if intentional"
    );
}
