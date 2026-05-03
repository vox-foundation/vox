import React from "react";

export interface NoIndexProps {
  rows: Array;
}

export function NoIndex({ rows }: NoIndexProps): React.ReactElement {
  return (
    <div className={"list"}>
      {rows.map((r, _i) => (<span
    >
      {r.name}
    </span>))}
    </div>
  );
}
