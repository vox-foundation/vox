import React, { useState } from "react";

export function WorkflowScrubber(): React.ReactElement {
  const [is_playing, set_is_playing] = useState(false);
  return (
<div className={["flex", "flex-col", "p-10", "bg-zinc-950", "h-full", "text-white", "gap-8"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-row", "justify-between", "items-center"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-col", "gap-2"].filter(Boolean).join(" ")}
>
  <p className={["text-3xl", "text-white", "tracking-tighter"].filter(Boolean).join(" ")}
>
  {"TIME TRAVEL"}
</p>
  <p className={["text-xs", "font-bold", "text-zinc-400", "tracking-widest"].filter(Boolean).join(" ")}
>
  {"DURABLE WORKFLOW STATE INSPECTOR"}
</p>
</div>
  <div className={["flex", "flex-row", "bg-white/5", "p-2", "rounded-2xl", "border", "border-white/5", "items-center", "gap-4"].filter(Boolean).join(" ")}
>
  <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "hover:bg-primary/90", "px-4", "py-2", "w-10", "h-10", "rounded-xl", "bg-white/5", "text-zinc-400"].filter(Boolean).join(" ")}
>
  {"<<"}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "hover:bg-primary/90", "px-4", "py-2", "w-12", "h-12", "rounded-xl", "bg-blue-600", "text-white", "flex items-center justify-center"].filter(Boolean).join(" ")} onClick={() => {
    set_is_playing(!is_playing);
}}
>
  {(is_playing ? "PAUSE" : "PLAY")}
</button>
  <button className={["inline-flex", "items-center", "justify-center", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "hover:bg-primary/90", "px-4", "py-2", "w-10", "h-10", "rounded-xl", "bg-white/5", "text-zinc-400"].filter(Boolean).join(" ")}
>
  {">>"}
</button>
</div>
</div>
  <div className={["bg-background", "rounded-lg", "border", "border-border", "p-4", "flex-1"].filter(Boolean).join(" ")}
>
  <div className={["flex", "flex-col", "items-center", "text-zinc-500", "gap-4"].filter(Boolean).join(" ")}
>
  <p className={["text-sm", "font-bold", "uppercase", "tracking-widest"].filter(Boolean).join(" ")}
>
  {"NO ACTIVE WORKFLOW"}
</p>
  <p className={"text-xs"}
>
  {"Durable state execution will appear here when orchestrated."}
</p>
</div>
</div>
</div>
  );
}
