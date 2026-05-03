import React from "react";

export interface PairProps {
  a: string;
  b: string;
}

export function Pair({ a, b }: PairProps): React.ReactElement {
  return (
    <>
    <span>
      {a}
    </span>
    <span>
      {b}
    </span>
    </>
  );
}
