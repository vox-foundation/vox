import React from "react";

import { MeshLegend } from "./MeshLegend";

export function NetworkTab(): React.ReactElement {
  return (
<div className={["flex", "flex-col", "flex-1", "overflow-hidden", "bg-zinc-950"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-row", "h-12", "border-b-true", "border-zinc-800", "px-6", "flex items-center", "flex justify-between", "shrink-0"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-col", "gap-0"].filter(Boolean).join(" ")}
>
  <p className={["text-sm", "text-white", "tracking-tighter"].filter(Boolean).join(" ")}
>
  {"NETWORK"}
</p>
  <p className={["text-xs", "text-zinc-500", "tracking-widest"].filter(Boolean).join(" ")}
>
  {"AGENT MESH TOPOLOGY"}
</p>
</div>
  <div className={["flex", "flex-row", "flex items-center", "gap-3"].filter(Boolean).join(" ")}
>
  <p className={["text-xs", "text-zinc-500"].filter(Boolean).join(" ")}
>
  {"0 nodes · 0 edges"}
</p>
  <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "h-10", "px-3", "rounded-lg", "bg-white/5", "border", "border-white/10", "text-zinc-400", "py-1.5"].filter(Boolean).join(" ")}
>
  {"REFRESH"}
</button>
</div>
</div>
  <div className={["bg-background", "rounded-lg", "border", "border-border", "p-4", "flex-1", "relative", "overflow-hidden"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-col", "flex-1", "flex items-center", "flex justify-center", "text-zinc-500", "gap-4"].filter(Boolean).join(" ")}
>
  <div className={["p-4", "w-16", "h-16", "rounded-2xl", "border", "border-white/10", "bg-zinc-900", "flex items-center", "flex justify-center"].filter(Boolean).join(" ")}
>
  <p className={"text-2xl"}
>
  {"⬡"}
</p>
</div>
  <div className={["flex", "flex-col", "flex items-center", "gap-1"].filter(Boolean).join(" ")}
>
  <p className={["text-sm", "font-bold", "uppercase", "tracking-widest"].filter(Boolean).join(" ")}
>
  {"NO MESH DATA"}
</p>
  <p className={"text-xs"}
>
  {"Agent graph renders here via React interop NetworkGraph (Phase 2)."}
</p>
</div>
</div>
  <MeshLegend  />
</div>
</div>
  );
}
