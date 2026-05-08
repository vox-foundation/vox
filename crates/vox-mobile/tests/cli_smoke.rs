use assert_cmd::Command;

#[test]
fn version_flag_prints_semver() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::starts_with("vox-mobile "));
}

#[test]
fn help_lists_subcommands() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor"))
        .stdout(predicates::str::contains("build"));
}
