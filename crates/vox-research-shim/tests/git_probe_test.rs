use vox_research_shim::research::domain::git_probe::execute_shallow_clone_and_probe;

// vox-arch-check: allow git-exec
fn git_cmd() -> tokio::process::Command {
    tokio::process::Command::new("git")
}

#[tokio::test]
async fn test_execute_shallow_clone_rejects_invalid_scheme() {
    let outcome = execute_shallow_clone_and_probe("ftp://bad-url", "git", &["status"], 5).await;
    assert!(outcome.is_err());
    let err_msg = outcome.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid git repository URL scheme"));
}

#[tokio::test]
async fn test_execute_shallow_clone_and_probe_offline_success() {
    let source_dir = tempfile::Builder::new()
        .prefix("vox-git-test-source-")
        .tempdir()
        .expect("create temp dir");

    // Initialize local git repo
    let init = git_cmd()
        .arg("init")
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git init");
    assert!(init.status.success());

    let _ = git_cmd()
        .args(["config", "user.email", "tester@example.com"])
        .current_dir(source_dir.path())
        .output()
        .await;
    let _ = git_cmd()
        .args(["config", "user.name", "Tester"])
        .current_dir(source_dir.path())
        .output()
        .await;

    // Create a file and commit it
    let file_path = source_dir.path().join("test.txt");
    tokio::fs::write(&file_path, "hello git probe")
        .await
        .expect("write test file");

    let add = git_cmd()
        .args(["add", "test.txt"])
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git add");
    assert!(add.status.success());

    let commit = git_cmd()
        .args(["commit", "-m", "init test commit"])
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git commit");
    assert!(commit.status.success());

    let rev_parse = git_cmd()
        .args(["rev-parse", "HEAD"])
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git rev-parse");
    assert!(rev_parse.status.success());
    let expected_sha = String::from_utf8_lossy(&rev_parse.stdout)
        .trim()
        .to_string();

    let repo_url = format!("file://{}", source_dir.path().display());
    let outcome = execute_shallow_clone_and_probe(&repo_url, "git", &["status"], 10)
        .await
        .expect("clone and probe execution");

    assert!(outcome.passed);
    assert_eq!(outcome.head_sha, expected_sha);
}

#[tokio::test]
async fn test_execute_shallow_clone_probe_failure() {
    let source_dir = tempfile::Builder::new()
        .prefix("vox-git-test-source-")
        .tempdir()
        .expect("create temp dir");

    let init = git_cmd()
        .arg("init")
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git init");
    assert!(init.status.success());

    let _ = git_cmd()
        .args(["config", "user.email", "tester@example.com"])
        .current_dir(source_dir.path())
        .output()
        .await;
    let _ = git_cmd()
        .args(["config", "user.name", "Tester"])
        .current_dir(source_dir.path())
        .output()
        .await;

    let file_path = source_dir.path().join("test.txt");
    tokio::fs::write(&file_path, "hello git probe failure")
        .await
        .expect("write test file");

    let add = git_cmd()
        .args(["add", "test.txt"])
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git add");
    assert!(add.status.success());

    let commit = git_cmd()
        .args(["commit", "-m", "init test commit"])
        .current_dir(source_dir.path())
        .output()
        .await
        .expect("git commit");
    assert!(commit.status.success());

    let repo_url = format!("file://{}", source_dir.path().display());
    // Run probe that fails (failing subcommand or non-existent command)
    let outcome = execute_shallow_clone_and_probe(
        &repo_url,
        "git",
        &["checkout", "nonexistent-branch-that-fails"],
        10,
    )
    .await
    .expect("clone succeeded, probe execution returns outcome");

    assert!(!outcome.passed);
    assert!(!outcome.head_sha.is_empty());
}
