# Harness Eval History (auto-generated)

`vox harness publish` writes each run to `runs/<run_id>.jsonl` in this directory, and
`vox harness history`/`report` and the Vox Axis GUI's Harness Health surface read every
file here: the per-run files plus the legacy append-only `runs.jsonl` (older runs; read,
never written). It is a git-tracked sync mechanism (see
`docs/superpowers/specs/2026-08-02-chat-harness-continuous-eval-design.md` §9). One file per
run means two results PRs can never conflict. **Never hand-edit these files**, per this
repo's convention for auto-generated files.

The nightly `harness-eval-nightly.yml` publishes through a pull request that lands via the
merge queue (main has no direct pushes), and uploads the files as a workflow artifact first
so a failed PR never loses results.
