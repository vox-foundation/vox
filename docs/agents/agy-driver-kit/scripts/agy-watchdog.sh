#!/bin/zsh
# Emits one line when the running agy has made no model call for >12 min (stuck on a tool), with its longest-lived descendant.
warned=""
for i in {1..40}; do pgrep -f "agy -p /drive-task (.planning|docs/superpowers)" >/dev/null && break; sleep 30; done
while pgrep -f "agy -p /drive-task (.planning|docs/superpowers)" >/dev/null; do
  a0=$(pgrep -f "agy -p /drive-task (.planning|docs/superpowers)" | head -1)
  L=$(lsof -p $a0 2>/dev/null | grep -o '/[^ ]*antigravity-cli/log/cli-[^ ]*\.log' | head -1)
  [[ -z $L ]] && L=$(ls -t ~/.gemini/antigravity-cli/log/cli-*.log | head -1)
  last=$(grep streamGenerateContent "$L" | tail -1 | awk '{print $2}' | cut -d. -f1)
  if [[ -n $last ]]; then
    idle=$(( $(date +%s) - $(date -j -f "%H:%M:%S" "$last" +%s) ))
    if (( idle > 720 )) && [[ $warned != $last ]]; then
      warned=$last
      a=$(pgrep -f "agy -p /drive-task (.planning|docs/superpowers)" | head -1)
      kids=$(pgrep -P $(pgrep -P $a | tr '\n' ',' | sed 's/,$//') 2>/dev/null | xargs -I{} ps -o etime=,command= -p {} 2>/dev/null | cut -c1-140 | tr '\n' ';')
      echo "STALL: agy idle ${idle}s since last model call $last; children: $kids"
    fi
  fi
  sleep 60
done
echo "agy run ended"
