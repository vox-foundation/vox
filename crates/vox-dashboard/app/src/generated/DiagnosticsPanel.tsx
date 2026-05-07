import React from "react";

export function DiagnosticsPanel(): React.ReactElement {
  return (
    <div className={["flex", "flex-col", "flex-1", "overflow-y-auto"].filter(Boolean).join(" ")}>
      <div className={["flex", "flex-row", "px-4", "py-2", "border-b-true", "border-white/10", "flex justify-between", "flex items-center", "shrink-0", "bg-zinc-900/50"].filter(Boolean).join(" ")}>
        <p className={["text-xs", "font-bold", "text-zinc-400", "uppercase", "tracking-widest"].filter(Boolean).join(" ")}>
          {"DIAGNOSTICS"}
        </p>
        <p className={["text-xs", "text-zinc-600"].filter(Boolean).join(" ")}>
          {"0 errors · 0 warnings"}
        </p>
      </div>
      <div className={["flex", "flex-col", "flex-1", "flex items-center", "flex justify-center", "text-zinc-500", "gap-3"].filter(Boolean).join(" ")}>
        <p className={["text-sm", "font-bold", "uppercase", "tracking-widest"].filter(Boolean).join(" ")}>
          {"NO DIAGNOSTICS"}
        </p>
        <p className={"text-xs"}>
          {"Run a build to see compiler output here."}
        </p>
      </div>
    </div>
  );
}
