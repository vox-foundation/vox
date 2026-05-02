import React from "react";

export interface ChatMessageProps {
  role: string;
  content: string;
}

export function ChatMessage({ role, content }: ChatMessageProps): React.ReactElement {
  return (
<row className={((role === "user") ? (() => { "justify-end px-4 py-2";
 })() : (() => { "justify-start px-4 py-2";
 })())}
>
  <panel className={((role === "user") ? (() => { "max-w-xl bg-blue-600/20 border border-blue-500/30 rounded-2xl rounded-br-sm px-4 py-3";
 })() : (() => { "max-w-2xl bg-white/5 border border-white/10 rounded-2xl rounded-bl-sm px-4 py-3";
 })())}
>
  <text className={"text-xs font-bold text-zinc-400 uppercase tracking-widest mb-2"}
>
  {(() => {
    role;
  })()}
</text>
  <text className={"text-sm text-white/80 leading-relaxed"}
>
  {(() => {
    content;
  })()}
</text>
</panel>
</row>
  );
}
