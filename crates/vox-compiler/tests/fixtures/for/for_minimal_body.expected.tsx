import React from "react";

export interface EmptyProps {
  rows: Array;
}

export function Empty({ rows }: EmptyProps): React.ReactElement {
  return (
    <div className={"list"}>
      {rows.map((r, _i) => (<span
    >

    </span>))}
    </div>
  );
}
