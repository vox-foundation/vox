use super::*;

#[test]
fn discovers_vox_and_tagged_markdown_runbooks_extracts_params_and_blocks_escaping_symlinks() {
    let outer = tempfile::tempdir().expect("outer tempdir");
    let outside_file = outer.path().join("outside_secret.vox");
    fs::write(&outside_file, "// @title Escaped\nfn main() {}\n").unwrap();
    let ws = outer.path().join("repo");
    for sub in ["scripts/ops", ".vox/runbooks/nested", "docs/src/how-to"] {
        fs::create_dir_all(ws.join(sub)).unwrap();
    }
    fs::write(
        ws.join("scripts/deploy.vox"),
        "// @title Deploy Service\n// @desc Ship staging\n// @param env=staging | staging,prod | Target environment\n// Deploy {{env}} with {{tag}} and {{env}}\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        ws.join("scripts/ops/smoke.vox"),
        "// @title Nested Smoke\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        ws.join(".vox/runbooks/check.md"),
        "---\ntitle: \"Health Check\"\ndescription: \"Verify cluster node\"\nparams:\n  - name: target\n    default: localhost\n    description: Hostname\n---\n\n# Health Check\n\n```sh\ncurl -fsSL http://{{target}}:8080/healthz\n```\n",
    )
    .unwrap();
    fs::write(
        ws.join(".vox/runbooks/nested/empty_prose.md"),
        "# Notes Only\n\nNo executable shell fence here.\n",
    )
    .unwrap();
    fs::write(
        ws.join("docs/src/how-to/tagged.md"),
        "---\ntitle: \"Rotate Keys\"\ndescription: \"@runbook Rotate signing keys\"\ntags: [security, runbook]\n---\n\n```vox\nvox secrets doctor --profile {{profile}}\n```\n",
    )
    .unwrap();
    fs::write(
        ws.join("docs/src/how-to/untagged.md"),
        "---\ntitle: \"Ordinary Guide\"\n---\n\n```sh\necho hi\n```\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside_file, ws.join("scripts/escaped.vox")).unwrap();

    let found = scan_workspace_runbooks(&ws);
    assert_eq!(found.len(), 4, "unexpected runbooks: {found:?}");
    assert!(!found.iter().any(|r| {
        r.path.contains("untagged") || r.path.contains("escaped") || r.path.contains("empty_prose")
    }));
    let by_path = |p: &str| found.iter().find(|r| r.path == p).unwrap();

    let deploy = by_path("scripts/deploy.vox");
    assert_eq!(deploy.kind, RunbookKind::Vox);
    assert_eq!(deploy.description, "Ship staging");
    assert_eq!(
        deploy.command_template,
        "vox run scripts/deploy.vox -- --env {{env}} --tag {{tag}}"
    );
    assert_eq!(
        by_path("scripts/ops/smoke.vox").command_template,
        "vox run scripts/ops/smoke.vox"
    );
    let names: Vec<_> = deploy.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["env", "tag"]);
    assert_eq!(deploy.params[0].default_value.as_deref(), Some("staging"));
    assert_eq!(deploy.params[0].options, vec!["staging", "prod"]);

    let check = by_path(".vox/runbooks/check.md");
    assert_eq!(check.kind, RunbookKind::Markdown);
    assert_eq!(check.title, "Health Check");
    assert_eq!(check.params[0].default_value.as_deref(), Some("localhost"));
    assert!(check.command_template.contains("{{target}}"));

    assert_eq!(
        by_path("docs/src/how-to/tagged.md").params[0].name,
        "profile"
    );
}
