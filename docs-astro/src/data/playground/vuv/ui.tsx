import React, { useState } from "react";

export interface CounterProps {
  label: string;
}

export function Counter({ label }: CounterProps): React.ReactElement {
  const [count, set_count] = useState(0);
  return (
    <div className="px-4">
    <div className={["flex", "flex-col", "gap-2", "p-4"].filter(Boolean).join(" ")}>
      <h3 className={["text-xl", "font-semibold"].filter(Boolean).join(" ")}>
        {label}
      </h3>
      <p className={["text-lg", "font-bold"].filter(Boolean).join(" ")}>
        {count}
      </p>
      <button className={["inline-flex", "items-center", "justify-center", "rounded-md", "text-sm", "font-medium", "ring-offset-background", "transition-colors", "focus-visible:outline-none", "focus-visible:ring-2", "focus-visible:ring-ring", "focus-visible:ring-offset-2", "disabled:pointer-events-none", "disabled:opacity-50", "border", "border-input", "bg-background", "hover:bg-accent", "hover:text-accent-foreground", "h-10", "px-4", "py-2"].filter(Boolean).join(" ")} onClick={() => {
        set_count(count + 1);
    }}>
        {"Add one"}
      </button>
    </div>
    </div>
  );
}
import React from "react";

import { Counter } from "./Counter";

export function Page(): React.ReactElement {
  return (
    <div className={["flex", "flex-row", "flex-wrap", "gap-4"].filter(Boolean).join(" ")}>
      <Counter label={"First"} />
      <Counter label={"Second"} />
    </div>
  );
}
