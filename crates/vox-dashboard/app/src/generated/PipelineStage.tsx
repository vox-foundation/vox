import React from "react";

export interface PipelineStageProps {
  name: string;
  desc: string;
}

export function PipelineStage({ name, desc }: PipelineStageProps): React.ReactElement {
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
