// Generated from crates/vox-dashboard/app/src/tabs/speak.vox
// DO NOT EDIT — edit the .vox source and re-run `vox build`.
import React, { useState } from "react";

type ChatSessionState = "Idle" | "Composing" | "Submitting";

function ChatMessage({ role, content }: { role: string; content: string }): React.ReactElement {
  return (
<row className={(role === "user" ? "justify-end px-4 py-2" : "justify-start px-4 py-2")}
>
  <panel className={(role === "user" ? "max-w-xl bg-blue-600/20 border border-blue-500/30 rounded-2xl rounded-br-sm px-4 py-3" : "max-w-2xl bg-white/5 border border-white/10 rounded-2xl rounded-bl-sm px-4 py-3")}
>
  <text className={"text-xs font-bold text-zinc-400 uppercase tracking-widest mb-2"}
>
  {role}
</text>
  <text className={"text-sm text-white/80 leading-relaxed"}
>
  {content}
</text>
</panel>
</row>
  );
}

function ComposerPanel(): React.ReactElement {
  const [is_submitting, set_is_submitting] = useState(false);
  return (
<column className={"border-t border-white/10 p-4 gap-3 bg-zinc-950/80"}
>
  <row className={"gap-3 items-end"}
>
  <panel className={"flex-1 bg-zinc-900 border border-white/10 rounded-2xl px-4 py-3 min-h-16"}
>
  <text className={"text-sm text-white/50 italic"}
>
  {"Type a message…"}
</text>
</panel>
  <button className={(is_submitting ? "w-12 h-12 rounded-xl bg-blue-600/50 text-white/50 flex items-center justify-center" : "w-12 h-12 rounded-xl bg-blue-600 text-white flex items-center justify-center")} onClick={() => {
    set_is_submitting(!is_submitting);
}}
>
  {(is_submitting ? "…" : "→")}
</button>
</row>
  <row className={"justify-between items-center px-1"}
>
  <text className={"text-xs text-zinc-500"}
>
  {"Shift+Enter for new line"}
</text>
  <text className={"text-xs text-zinc-600"}
>
  {"Transport bridge: Phase 2"}
</text>
</row>
</column>
  );
}

export function SpeakTab(): React.ReactElement {
  return (
<column className={"flex-1 overflow-hidden bg-zinc-950"}
>
  <row className={"h-12 border-b border-zinc-800 px-6 items-center justify-between shrink-0"}
>
  <column className={"gap-0"}
>
  <text className={"text-sm font-black tracking-tighter text-white"}
>
  {"LOQUELA"}
</text>
  <text className={"text-xs text-zinc-500 tracking-widest"}
>
  {"VOICE INTERFACE"}
</text>
</column>
  <row className={"gap-2 items-center"}
>
  <panel className={"w-2 h-2 rounded-full bg-zinc-600"}
>
</panel>
  <text className={"text-xs text-zinc-500"}
>
  {"NO ACTIVE SESSION"}
</text>
</row>
</row>
  <panel className={"flex-1 overflow-y-auto py-4"}
>
  <column className={"items-center justify-center h-full gap-4 text-zinc-500"}
>
  <text className={"text-sm font-bold uppercase tracking-widest"}
>
  {"START A CONVERSATION"}
</text>
  <text className={"text-xs"}
>
  {"Messages will appear here once a session is active."}
</text>
</column>
</panel>
  <ComposerPanel  />
</column>
  );
}
