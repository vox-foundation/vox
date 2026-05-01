// Generated from crates/vox-dashboard/app/src/tabs/forge.vox
// DO NOT EDIT — edit the .vox source and re-run `vox build`.
import React, { useState } from "react";

type ScrubberState = "Paused" | "Playing" | "Scrubbing";
type PipelinePhase = "Idle" | "Running" | "Completed" | "Failed";

function PipelineStage({ name, desc }: { name: string; desc: string }): React.ReactElement {
  return (
<column className={"p-8 flex-1 border-r border-white/5 gap-4"}
>
  <row className={"justify-between mb-6"}
>
  <panel className={"w-10 h-10 rounded-xl bg-zinc-900 border border-white/5 items-center justify-center"}
>
  <text className={"text-zinc-500 text-xs font-mono"}
>
  {name}
</text>
</panel>
  <text className={"text-xs font-bold text-rose-500 bg-rose-500/10 px-2 py-1 rounded border border-rose-500/20"}
>
  {"IDLE"}
</text>
</row>
  <text className={"text-2xl font-bold text-white/90"}
>
  {name}
</text>
  <text className={"text-zinc-500 text-sm leading-relaxed"}
>
  {desc}
</text>
  <panel className={"flex-1 rounded-2xl border border-white/5 p-5 font-mono"}
>
  <text className={"text-xs text-zinc-500 italic"}
>
  {"No output yet."}
</text>
</panel>
</column>
  );
}

function PipelineView(): React.ReactElement {
  return (
<row className={"h-full bg-zinc-950"}
>
  <PipelineStage name={"Lexer"} desc={"Logos-based tokenization"} />
  <PipelineStage name={"Parser"} desc={"Rowan GreenTree CST generation"} />
  <PipelineStage name={"HIR"} desc={"High-level IR with name resolution"} />
  <PipelineStage name={"TypeCheck"} desc={"Bidirectional unification logic"} />
  <PipelineStage name={"CodeGen"} desc={"Rust and TypeScript emission"} />
</row>
  );
}

function WorkflowScrubber(): React.ReactElement {
  const [is_playing, set_is_playing] = useState(false);
  return (
<column className={"p-10 bg-zinc-950 h-full gap-8 text-white"}
>
  <row className={"justify-between items-center"}
>
  <column className={"gap-2"}
>
  <text className={"text-3xl font-black tracking-tighter text-white"}
>
  {"TIME TRAVEL"}
</text>
  <text className={"text-xs text-zinc-400 font-bold tracking-widest"}
>
  {"DURABLE WORKFLOW STATE INSPECTOR"}
</text>
</column>
  <row className={"gap-4 bg-white/5 p-2 rounded-2xl border border-white/5 items-center"}
>
  <button className={"w-10 h-10 rounded-xl bg-white/5 text-zinc-400 flex items-center justify-center"}
>
  {"<<"}
</button>
  <button className={"w-12 h-12 rounded-xl bg-blue-600 text-white flex items-center justify-center"} onClick={() => {
    set_is_playing(!is_playing);
}}
>
  {(is_playing ? "PAUSE" : "PLAY")}
</button>
  <button className={"w-10 h-10 rounded-xl bg-white/5 text-zinc-400 flex items-center justify-center"}
>
  {">>"}
</button>
</row>
</row>
  <panel className={"flex-1 flex items-center justify-center"}
>
  <column className={"items-center gap-4 text-zinc-500"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"NO ACTIVE WORKFLOW"}
</text>
  <text className={"text-xs"}
>
  {"Durable state execution will appear here when orchestrated."}
</text>
</column>
</panel>
</column>
  );
}

export function ForgeTab(): React.ReactElement {
  const [active_panel, set_active_panel] = useState("pipeline");
  return (
<column className={"flex-1 overflow-hidden"}
>
  <row className={"h-10 border-b border-zinc-800 px-4 items-center gap-2 shrink-0"}
>
  <button className={"tab-btn"} onClick={() => {
    set_active_panel("pipeline");
}}
>
  {"PIPELINE"}
</button>
  <button className={"tab-btn"} onClick={() => {
    set_active_panel("scrubber");
}}
>
  {"SCRUBBER"}
</button>
</row>
  <panel className={"flex-1 overflow-hidden"}
>
  {(active_panel === "pipeline" ? <PipelineView  /> : <WorkflowScrubber  />)}
</panel>
</column>
  );
}
