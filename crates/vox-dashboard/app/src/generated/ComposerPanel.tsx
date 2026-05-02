import React, { useState } from "react";

export function ComposerPanel(): React.ReactElement {
  const [is_submitting, set_is_submitting] = useState(false);
  return (
<column className={"border-t border-white/10 p-4 gap-3 bg-zinc-950/80"}
>
  <row className={"gap-3 items-end"}
>
  <panel className={"flex-1 bg-zinc-900 border border-white/10 rounded-2xl px-4 py-3 min-h-16"}
>
  <text className={"text-sm text-white/50 italic"}
>
  {"Type a message…"}
</text>
</panel>
  <button className={((is_submitting) ? (() => { "w-12 h-12 rounded-xl bg-blue-600/50 text-white/50 flex items-center justify-center";
 })() : (() => { "w-12 h-12 rounded-xl bg-blue-600 text-white flex items-center justify-center";
 })())} onClick={() => {
    set_is_submitting(!is_submitting);
}}
>
  {((is_submitting) ? (() => { "…";
 })() : (() => { "→";
 })())}
</button>
</row>
  <row className={"justify-between items-center px-1"}
>
  <text className={"text-xs text-zinc-500"}
>
  {"Shift+Enter for new line"}
</text>
  <text className={"text-xs text-zinc-600"}
>
  {"Transport bridge: Phase 2"}
</text>
</row>
</column>
  );
}
