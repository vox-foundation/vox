import React from "react";

export interface RowsProps {
  rows: Array;
}

export function Rows({ rows }: RowsProps): React.ReactElement {
  return (
    <div className={"rows"}>
      {rows.map((r, i) => (<>
      <span
    >
      {r.name}
    </span>
      <span
    >
      {r.value}
    </span>
    </>))}
    </div>
  );
}
