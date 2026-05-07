import React from "react";

export interface NodeBadgeProps {
  agent_id: string;
  status: string;
}

export function NodeBadge({ agent_id, status }: NodeBadgeProps): React.ReactElement {
  return (
    <div className={["flex", "flex-row", "px-3", "py-2", "bg-zinc-900", "border", "border-white/10", "rounded-xl", "flex items-center", "gap-3"].filter(Boolean).join(" ")}>
      <div className={["border", "border-border", "p-4", "w-2", "h-2", "rounded-full", (status === "active" ? "bg-emerald-400" : "bg-zinc-600")].filter(Boolean).join(" ")} role={region} />
      <div className={["flex", "flex-col", "gap-0"].filter(Boolean).join(" ")}>
        <p className={["text-xs", "font-mono", "text-white/80"].filter(Boolean).join(" ")}>
          {agent_id}
        </p>
        <p className={["text-xs", "text-zinc-500"].filter(Boolean).join(" ")}>
          {status}
        </p>
      </div>
    </div>
  );
}
