import React, { useState } from "react";

export function AppShell(): React.ReactElement {
  const [active, set_active] = useState("mesh");
  const [rail_collapsed, set_rail_collapsed] = useState(false);
  return (
<column min_h={"screen"} overflow={"hidden"} bg={"zinc.950"} color={"zinc.100"} fontFamily={"sans"}
>
  <TopBar workspace={"aurelia-mesh"} run_status={"idle"} run_label={"idle"} />
  <row flex={1} min_h={0} overflow={"hidden"}
>
  <LeftRail active={active} collapsed={rail_collapsed} on_speak={(() => ((() => {
    set_active("speak");
  })()))} on_mesh={(() => ((() => {
    set_active("mesh");
  })()))} on_forge={(() => ((() => {
    set_active("forge");
  })()))} on_code={(() => ((() => {
    set_active("code");
  })()))} on_models={(() => ((() => {
    set_active("models");
  })()))} on_runs={(() => ((() => {
    set_active("runs");
  })()))} on_settings={(() => ((() => {
    set_active("settings");
  })()))} />
  <panel flex={1} min_w={0} overflow={"hidden"} bg={"zinc.950"} raw_class={"flex flex-col"}
>
  {(active === "speak" ? <SpeakSurface  /> : (active === "mesh" ? <MeshSurface  /> : (active === "forge" ? <ForgeSurface  /> : (active === "code" ? <CodeSurface  /> : (active === "models" ? <ModelsSurface  /> : (active === "runs" ? <RunsSurface  /> : <SettingsSurface  />))))))}
</panel>
</row>
  <StatusBar active={active} />
</column>
  );
}
