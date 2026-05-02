import React, { useState } from "react";

import { PipelineView } from "./PipelineView";
import { WorkflowScrubber } from "./WorkflowScrubber";

export function ForgeTab(): React.ReactElement {
  const [active_panel, set_active_panel] = useState("pipeline");
  return (
<column className={"flex-1 overflow-hidden"}
>
  <row className={"h-10 border-b border-zinc-800 px-4 items-center gap-2 shrink-0"}
>
  <button className={"tab-btn"} onClick={() => {
    set_active_panel("pipeline");
}}
>
  {"PIPELINE"}
</button>
  <button className={"tab-btn"} onClick={() => {
    set_active_panel("scrubber");
}}
>
  {"SCRUBBER"}
</button>
</row>
  <panel className={"flex-1 overflow-hidden"}
>
  {(active_panel === "pipeline" ? <PipelineView  /> : <WorkflowScrubber  />)}
</panel>
</column>
  );
}
