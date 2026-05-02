import React, { useState } from "react";

import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { TaskDispatch } from "./TaskDispatch";

export function CommandTab(): React.ReactElement {
  const [active_panel, set_active_panel] = useState("diagnostics");
  return (
<column className={"flex-1 overflow-hidden bg-zinc-950"}
>
  <row className={"h-12 border-b border-zinc-800 px-6 items-center justify-between shrink-0"}
>
  <text className={"text-sm font-black tracking-tighter text-white"}
>
  {"COMMAND"}
</text>
  <row className={"gap-2"}
>
  <button className={"tab-btn"} onClick={() => {
    set_active_panel("diagnostics");
}}
>
  {"DIAGNOSTICS"}
</button>
  <button className={"tab-btn"} onClick={() => {
    set_active_panel("context");
}}
>
  {"CONTEXT"}
</button>
</row>
</row>
  <panel className={"flex-1 overflow-hidden"}
>
  {(active_panel === "diagnostics" ? <DiagnosticsPanel  /> : <column className={"flex-1 items-center justify-center gap-3 text-zinc-500"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"CONTEXT EXPLORER"}
</text>
  <text className={"text-xs"}
>
  {"Context window analysis — Phase 2."}
</text>
</column>)}
</panel>
  <TaskDispatch  />
</column>
  );
}
