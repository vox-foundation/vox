import React from "react";

export interface ChatMessageProps {
  role: string;
  content: string;
}

export function ChatMessage({ role, content }: ChatMessageProps): React.ReactElement {
  return (
<div className={["flex", "flex-row", "px-4", "py-2", (role === "user" ? "flex justify-end" : "flex justify-start")].filter(Boolean).join(" ")}
>
  <div className={[(role === "user" ? "max-w-xl" : "max-w-2xl"), (role === "user" ? "bg-blue-600/20" : "bg-white/5"), "border", (role === "user" ? "border-blue-500/30" : "border-white/10"), "rounded-2xl", (role === "user" ? "rounded-br-sm" : "rounded-br-2xl"), (role === "user" ? "rounded-bl-2xl" : "rounded-bl-sm"), "px-4", "py-3"].filter(Boolean).join(" ")}
>
  <p className={["text-xs", "font-bold", "text-zinc-400", "uppercase", "tracking-widest", "mb-2"].filter(Boolean).join(" ")}
>
  {role}
</p>
  <p className={["text-sm", "text-white/80", "leading-relaxed"].filter(Boolean).join(" ")}
>
  {content}
</p>
</div>
</div>
  );
}
