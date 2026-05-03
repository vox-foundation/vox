import React from "react";

export interface RunsTableProps {
  rows: Array;
}

export function RunsTable({ rows }: RunsTableProps): React.ReactElement {
  return (
    <div className={"table"}>
      {rows.map((r, i) => (<div className={"row"} key={r.id}
    >
      <span
    >
      {r.id}
    </span>
      <span
    >
      {r.duration}
    </span>
    </div>))}
    </div>
  );
}
