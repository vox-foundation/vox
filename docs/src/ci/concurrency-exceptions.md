---
title: "Workflow concurrency exceptions"
description: "Registered exceptions for workflows that intentionally omit cancel-in-progress: true (cancelling mid-run would be wrong)."
category: "CI & Quality"
training_eligible: true

schema_type: "TechArticle"
---

# Workflow concurrency exceptions

`vox ci workflow-concurrency-guard` requires every workflow triggered by `push`
or `pull_request` to declare a top-level `concurrency:` mapping containing
`cancel-in-progress: true`, so a superseded run dies at the source instead of
burning GitHub-hosted runner minutes on a commit nobody will merge. Omitting
`concurrency:` entirely, using a bare group string, or declaring a group
without `cancel-in-progress: true` all count as violations — a non-cancelling
group serializes runs but never reclaims the minutes.

A workflow belongs on the list below when cancelling it mid-run would destroy
something a later run cannot recreate. In practice that is the publish lanes:
a tag-push release build produces artifacts *for that tag*, and a newer tag's
run is not a newer attempt at the same work, so "the latest run wins" — the
assumption `cancel-in-progress` encodes — is simply false there.

- `release-binaries.yml` — tag-push only; a release build must never be cancelled by a later tag.
- `release-gui.yml` — tag-push only; same as above.
- `release-installers.yml` — tag-push only; same as above.
- `scorecard.yml` — pushes to `main` only; a supply-chain scorecard run should complete, and `main` pushes are not the churn the rule targets.
