//! Second `scaffold_react_app` run must not rewrite manifest-driven SPA entry (e3).

use std::fs;

use vox_cli::frontend;

#[test]
fn spa_scaffold_rerun_keeps_main_tsx_stable() {
    let base = std::env::temp_dir().join(format!("vox-scaffold-idem-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let app = base.join("app");
    let gen_dir = base.join("gen");
    fs::create_dir_all(&gen_dir).expect("gen dir");
    fs::write(
        gen_dir.join("routes.manifest.ts"),
        r#"import type { ComponentType } from "react"
import { Home } from "./Home.tsx"

export type VoxRoute = {
  path: string
  component: ComponentType<any>
}
export const voxRoutes: VoxRoute[] = [
  { path: "/", component: Home },
]
"#,
    )
    .expect("manifest");
    fs::write(
        gen_dir.join("Home.tsx"),
        "import React from \"react\";\nexport function Home(): React.ReactElement { return <div />; }\n",
    )
    .expect("Home");
    fs::write(
        gen_dir.join("vox-tanstack-query.tsx"),
        "import React from \"react\";\nexport function VoxQueryProvider(props: { children?: React.ReactNode }) { return <>{props.children}</>; }\n",
    )
    .expect("vox-tanstack-query");

    frontend::scaffold_react_app(&app, &gen_dir, false).expect("scaffold 1");
    let cj = fs::read_to_string(app.join("components.json")).expect("components.json");
    assert!(
        cj.contains("\"rsc\": false"),
        "SPA scaffold should write shadcn-compatible components.json"
    );
    let main1 = fs::read_to_string(app.join("src/main.tsx")).expect("main 1");
    frontend::scaffold_react_app(&app, &gen_dir, false).expect("scaffold 2");
    let main2 = fs::read_to_string(app.join("src/main.tsx")).expect("main 2");
    assert_eq!(
        main1, main2,
        "second scaffold should not churn main.tsx; diff would confuse users"
    );
    let router1 = fs::read_to_string(app.join("src/vox-manifest-router.tsx")).expect("router");
    assert!(
        router1.contains("voxRoutes"),
        "manifest router should consume voxRoutes"
    );
    let ad1 =
        fs::read_to_string(app.join("src/vox-manifest-route-adapter.tsx")).expect("adapter 1");
    frontend::scaffold_react_app(&app, &gen_dir, false).expect("scaffold 3");
    let ad2 =
        fs::read_to_string(app.join("src/vox-manifest-route-adapter.tsx")).expect("adapter 2");
    assert_eq!(
        ad1, ad2,
        "second+ scaffolds should not rewrite vox-manifest-route-adapter.tsx"
    );

    let _ = fs::remove_dir_all(&base);
}
