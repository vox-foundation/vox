import React from "react";

export interface MatrixProps {
  matrix: Array;
}

export function Matrix({ matrix }: MatrixProps): React.ReactElement {
  return (
    <div className={"matrix"}>
      {matrix.map((row, i) => (<div className={"row"}
    >
      {row.map((cell, j) => (<span
    >
      {cell}
    </span>))}
    </div>))}
    </div>
  );
}
