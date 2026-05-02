import React from "react";

export interface NodeBadgeProps {
  agent_id: string;
  status: string;
}

export function NodeBadge({ agent_id, status }: NodeBadgeProps): React.ReactElement {
  return (
<row className={"px-3 py-2 bg-zinc-900 border border-white/10 rounded-xl gap-3 items-center"}
>
  <panel className={((status === "active") ? (() => { "w-2 h-2 rounded-full bg-emerald-400";
 })() : (() => { "w-2 h-2 rounded-full bg-zinc-600";
 })())}
>
  
</panel>
  <column className={"gap-0"}
>
  <text className={"text-xs font-mono text-white/80"}
>
  {(() => {
    agent_id;
  })()}
</text>
  <text className={"text-xs text-zinc-500"}
>
  {(() => {
    status;
  })()}
</text>
</column>
</row>
  );
}
