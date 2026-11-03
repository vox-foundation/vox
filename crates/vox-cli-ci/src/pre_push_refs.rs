//! Git pre-push stdin: `<local ref> <local sha> <remote ref> <remote sha>` per line.
//! Work reaches `main` through a PR (AGENTS.md, PR & Review Discipline), so a push that
//! updates remote `main` is refused unless `VOX_ALLOW_MAIN_PUSH=1`.

pub fn refused_main_push(stdin: &str, allow: bool) -> Option<String> {
    if allow {
        return None;
    }
    stdin
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .find(|remote_ref| *remote_ref == "refs/heads/main")
        .map(|_| {
            "refusing to push to main: open a draft PR (git push -u origin HEAD:<branch> && gh pr create --draft), or set VOX_ALLOW_MAIN_PUSH=1".to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    const Z: &str = "0000000000000000000000000000000000000000";

    #[test]
    fn refuses_a_push_that_updates_remote_main() {
        let s = format!("refs/heads/main abc refs/heads/main {Z}\n");
        assert!(refused_main_push(&s, false).is_some());
    }

    #[test]
    fn allows_a_branch_push_and_the_explicit_override() {
        let s = format!("refs/heads/main abc refs/heads/land/x {Z}\n");
        assert!(refused_main_push(&s, false).is_none());
        let m = format!("refs/heads/main abc refs/heads/main {Z}\n");
        assert!(refused_main_push(&m, true).is_none());
    }

    #[test]
    fn ignores_empty_stdin() {
        assert!(refused_main_push("", false).is_none());
    }
}
