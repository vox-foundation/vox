import React from "react";

export interface PipelineCardProps {
  last: boolean;
}

export function PipelineCard({ last }: PipelineCardProps): React.ReactElement {
  return (
    <div className={last}>
      <span>
        {"card"}
      </span>
    </div>
  );
}

import React, { useState } from "react";

import { PipelineCard } from "./PipelineCard";

export function Stages(): React.ReactElement {
  const [stages, set_stages] = useState(5);
  const [i, set_i] = useState(0);
  return (
    <PipelineCard last={i === stages} />
  );
}

import React from "react";

export interface MixedPropsProps {
  label: string;
  size: number;
}

export function MixedProps({ label, size }: MixedPropsProps): React.ReactElement {
  return (
<button icon={"play"} size={size} className={label}
>
  <span
>
  {label}
</span>
</button>
  );
}
