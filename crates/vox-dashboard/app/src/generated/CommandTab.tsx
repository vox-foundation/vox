import React, { useState } from "react";

import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { TaskDispatch } from "./TaskDispatch";

export function CommandTab(): React.ReactElement {
  const [active_panel, set_active_panel] = useState("diagnostics");
  return (
<div className={["flex", "flex-col", "flex-1", "overflow-hidden", "bg-zinc-950"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-row", "h-12", "border-b-true", "border-zinc-800", "px-6", "flex items-center", "flex justify-between", "shrink-0"].filter(Boolean).join(" ")}
>
  <p className={["text-sm", "text-white", "tracking-tighter"].filter(Boolean).join(" ")}
>
  {"COMMAND"}
</p>
  <div className={["flex", "flex-row", "gap-2"].filter(Boolean).join(" ")}
>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_active_panel("diagnostics");
}}
>
  {"DIAGNOSTICS"}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_active_panel("context");
}}
>
  {"CONTEXT"}
</button>
</div>
</div>
  <div className={["bg-background", "rounded-lg", "border", "border-border", "p-4", "flex-1", "overflow-hidden"].filter(Boolean).join(" ")}
>
  {(active_panel === "diagnostics" ? <DiagnosticsPanel  /> : <div className={["flex", "flex-col", "flex-1", "flex items-center", "flex justify-center", "text-zinc-500", "gap-3"].filter(Boolean).join(" ")}
>
  <p className={["text-sm", "font-bold", "uppercase", "tracking-widest"].filter(Boolean).join(" ")}
>
  {"CONTEXT EXPLORER"}
</p>
  <p className={"text-xs"}
>
  {"Context window analysis — Phase 2."}
</p>
</div>)}
</div>
  <TaskDispatch  />
</div>
  );
}
