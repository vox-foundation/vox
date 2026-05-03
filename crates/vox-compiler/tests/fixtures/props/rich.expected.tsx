import React from "react";

export interface BoxProps {
  width: number;
  label: string;
}

export function Box({ width, label }: BoxProps): React.ReactElement {
  return (
<div style={{ width: width, padding: 8 }} className={"card"}
>
  <span
>
  {label}
</span>
</div>
  );
}

import React, { useState } from "react";

import { Box } from "./Box";

export function Page(): React.ReactElement {
  const [count, set_count] = useState(12);
  return (
<Box width={count * 16} label={"hi"} />
  );
}
