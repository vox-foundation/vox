import React, { useState } from "react";

export function AppShell(): React.ReactElement {
  const [active, set_active] = useState("mesh");
  const [rail_collapsed, set_rail_collapsed] = useState(false);
  return (
    <div className={["flex", "flex-col", "min-h-screen", "overflow-hidden", "bg-zinc-950", "text-zinc-100", "font-sans"].filter(Boolean).join(" ")}>
      <TopBar run_label={"idle"} run_status={"idle"} workspace={"aurelia-mesh"} />
      <div className={["flex", "flex-row", "flex-1", "min-h-0", "overflow-hidden"].filter(Boolean).join(" ")}>
        <LeftRail active={active} collapsed={rail_collapsed} on_code={(() => ((() => {
        set_active("code");
      })()))} on_forge={(() => ((() => {
        set_active("forge");
      })()))} on_mesh={(() => ((() => {
        set_active("mesh");
      })()))} on_models={(() => ((() => {
        set_active("models");
      })()))} on_runs={(() => ((() => {
        set_active("runs");
      })()))} on_settings={(() => ((() => {
        set_active("settings");
      })()))} on_speak={(() => ((() => {
        set_active("speak");
      })()))} />
        <div className={["rounded-lg", "border", "border-border", "p-4", "flex-1", "min-w-0", "overflow-hidden", "bg-zinc-950", "flex flex-col"].filter(Boolean).join(" ")} role={region}>
          {(active === "speak" ? <SpeakSurface  /> : (active === "mesh" ? <MeshSurface  /> : (active === "forge" ? <ForgeSurface  /> : (active === "code" ? <CodeSurface  /> : (active === "models" ? <ModelsSurface  /> : (active === "runs" ? <RunsSurface  /> : <SettingsSurface  />))))))}
        </div>
      </div>
      <StatusBar active={active} />
    </div>
  );
}
