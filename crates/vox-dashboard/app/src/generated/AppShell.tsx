import React, { useState } from "react";

export function AppShell(): React.ReactElement {
  const [tab, set_tab] = useState("speak");
  return (
<column className={"min-h-screen bg-zinc-950 text-zinc-100 font-mono"}
>
  <row className={"h-10 border-b border-zinc-800 px-4 items-center justify-between shrink-0"}
>
  <text className={"text-xs tracking-widest text-zinc-500"}
>
  {"VOX ORCHESTRATOR"}
</text>
  <row className={"gap-1"}
>
  <button className={"tab-btn"} onClick={() => {
    set_tab("speak");
}}
>
  {"LOQUELA"}
</button>
  <button className={"tab-btn"} onClick={() => {
    set_tab("command");
}}
>
  {"IMPERIUM"}
</button>
  <button className={"tab-btn"} onClick={() => {
    set_tab("network");
}}
>
  {"RETE"}
</button>
  <button className={"tab-btn"} onClick={() => {
    set_tab("forge");
}}
>
  {"FABRICA"}
</button>
</row>
</row>
  <panel className={"flex-1 overflow-hidden"}
>
  {(tab === "speak" ? <SpeakTab  /> : (tab === "command" ? <CommandTab  /> : (tab === "network" ? <NetworkTab  /> : <ForgeTab  />)))}
</panel>
</column>
  );
}
