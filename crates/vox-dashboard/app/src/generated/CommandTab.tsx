// Generated from crates/vox-dashboard/app/src/tabs/command.vox
// DO NOT EDIT — edit the .vox source and re-run `vox build`.
import React, { useState } from "react";

type DiagnosticViewState = "Collapsed" | "Expanded" | "Filtered";

function DiagnosticRow({ severity, message, location }: { severity: string; message: string; location: string }): React.ReactElement {
  return (
<row className={"px-4 py-3 border-b border-white/5 gap-4 items-start hover:bg-white/5"}
>
  <panel className={(severity === "error" ? "w-2 h-2 rounded-full bg-rose-500 mt-1.5 shrink-0" : "w-2 h-2 rounded-full bg-amber-400 mt-1.5 shrink-0")}
>
</panel>
  <column className={"flex-1 gap-1 min-w-0"}
>
  <text className={"text-sm text-white/80 font-mono leading-snug"}
>
  {message}
</text>
  <text className={"text-xs text-zinc-500 font-mono"}
>
  {location}
</text>
</column>
  <text className={(severity === "error" ? "text-xs font-bold text-rose-400 uppercase shrink-0" : "text-xs font-bold text-amber-400 uppercase shrink-0")}
>
  {severity}
</text>
</row>
  );
}

function DiagnosticsPanel(): React.ReactElement {
  return (
<column className={"flex-1 overflow-y-auto"}
>
  <row className={"px-4 py-2 border-b border-white/10 justify-between items-center shrink-0 bg-zinc-900/50"}
>
  <text className={"text-xs font-bold text-zinc-400 uppercase tracking-widest"}
>
  {"DIAGNOSTICS"}
</text>
  <text className={"text-xs text-zinc-600"}
>
  {"0 errors · 0 warnings"}
</text>
</row>
  <column className={"flex-1 items-center justify-center gap-3 text-zinc-500"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"NO DIAGNOSTICS"}
</text>
  <text className={"text-xs"}
>
  {"Run a build to see compiler output here."}
</text>
</column>
</column>
  );
}

function TaskDispatch(): React.ReactElement {
  const [is_running, set_is_running] = useState(false);
  return (
<column className={"border-t border-white/10 p-4 gap-3 shrink-0"}
>
  <row className={"gap-3 items-center"}
>
  <button className={(is_running ? "px-4 py-2 rounded-xl bg-rose-600/80 text-white text-sm font-bold" : "px-4 py-2 rounded-xl bg-blue-600 text-white text-sm font-bold hover:bg-blue-500")} onClick={() => {
    set_is_running(!is_running);
}}
>
  {(is_running ? "STOP" : "RUN BUILD")}
</button>
  <button className={"px-4 py-2 rounded-xl bg-white/5 text-zinc-400 text-sm font-bold border border-white/10"}
>
  {"CLEAR"}
</button>
  <panel className={"flex-1"}
>
</panel>
  <text className={(is_running ? "text-xs text-blue-400 font-bold uppercase" : "text-xs text-zinc-600 uppercase")}
>
  {(is_running ? "BUILDING…" : "IDLE")}
</text>
</row>
  <text className={"text-xs text-zinc-600"}
>
  {"Transport bridge: Phase 2 — output streams via @island"}
</text>
</column>
  );
}

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
