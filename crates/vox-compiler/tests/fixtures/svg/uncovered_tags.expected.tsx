import React from "react";

export function Filtered(): React.ReactElement {
  return (
<svg viewBox={"0 0 100 100"}
>
  <defs
>
  <linearGradient id={"lg"} x1={"0"} y1={"0"} x2={"1"} y2={"1"}
>
  <stop offset={"0%"} stopColor={"#34d399"} />
  <stop offset={"100%"} stopColor={"#2563eb"} />
</linearGradient>
  <filter id={"blur"}
>
  <feGaussianBlur stdDeviation={"2"} />
</filter>
</defs>
  <rect width={"100"} height={"100"} fill={"url(#lg)"} />
  <foreignObject x={"10"} y={"10"} width={"80"} height={"80"}
>
  <div
>
  {html}
  {inside}
  {svg}
</div>
</foreignObject>
</svg>
  );
}
