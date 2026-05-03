import React, { useState } from "react";

import { PipelineView } from "./PipelineView";
import { WorkflowScrubber } from "./WorkflowScrubber";

export function ForgeTab(): React.ReactElement {
  const [active_panel, set_active_panel] = useState("pipeline");
  return (
<div className={["flex", "flex-col", "flex-1", "overflow-hidden"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-row", "h-10", "border-b-true", "border-zinc-800", "px-4", "flex items-center", "shrink-0", "gap-2"].filter(Boolean).join(" ")}
>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_active_panel("pipeline");
}}
>
  {"PIPELINE"}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "bg-primary", "text-primary-foreground", "hover:bg-primary/90", "h-10", "px-4", "py-2", "tab-btn"].filter(Boolean).join(" ")} onClick={() => {
    set_active_panel("scrubber");
}}
>
  {"SCRUBBER"}
</button>
</div>
  <div className={["bg-background", "rounded-lg", "border", "border-border", "p-4", "flex-1", "overflow-hidden"].filter(Boolean).join(" ")}
>
  {(active_panel === "pipeline" ? <PipelineView  /> : <WorkflowScrubber  />)}
</div>
</div>
  );
}
