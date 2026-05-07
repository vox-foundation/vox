import React from "react";

export function MeshLegend(): React.ReactElement {
  return (
    <div className={["flex", "flex-col", "absolute", "bottom-4", "left-4", "bg-zinc-900/90", "border", "border-white/10", "rounded-2xl", "p-4", "gap-2", "backdrop-blur"].filter(Boolean).join(" ")}>
      <p className={["text-xs", "font-bold", "text-zinc-400", "uppercase", "tracking-widest", "mb-1"].filter(Boolean).join(" ")}>
        {"LEGEND"}
      </p>
      <div className={["flex", "flex-row", "flex items-center", "gap-2"].filter(Boolean).join(" ")}>
        <div className={["border", "border-border", "p-4", "w-3", "bg-emerald-400", "rounded-DEFAULT", "h-0.5"].filter(Boolean).join(" ")} role={region} />
        <p className={["text-xs", "text-zinc-400"].filter(Boolean).join(" ")}>
          {"Active channel"}
        </p>
      </div>
      <div className={["flex", "flex-row", "flex items-center", "gap-2"].filter(Boolean).join(" ")}>
        <div className={["border", "border-border", "p-4", "w-3", "bg-zinc-600", "rounded-DEFAULT", "h-0.5"].filter(Boolean).join(" ")} role={region} />
        <p className={["text-xs", "text-zinc-400"].filter(Boolean).join(" ")}>
          {"Idle channel"}
        </p>
      </div>
      <div className={["flex", "flex-row", "flex items-center", "gap-2"].filter(Boolean).join(" ")}>
        <div className={["border", "border-border", "p-4", "w-2", "h-2", "rounded-full", "bg-rose-500"].filter(Boolean).join(" ")} role={region} />
        <p className={["text-xs", "text-zinc-400"].filter(Boolean).join(" ")}>
          {"Error node"}
        </p>
      </div>
    </div>
  );
}
