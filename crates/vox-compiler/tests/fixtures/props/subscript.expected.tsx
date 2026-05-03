import React from "react";

export interface IndexedProps {
  items: Array;
  i: number;
}

export function Indexed({ items, i }: IndexedProps): React.ReactElement {
  return (
    <div>
      <span>
        {items[0]}
      </span>
      <span>
        {items[i]}
      </span>
      <span>
        {items[i + 1]}
      </span>
    </div>
  );
}
