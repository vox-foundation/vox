import React, { useState } from "react";

export function TaskDispatch(): React.ReactElement {
  const [is_running, set_is_running] = useState(false);
  return (
<column className={"border-t border-white/10 p-4 gap-3 shrink-0"}
>
  <row className={"gap-3 items-center"}
>
  <button className={((is_running) ? (() => { "px-4 py-2 rounded-xl bg-rose-600/80 text-white text-sm font-bold";
 })() : (() => { "px-4 py-2 rounded-xl bg-blue-600 text-white text-sm font-bold hover:bg-blue-500";
 })())} onClick={() => {
    set_is_running(!is_running);
}}
>
  {((is_running) ? (() => { "STOP";
 })() : (() => { "RUN BUILD";
 })())}
</button>
  <button className={"px-4 py-2 rounded-xl bg-white/5 text-zinc-400 text-sm font-bold border border-white/10"}
>
  {"CLEAR"}
</button>
  <panel className={"flex-1"}
>
  
</panel>
  <text className={((is_running) ? (() => { "text-xs text-blue-400 font-bold uppercase";
 })() : (() => { "text-xs text-zinc-600 uppercase";
 })())}
>
  {((is_running) ? (() => { "BUILDING…";
 })() : (() => { "IDLE";
 })())}
</text>
</row>
  <text className={"text-xs text-zinc-600"}
>
  {"Transport bridge: Phase 2 — output streams via @island"}
</text>
</column>
  );
}
