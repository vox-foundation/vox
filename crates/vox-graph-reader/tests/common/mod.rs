//! Temp git repo fixture for history tests.
#![allow(dead_code)]
use std::path::Path;
use std::process::Command;

pub struct Repo {
    pub dir: tempfile::TempDir,
}

impl Repo {
    pub fn new() -> Self {
        let r = Repo {
            dir: tempfile::tempdir().unwrap(),
        };
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["config", "user.email", "t@example.com"]);
        r.git(&["config", "user.name", "Tester"]);
        r.git(&["config", "commit.gpgsign", "false"]);
        r.git(&["config", "core.hooksPath", "/dev/null"]);
        r
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) -> String {
        let o = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout).unwrap()
    }

    pub fn write(&self, rel: &str, content: &str) {
        let p = self.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    /// Stage everything (including deletions) and commit; returns the new HEAD sha.
    pub fn commit(&self, msg: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--allow-empty", "-m", msg]);
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }
}
