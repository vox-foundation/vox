import React from "react";

export function MeshLegend(): React.ReactElement {
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
