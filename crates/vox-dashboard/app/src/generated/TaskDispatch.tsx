import React, { useState } from "react";

export function TaskDispatch(): React.ReactElement {
  const [is_running, set_is_running] = useState(false);
  return (
    <div className={["flex", "flex-col", "border-t-true", "border-white/10", "p-4", "shrink-0", "gap-3"].filter(Boolean).join(" ")}>
      <div className={["flex", "flex-row", "flex items-center", "gap-3"].filter(Boolean).join(" ")}>
        <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "h-8", "text-xs", "px-4", "py-2", "rounded-xl", (is_running ? "bg-rose-600/80" : "bg-blue-600"), "text-white", (is_running ? "" : "hover:bg-blue-500")].filter(Boolean).join(" ")} onClick={() => {
        set_is_running(!is_running);
    }}>
          {(is_running ? "STOP" : "RUN BUILD")}
        </button>
        <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "h-8", "text-xs", "px-4", "py-2", "rounded-xl", "bg-white/5", "text-zinc-400", "border", "border-white/10"].filter(Boolean).join(" ")}>
          {"CLEAR"}
        </button>
        <div className={["bg-background", "rounded-lg", "border", "border-border", "p-4", "flex-1"].filter(Boolean).join(" ")} role={region} />
        <p className={["text-xs", "font-bold", "uppercase", (is_running ? "text-blue-400" : "text-zinc-600")].filter(Boolean).join(" ")}>
          {(is_running ? "BUILDING…" : "IDLE")}
        </p>
      </div>
      <p className={["text-xs", "text-zinc-600"].filter(Boolean).join(" ")}>
        {"Transport bridge: Phase 2 — output streams via React interop"}
      </p>
    </div>
  );
}
