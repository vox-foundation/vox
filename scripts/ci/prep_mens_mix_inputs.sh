#!/usr/bin/env bash
# Create minimal JSONL inputs required by mens/config/mix.yaml for strict CI mixing.
# *.jsonl is gitignored; this keeps `vox mens corpus mix` honest without checked-in corpora.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DATA="${REPO_ROOT}/mens/data"

mkdir -p "${DATA}"

one_pair='{"prompt":"CI mix smoke: hello","response":"ok","rating":4}'
tool_trace='{"task_prompt":"CI tool supervision","tool_name":"noop","arguments_json":"{}","result_json":"{\"ok\":true}","success":true}'

printf '%s\n' "${one_pair}" > "${DATA}/train_full_backup.jsonl"
printf '%s\n' "${tool_trace}" > "${DATA}/tool_traces.example.jsonl"
printf '%s\n' "${one_pair}" > "${DATA}/synthetic.jsonl"
printf '%s\n' "${one_pair}" > "${DATA}/golden_pairs.jsonl"
printf '%s\n' "${one_pair}" > "${DATA}/synthetic_search.jsonl"
printf '%s\n' "${one_pair}" > "${DATA}/golden_validated.jsonl"

echo "prep_mens_mix_inputs: wrote minimal lines under ${DATA}"
