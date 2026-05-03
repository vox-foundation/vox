import React from "react";

import { MeshLegend } from "./MeshLegend";

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
  {"Agent graph renders here via NetworkGraph React component (Phase 2)."}
</text>
</column>
</column>
  <MeshLegend  />
</panel>
</column>
  );
}
