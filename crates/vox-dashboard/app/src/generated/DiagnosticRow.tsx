import React from "react";

export interface DiagnosticRowProps {
  severity: string;
  message: string;
  location: string;
}

export function DiagnosticRow({ severity, message, location }: DiagnosticRowProps): React.ReactElement {
  return (
    <div className={["flex", "flex-row", "px-4", "py-3", "border-b-true", "border-white/5", "flex items-start", "gap-4", "hover:bg-white/5"].filter(Boolean).join(" ")}>
      <div className={["border", "border-border", "p-4", "w-2", "h-2", "rounded-full", (severity === "error" ? "bg-rose-500" : "bg-amber-400"), "mt-1.5", "shrink-0"].filter(Boolean).join(" ")} role={region} />
      <div className={["flex", "flex-col", "flex-1", "min-w-0", "gap-1"].filter(Boolean).join(" ")}>
        <p className={["text-sm", "text-white/80", "font-mono", "leading-snug"].filter(Boolean).join(" ")}>
          {message}
        </p>
        <p className={["text-xs", "text-zinc-500", "font-mono"].filter(Boolean).join(" ")}>
          {location}
        </p>
      </div>
      <p className={["text-xs", "font-bold", "uppercase", "shrink-0", (severity === "error" ? "text-rose-400" : "text-amber-400")].filter(Boolean).join(" ")}>
        {severity}
      </p>
    </div>
  );
}
