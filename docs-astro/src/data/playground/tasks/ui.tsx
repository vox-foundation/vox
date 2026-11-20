import React from "react";

export function TaskBoard(): React.ReactElement {
  return (
    <div className="px-4">
    <div className={["flex", "flex-col", "task-card"].filter(Boolean).join(" ")}>
      <h3 className={["text-xl", "font-semibold"].filter(Boolean).join(" ")}>
        {"Active Tasks"}
      </h3>
      <p className={"text-base"}>
        {"Typed task board with Result[T]"}
      </p>
    </div>
    </div>
  );
}
