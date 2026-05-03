import React, { useState } from "react";

import { CommandTab } from "./CommandTab";
import { ForgeTab } from "./ForgeTab";
import { NetworkTab } from "./NetworkTab";
import { SpeakTab } from "./SpeakTab";

export function AppShell(): React.ReactElement {
  const [tab, set_tab] = useState("speak");
  return (
<div className={["flex", "flex-col", "min-h-screen", "bg-zinc-950", "text-zinc-100", "font-mono"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-row", "h-10", "border-b-true", "border-zinc-800", "px-4", "flex items-center", "flex justify-between", "shrink-0"].filter(Boolean).join(" ")}
>
  <p className={["text-xs", "tracking-widest", "text-zinc-500"].filter(Boolean).join(" ")}
>
  {"VOX ORCHESTRATOR"}
</p>
  <div className={["flex", "flex-row", "gap-1"].filter(Boolean).join(" ")}
>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_tab("speak");
}}
>
  {"LOQUELA"}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_tab("command");
}}
>
  {"IMPERIUM"}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_tab("network");
}}
>
  {"RETE"}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_tab("forge");
}}
>
  {"FABRICA"}
</button>
</div>
</div>
  <div className={["bg-background", "rounded-lg", "border", "border-border", "p-4", "flex-1", "overflow-hidden"].filter(Boolean).join(" ")}
>
  {(tab === "speak" ? <SpeakTab  /> : (tab === "command" ? <CommandTab  /> : (tab === "network" ? <NetworkTab  /> : <ForgeTab  />)))}
</div>
</div>
  );
}
