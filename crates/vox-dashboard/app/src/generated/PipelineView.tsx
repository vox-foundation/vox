import React from "react";

import { PipelineStage } from "./PipelineStage";

export function PipelineView(): React.ReactElement {
  return (
<div className={["flex", "flex-row", "h-full", "bg-zinc-950"].filter(Boolean).join(" ")}
>
  <PipelineStage name={"Lexer"} desc={"Logos-based tokenization"} />
  <PipelineStage name={"Parser"} desc={"Rowan GreenTree CST generation"} />
  <PipelineStage name={"HIR"} desc={"High-level IR with name resolution"} />
  <PipelineStage name={"TypeCheck"} desc={"Bidirectional unification logic"} />
  <PipelineStage name={"CodeGen"} desc={"Rust and TypeScript emission"} />
</div>
  );
}
