# API Coverage — Hugging Face Hub dataset repositories (hf-hub 1.0)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> New surface in phase 338: dataset repos `<ns>/vox-mens-eval` and `<ns>/vox-mens-data` (plan 338-09).
> Model-repo calls (create, upload, tag, list refs, pull) are the existing phase 324-336 integration and
> are reused unchanged for the optional stale-card re-sync. The `vox mens serve` chat endpoint used by the
> plan 338-08 parity check and the MCP tool registry used by plan 338-05 are first-party, in-repo
> surfaces, not external APIs.

| capability | decision | reason |
|---|---|---|
| create_dataset_repo | INTEGRATE | |
| set_visibility_at_create | INTEGRATE | |
| upload_folder_commit | INTEGRATE | |
| create_version_tag | INTEGRATE | |
| list_refs_tags | INTEGRATE | |
| dataset_card_readme_metadata | INTEGRATE | |
| token_auth_via_clavis | INTEGRATE | |
| download_dataset_files | OPT-OUT | not needed yet: evals read local pinned files and receipts cite the revision; fetch-by-revision for third-party reproduction is a follow-up |
| change_visibility_after_create | OPT-OUT | explicitly out of scope: visibility is a one-way decision made once at the D-25 human checkpoint |
| delete_repo_or_files | OPT-OUT | not needed: destructive and outside every 338 requirement |
| gated_access_requests | OPT-OUT | not needed yet: private visibility already keeps held-out items out of public training data |
| dataset_viewer_parquet_config | OPT-OUT | not needed: consumers read the jsonl files directly |
| validate_yaml_card_metadata | OPT-OUT | not needed: the card uses only standard license and tags fields |
| discussions_and_pull_requests | OPT-OUT | not needed: repos are written only by the publish command |
