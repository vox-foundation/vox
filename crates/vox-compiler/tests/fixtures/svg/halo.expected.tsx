import React from "react";

export function Halo(): React.ReactElement {
  return (
<svg viewBox={"0 0 100 60"} preserveAspectRatio={"xMidYMid meet"}
>
  <defs
>
  <pattern id={"g"} width={"10"} height={"10"} patternUnits={"userSpaceOnUse"}
>
  <path d={"M 10 0 L 0 0 0 10"} strokeWidth={"1"} stroke={"rgba(255,255,255,0.1)"} fill={"none"} />
</pattern>
  <radialGradient id={"halo"} cx={"0.5"} cy={"0.5"} r={"0.5"}
>
  <stop offset={"0%"} stopColor={"#34d399"} stopOpacity={"0.4"} />
  <stop offset={"100%"} stopColor={"#34d399"} stopOpacity={"0"} />
</radialGradient>
</defs>
  <rect width={"100"} height={"60"} fill={"url(#g)"} />
</svg>
  );
}
