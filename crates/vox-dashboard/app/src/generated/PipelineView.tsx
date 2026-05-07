import React from "react";

import { PipelineStage } from "./PipelineStage";

export function PipelineView(): React.ReactElement {
  return (
    <div className={["flex", "flex-row", "h-full", "bg-zinc-950"].filter(Boolean).join(" ")}>
      <PipelineStage desc={"Logos-based tokenization"} name={"Lexer"} />
      <PipelineStage desc={"Rowan GreenTree CST generation"} name={"Parser"} />
      <PipelineStage desc={"High-level IR with name resolution"} name={"HIR"} />
      <PipelineStage desc={"Bidirectional unification logic"} name={"TypeCheck"} />
      <PipelineStage desc={"Rust and TypeScript emission"} name={"CodeGen"} />
    </div>
  );
}
