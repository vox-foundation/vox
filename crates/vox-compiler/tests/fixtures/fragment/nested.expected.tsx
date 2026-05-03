import React from "react";

export interface NestedProps {
  x: string;
}

export function Nested({ x }: NestedProps): React.ReactElement {
  return (
    <>
    <>
    <span>
      {x}
    </span>
    </>
    </>
  );
}
