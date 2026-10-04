use std::path::Path;

#[test]
fn test_fmt_idempotence_across_golden_corpus() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // CARGO_MANIFEST_DIR is crates/vox-compiler, we need to go up to the repo root
    let root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let golden_dir = root.join("examples").join("golden");

    let mut count = 0;

    // We do a recursive walk manually to avoid pulling in walkdir just for a test
    let mut dirs_to_visit = vec![golden_dir];

    while let Some(dir) = dirs_to_visit.pop() {
        if !dir.exists() || !dir.is_dir() {
            continue;
        }

        let entries = std::fs::read_dir(&dir).unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                dirs_to_visit.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("vox") {
                let source = std::fs::read_to_string(&path).unwrap();
                let once = vox_compiler::fmt::format(&source);
                let twice = vox_compiler::fmt::format(&once);

                assert_eq!(
                    once,
                    twice,
                    "Formatting is not idempotent for file: {}\n\nFirst pass:\n{}\n\nSecond pass:\n{}",
                    path.display(),
                    once,
                    twice
                );

                count += 1;
            }
        }
    }

    assert!(
        count >= 43,
        "Expected to test at least 43 golden files, found {}",
        count
    );
}
