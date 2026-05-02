import React from "react";

export function DiagnosticsPanel(): React.ReactElement {
  return (
<column className={"flex-1 overflow-y-auto"}
>
  <row className={"px-4 py-2 border-b border-white/10 justify-between items-center shrink-0 bg-zinc-900/50"}
>
  <text className={"text-xs font-bold text-zinc-400 uppercase tracking-widest"}
>
  {"DIAGNOSTICS"}
</text>
  <text className={"text-xs text-zinc-600"}
>
  {"0 errors · 0 warnings"}
</text>
</row>
  <column className={"flex-1 items-center justify-center gap-3 text-zinc-500"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"NO DIAGNOSTICS"}
</text>
  <text className={"text-xs"}
>
  {"Run a build to see compiler output here."}
</text>
</column>
</column>
  );
}
