#!/usr/bin/env bash
# Sparse S4 monitor: ~20 min between checkins. Not project automation — ops evidence helper.
set -euo pipefail
R=/Users/brbrainerd/dev/vox-mens-compare
D=$R/mens/runs/qwen3-0.6b
STATUS=$R/.planning/phases/320-train-the-sizes-size-03/evidence/s4-0.6B-monitor.log
INTERVAL=${1:-1200}
MAX=${2:-12}

echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) monitor_start interval=${INTERVAL}s max=${MAX}" >>"$STATUS"
for i in $(seq 1 "$MAX"); do
  sleep "$INTERVAL"
  ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  if ! pgrep -f 'vox-ml-cli mens train.*qwen3-0.6b' >/dev/null; then
    steps=$(rg -c '"event":"step"' "$D/telemetry.jsonl" 2>/dev/null || echo 0)
    status=$(jq -r .status "$D/train-record.json" 2>/dev/null || echo none)
    echo "$ts DONE steps=$steps record=$status" >>"$STATUS"
    echo "TRAIN_DONE steps=$steps status=$status"
    exit 0
  fi
  steps=$(rg -c '"event":"step"' "$D/telemetry.jsonl" 2>/dev/null || echo 0)
  last=$(tail -1 "$D/train.log" | sed 's/\x1b\[[0-9;]*m//g' | tr -s ' ')
  div=ok
  if rg -q '"loss":null' "$D/telemetry.jsonl" 2>/dev/null; then
    div=NULL_LOSS
  fi
  echo "$ts alive steps=$steps div=$div | $last" >>"$STATUS"
  echo "CHECKIN_$i steps=$steps div=$div"
done
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) monitor_timeout_${MAX}_intervals" >>"$STATUS"
echo "MONITOR_TIMEOUT"
