use super::*;

fn req(line: &str, cwd: &Path, limit: usize) -> TermCompletionRequest {
    TermCompletionRequest {
        line: line.into(),
        cursor: line.len(),
        cwd: Some(cwd.to_string_lossy().into_owned()),
        shell_mode: None,
        limit: Some(limit),
    }
}

#[test]
fn completes_builtins_path_files_dotfiles_git_worktree_refs_and_cli_specs() {
    let engine = CompletionEngine::new();
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().join("workspace");
    fs::create_dir_all(ws.join("src")).unwrap();
    fs::write(ws.join("Cargo.toml"), "[package]\n").unwrap();
    fs::write(ws.join(".secret_env"), "HIDDEN=1\n").unwrap();

    assert!(
        engine
            .complete(&req("exp", &ws, 20))
            .iter()
            .any(|i| i.label == "export" && i.kind == TermCompletionKind::Builtin)
    );
    assert!(
        engine
            .complete(&req("sh", &ws, 100))
            .iter()
            .any(|i| i.kind == TermCompletionKind::Executable)
    );

    let rel = engine.complete(&req("ls ./", &ws, 20));
    assert!(
        rel.iter()
            .any(|i| i.insert_text == "./src/" && i.kind == TermCompletionKind::Directory)
    );
    assert!(
        rel.iter()
            .any(|i| i.insert_text == "./Cargo.toml" && i.kind == TermCompletionKind::Path)
    );
    assert!(!rel.iter().any(|i| i.label.contains(".secret_env")));
    assert!(
        engine
            .complete(&req("cat .s", &ws, 20))
            .iter()
            .any(|i| i.insert_text == ".secret_env")
    );
    assert!(engine.complete(&req("cat ../../../", &ws, 20)).is_empty());

    let std_repo = dir.path().join("std_repo");
    fs::create_dir_all(std_repo.join(".git/refs/heads")).unwrap();
    fs::write(std_repo.join(".git/refs/heads/develop"), "abcdef\n").unwrap();
    assert!(
        engine
            .complete(&req("git checkout ", &std_repo, 20))
            .iter()
            .any(|i| i.label == "develop" && i.kind == TermCompletionKind::GitRef)
    );

    let common_git = dir.path().join("bare.git");
    let wt_git = common_git.join("worktrees/wt1");
    fs::create_dir_all(common_git.join("refs/heads/feat")).unwrap();
    fs::create_dir_all(common_git.join("refs/tags")).unwrap();
    fs::create_dir_all(&wt_git).unwrap();
    fs::write(common_git.join("refs/heads/main"), "1111111\n").unwrap();
    fs::write(common_git.join("refs/heads/feat/warp"), "2222222\n").unwrap();
    fs::write(common_git.join("refs/tags/v1.2.0"), "3333333\n").unwrap();
    fs::write(
        common_git.join("packed-refs"),
        "# pack-refs\n4444444 refs/heads/release/v75\n5555555 refs/remotes/origin/main\n",
    )
    .unwrap();
    fs::write(wt_git.join("commondir"), "../..\n").unwrap();
    fs::write(ws.join(".git"), format!("gitdir: {}\n", wt_git.display())).unwrap();

    let refs = engine.complete(&req("git switch ", &ws, 50));
    for expected in ["main", "feat/warp", "release/v75", "v1.2.0"] {
        assert!(
            refs.iter()
                .any(|i| i.label == expected && i.kind == TermCompletionKind::GitRef),
            "missing {expected}"
        );
    }

    for (line, expected_label, expected_kind) in [
        ("cargo ch", "check", TermCompletionKind::Subcommand),
        ("cargo test --rel", "--release", TermCompletionKind::Flag),
        ("git che", "checkout", TermCompletionKind::Subcommand),
        ("pnpm --f", "--filter", TermCompletionKind::Flag),
        ("npm out", "outdated", TermCompletionKind::Subcommand),
        ("docker co", "compose", TermCompletionKind::Subcommand),
        ("vox doc", "doctor", TermCompletionKind::Subcommand),
        ("git add Car", "Cargo.toml", TermCompletionKind::Path),
        ("vim sr", "src/", TermCompletionKind::Directory),
    ] {
        let items = engine.complete(&req(line, &ws, 20));
        assert!(
            items
                .iter()
                .any(|i| i.label == expected_label && i.kind == expected_kind),
            "missing {expected_label}"
        );
    }
    if std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .is_some()
    {
        assert!(
            engine
                .complete(&req("cd ~", &ws, 20))
                .iter()
                .any(|i| i.insert_text == "~/" && i.kind == TermCompletionKind::Directory)
        );
    }
}
