import React, { useState } from "react";

export function ComposerPanel(): React.ReactElement {
  const [is_submitting, set_is_submitting] = useState(false);
  return (
<div className={["flex", "flex-col", "border-t-true", "border-white/10", "p-4", "gap-3", "bg-zinc-950/80"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-row", "gap-3", "flex items-end"].filter(Boolean).join(" ")}
>
  <div className={["flex-1", "bg-zinc-900", "border", "border-white/10", "rounded-2xl", "px-4", "py-3", "min-h-16"].filter(Boolean).join(" ")}
>
  <p className={["text-sm", "text-white/50", "italic"].filter(Boolean).join(" ")}
>
  {"Type a message…"}
</p>
</div>
  <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "px-4", "py-2", "w-12", "h-12", "rounded-xl", (is_submitting ? "bg-blue-600/50" : "bg-blue-600"), (is_submitting ? "text-white/50" : "text-white"), "flex items-center justify-center"].filter(Boolean).join(" ")} onClick={() => {
    set_is_submitting(!is_submitting);
}}
>
  {(is_submitting ? "…" : "→")}
</button>
</div>
  <div className={["flex", "flex-row", "flex justify-between", "flex items-center", "px-1"].filter(Boolean).join(" ")}
>
  <p className={["text-xs", "text-zinc-500"].filter(Boolean).join(" ")}
>
  {"Shift+Enter for new line"}
</p>
  <p className={["text-xs", "text-zinc-600"].filter(Boolean).join(" ")}
>
  {"Transport bridge: Phase 2"}
</p>
</div>
</div>
  );
}
