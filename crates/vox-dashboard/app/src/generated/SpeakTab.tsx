import React from "react";

import { ComposerPanel } from "./ComposerPanel";

export function SpeakTab(): React.ReactElement {
  return (
<column className={"flex-1 overflow-hidden bg-zinc-950"}
>
  <row className={"h-12 border-b border-zinc-800 px-6 items-center justify-between shrink-0"}
>
  <column className={"gap-0"}
>
  <text className={"text-sm font-black tracking-tighter text-white"}
>
  {"LOQUELA"}
</text>
  <text className={"text-xs text-zinc-500 tracking-widest"}
>
  {"VOICE INTERFACE"}
</text>
</column>
  <row className={"gap-2 items-center"}
>
  <panel className={"w-2 h-2 rounded-full bg-zinc-600"}
>
  
</panel>
  <text className={"text-xs text-zinc-500"}
>
  {"NO ACTIVE SESSION"}
</text>
</row>
</row>
  <panel className={"flex-1 overflow-y-auto py-4"}
>
  <column className={"items-center justify-center h-full gap-4 text-zinc-500"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"START A CONVERSATION"}
</text>
  <text className={"text-xs"}
>
  {"Messages will appear here once a session is active."}
</text>
</column>
</panel>
  <ComposerPanel  />
</column>
  );
}
