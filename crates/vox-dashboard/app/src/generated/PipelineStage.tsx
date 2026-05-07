import React from "react";

export interface PipelineStageProps {
  name: string;
  desc: string;
}

export function PipelineStage({ name, desc }: PipelineStageProps): React.ReactElement {
  return (
    <div className={["flex", "flex-col", "p-8", "flex-1", "border-r-true", "border-white/5", "gap-4"].filter(Boolean).join(" ")}>
      <div className={["flex", "flex-row", "flex justify-between", "mb-6"].filter(Boolean).join(" ")}>
        <div className={["p-4", "w-10", "h-10", "rounded-xl", "bg-zinc-900", "border", "border-white/5", "flex items-center", "flex justify-center"].filter(Boolean).join(" ")} role={region}>
          <p className={["text-xs", "text-zinc-500", "font-mono"].filter(Boolean).join(" ")}>
            {name}
          </p>
        </div>
        <p className={["text-xs", "font-bold", "text-rose-500", "bg-rose-500/10", "px-2", "py-1", "rounded-DEFAULT", "border", "border-rose-500/20"].filter(Boolean).join(" ")}>
          {"IDLE"}
        </p>
      </div>
      <p className={["text-2xl", "font-bold", "text-white/90"].filter(Boolean).join(" ")}>
        {name}
      </p>
      <p className={["text-sm", "text-zinc-500", "leading-relaxed"].filter(Boolean).join(" ")}>
        {desc}
      </p>
      <div className={["bg-background", "flex-1", "rounded-2xl", "border", "border-white/5", "p-5", "font-mono"].filter(Boolean).join(" ")} role={region}>
        <p className={["text-xs", "text-zinc-500", "italic"].filter(Boolean).join(" ")}>
          {"No output yet."}
        </p>
      </div>
    </div>
  );
}
