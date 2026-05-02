import React, { useState } from "react";

export function WorkflowScrubber(): React.ReactElement {
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
  {((is_playing) ? (() => { "PAUSE";
 })() : (() => { "PLAY";
 })())}
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
