// Generated from crates/vox-dashboard/app/src/tabs/network.vox
// DO NOT EDIT — edit the .vox source and re-run `vox build`.
import React from "react";

type MeshViewState = "Overview" | "NodeSelected";

function NodeBadge({ agent_id, status }: { agent_id: string; status: string }): React.ReactElement {
  return (
<row className={"px-3 py-2 bg-zinc-900 border border-white/10 rounded-xl gap-3 items-center"}
>
  <panel className={(status === "active" ? "w-2 h-2 rounded-full bg-emerald-400" : "w-2 h-2 rounded-full bg-zinc-600")}
>
</panel>
  <column className={"gap-0"}
>
  <text className={"text-xs font-mono text-white/80"}
>
  {agent_id}
</text>
  <text className={"text-xs text-zinc-500"}
>
  {status}
</text>
</column>
</row>
  );
}

function MeshLegend(): React.ReactElement {
  return (
<column className={"absolute bottom-4 left-4 gap-2 bg-zinc-900/90 backdrop-blur border border-white/10 rounded-2xl p-4"}
>
  <text className={"text-xs font-bold text-zinc-400 uppercase tracking-widest mb-1"}
>
  {"LEGEND"}
</text>
  <row className={"gap-2 items-center"}
>
  <panel className={"w-3 h-0.5 bg-emerald-400 rounded"}
>
</panel>
  <text className={"text-xs text-zinc-400"}
>
  {"Active channel"}
</text>
</row>
  <row className={"gap-2 items-center"}
>
  <panel className={"w-3 h-0.5 bg-zinc-600 rounded"}
>
</panel>
  <text className={"text-xs text-zinc-400"}
>
  {"Idle channel"}
</text>
</row>
  <row className={"gap-2 items-center"}
>
  <panel className={"w-2 h-2 rounded-full bg-rose-500"}
>
</panel>
  <text className={"text-xs text-zinc-400"}
>
  {"Error node"}
</text>
</row>
</column>
  );
}

export function NetworkTab(): React.ReactElement {
  return (
<column className={"flex-1 overflow-hidden bg-zinc-950"}
>
  <row className={"h-12 border-b border-zinc-800 px-6 items-center justify-between shrink-0"}
>
  <column className={"gap-0"}
>
  <text className={"text-sm font-black tracking-tighter text-white"}
>
  {"NETWORK"}
</text>
  <text className={"text-xs text-zinc-500 tracking-widest"}
>
  {"AGENT MESH TOPOLOGY"}
</text>
</column>
  <row className={"gap-3 items-center"}
>
  <text className={"text-xs text-zinc-500"}
>
  {"0 nodes · 0 edges"}
</text>
  <button className={"px-3 py-1.5 rounded-lg bg-white/5 border border-white/10 text-xs text-zinc-400"}
>
  {"REFRESH"}
</button>
</row>
</row>
  <panel className={"flex-1 relative overflow-hidden"}
>
  <column className={"flex-1 items-center justify-center gap-4 text-zinc-500"}
>
  <panel className={"w-16 h-16 rounded-2xl border border-white/10 bg-zinc-900 items-center justify-center"}
>
  <text className={"text-2xl"}
>
  {"⬡"}
</text>
</panel>
  <column className={"items-center gap-1"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"NO MESH DATA"}
</text>
  <text className={"text-xs"}
>
  {"Agent graph renders here via @island NetworkGraph (Phase 2)."}
</text>
</column>
</column>
  <MeshLegend  />
</panel>
</column>
  );
}
