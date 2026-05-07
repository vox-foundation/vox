import React from "react";

import { ComposerPanel } from "./ComposerPanel";

export function SpeakTab(): React.ReactElement {
  return (
    <div className={["flex", "flex-col", "flex-1", "overflow-hidden", "bg-zinc-950"].filter(Boolean).join(" ")}>
      <div className={["flex", "flex-row", "h-12", "border-b-true", "border-zinc-800", "px-6", "flex items-center", "flex justify-between", "shrink-0"].filter(Boolean).join(" ")}>
        <div className={["flex", "flex-col", "gap-0"].filter(Boolean).join(" ")}>
          <p className={["text-sm", "text-white", "tracking-tighter"].filter(Boolean).join(" ")}>
            {"LOQUELA"}
          </p>
          <p className={["text-xs", "text-zinc-500", "tracking-widest"].filter(Boolean).join(" ")}>
            {"VOICE INTERFACE"}
          </p>
        </div>
        <div className={["flex", "flex-row", "flex items-center", "gap-2"].filter(Boolean).join(" ")}>
          <div className={["border", "border-border", "p-4", "w-2", "h-2", "rounded-full", "bg-zinc-600"].filter(Boolean).join(" ")} role={region} />
          <p className={["text-xs", "text-zinc-500"].filter(Boolean).join(" ")}>
            {"NO ACTIVE SESSION"}
          </p>
        </div>
      </div>
      <div className={["bg-background", "rounded-lg", "border", "border-border", "flex-1", "overflow-y-auto", "py-4"].filter(Boolean).join(" ")} role={region}>
        <div className={["flex", "flex-col", "flex items-center", "flex justify-center", "h-full", "text-zinc-500", "gap-4"].filter(Boolean).join(" ")}>
          <p className={["text-sm", "font-bold", "uppercase", "tracking-widest"].filter(Boolean).join(" ")}>
            {"START A CONVERSATION"}
          </p>
          <p className={"text-xs"}>
            {"Messages will appear here once a session is active."}
          </p>
        </div>
      </div>
      <ComposerPanel />
    </div>
  );
}
