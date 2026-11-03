---
title: Land local main through a draft PR to unblock nightly
date: 2026-10-03
priority: urgent
phase: "06.1"
---

Every scheduled nightly since at least 2026-09-28 fails in its `setup` job (and `full`) because `Cargo.lock` on
`origin/main` is out of sync with the manifests (`--locked` refuses to update it), which skips ten downstream jobs.
Local `main` already carries the one-line lock fix and passes `cargo metadata --locked --offline`, but it was 57
commits ahead of `origin/main` on 2026-10-03.

Action (Task 0.1 of `docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md`): with the owner's approval to
push, `git push -u origin main:land/2026-10-03-main`, `gh pr create --draft`, read CI, merge through the queue. Done
when the next scheduled nightly's `setup` job passes.
