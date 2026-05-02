import React from "react";

export interface DiagnosticRowProps {
  severity: string;
  message: string;
  location: string;
}

export function DiagnosticRow({ severity, message, location }: DiagnosticRowProps): React.ReactElement {
  return (
<row className={"px-4 py-3 border-b border-white/5 gap-4 items-start hover:bg-white/5"}
>
  <panel className={((severity === "error") ? (() => { "w-2 h-2 rounded-full bg-rose-500 mt-1.5 shrink-0";
 })() : (() => { "w-2 h-2 rounded-full bg-amber-400 mt-1.5 shrink-0";
 })())}
>
  
</panel>
  <column className={"flex-1 gap-1 min-w-0"}
>
  <text className={"text-sm text-white/80 font-mono leading-snug"}
>
  {(() => {
    message;
  })()}
</text>
  <text className={"text-xs text-zinc-500 font-mono"}
>
  {(() => {
    location;
  })()}
</text>
</column>
  <text className={((severity === "error") ? (() => { "text-xs font-bold text-rose-400 uppercase shrink-0";
 })() : (() => { "text-xs font-bold text-amber-400 uppercase shrink-0";
 })())}
>
  {(() => {
    severity;
  })()}
</text>
</row>
  );
}
