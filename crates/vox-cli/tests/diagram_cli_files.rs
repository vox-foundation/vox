//! `vox diagram` file-writing paths (`fmt --write`, `-o`): symlinks, read-only targets, an output
//! that is the input, no temp litter. Plus the docs claims that review fixes depend on.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const VALID: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";
const NON_CANONICAL: &str = "graph {\nnode a:\"A\"\n   node b: \"B\"\n a -> b\n}\n";

fn vox(args: &[&str], stdin: Option<&str>) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("vox should spawn");
    let mut pipe = child.stdin.take().expect("stdin pipe");
    if let Some(s) = stdin {
        pipe.write_all(s.as_bytes()).expect("write stdin");
    }
    drop(pipe);
    let o = child.wait_with_output().expect("vox should run");
    (
        o.status.code().expect("exit code"),
        String::from_utf8(o.stdout).expect("utf8 stdout"),
        String::from_utf8(o.stderr).expect("utf8 stderr"),
    )
}

fn put(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write fixture");
    p.display().to_string()
}

fn entries(dir: &Path) -> usize {
    std::fs::read_dir(dir).unwrap().count()
}

#[cfg(unix)]
fn is_root_or_ignores_modes(path: &str) -> bool {
    std::fs::OpenOptions::new().append(true).open(path).is_ok()
}

#[cfg(unix)]
#[test]
fn fmt_write_writes_through_a_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let real = put(dir.path(), "real.vdg", NON_CANONICAL);
    let link = dir.path().join("link.vdg");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let (code, _, err) = vox(&["diagram", "fmt", "--write", link.to_str().unwrap()], None);
    assert_eq!(code, 0, "{err}");
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
    assert_ne!(std::fs::read_to_string(&real).unwrap(), NON_CANONICAL);
    assert_eq!(vox(&["diagram", "fmt", "--check", &real], None).0, 0);
    assert_eq!(entries(dir.path()), 2, "temp sibling left behind");
}

#[cfg(unix)]
#[test]
fn fmt_write_keeps_the_source_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = put(dir.path(), "a.vdg", NON_CANONICAL);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(vox(&["diagram", "fmt", "--write", &path], None).0, 0);
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[cfg(unix)]
#[test]
fn fmt_write_refuses_a_read_only_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = put(dir.path(), "a.vdg", NON_CANONICAL);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
    if is_root_or_ignores_modes(&path) {
        return;
    }
    let (code, _, err) = vox(&["diagram", "fmt", "--write", &path], None);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("read-only"), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), NON_CANONICAL);
    assert_eq!(entries(dir.path()), 1, "temp sibling left behind");
}

#[test]
fn output_may_not_be_the_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = put(dir.path(), "in.vdg", VALID);
    let dotted = format!("{}/./in.vdg", dir.path().display());
    for sub in ["export", "render", "import"] {
        for out in [&path, &dotted] {
            let (code, stdout, err) = vox(&["diagram", sub, "-o", out, &path], None);
            assert_eq!((code, stdout.as_str()), (2, ""), "{sub} -o {out}: {err}");
            assert!(err.contains("input"), "{err}");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), VALID, "{sub}");
        }
    }
    #[cfg(unix)]
    {
        let link = dir.path().join("alias.vdg");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        let (code, _, err) = vox(
            &["diagram", "export", "-o", link.to_str().unwrap(), &path],
            None,
        );
        assert_eq!(code, 2, "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), VALID);
    }
}

#[test]
fn output_file_is_written_whole_without_temp_litter() {
    let dir = tempfile::tempdir().unwrap();
    let input = put(dir.path(), "in.vdg", VALID);
    let out = dir.path().join("out.svg");
    let (code, _, err) = vox(
        &["diagram", "render", "-o", out.to_str().unwrap(), &input],
        None,
    );
    assert_eq!(code, 0, "{err}");
    assert!(std::fs::read_to_string(&out).unwrap().starts_with("<svg"));
    assert_eq!(entries(dir.path()), 2, "temp sibling left behind");
}

#[cfg(unix)]
#[test]
fn output_refuses_a_read_only_target_and_follows_symlinks() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let input = put(dir.path(), "in.vdg", VALID);
    let real = put(dir.path(), "real.svg", "old");
    let link = dir.path().join("link.svg");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let args = ["diagram", "render", "-o", link.to_str().unwrap(), &input];
    assert_eq!(vox(&args, None).0, 0);
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
    assert!(std::fs::read_to_string(&real).unwrap().starts_with("<svg"));

    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o444)).unwrap();
    if is_root_or_ignores_modes(&real) {
        return;
    }
    let before = std::fs::read_to_string(&real).unwrap();
    let (code, _, err) = vox(&args, None);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("read-only"), "{err}");
    assert_eq!(std::fs::read_to_string(&real).unwrap(), before);
}

#[test]
fn fmt_check_on_stdin_suggests_a_command_that_exists() {
    let (code, _, err) = vox(&["diagram", "fmt", "--check", "-"], Some(NON_CANONICAL));
    assert_eq!(code, 1);
    assert!(!err.contains("--write <stdin>"), "{err}");
    assert!(err.contains("`vox diagram fmt -`"), "{err}");
}

#[test]
fn docs_send_agents_to_the_json_envelope_for_fixes() {
    let agents = include_str!("../../../AGENTS.md");
    let page = include_str!("../../../docs/src/reference/vox-diagram.md");
    let cli = include_str!("../../../docs/src/reference/cli.md");
    assert!(agents.contains("vox --json diagram check"), "AGENTS.md");
    assert!(page.contains("vox --json diagram check"), "reference page");
    for (name, text) in [("reference page", page), ("cli.md", cli)] {
        assert!(!text.contains("same envelope"), "{name}");
        assert!(text.contains("`data` field"), "{name}");
    }
}
