//! Assert TanStack Start scaffold file layout (no Node).
//! Blueprint OP-0169: Start-mode scaffold stays compatible with Express `server.ts` emission flags
//! (`VOX_EMIT_EXPRESS_SERVER`); route tree is client-only here.
#![allow(missing_docs)]

use std::fs;

use vox_cli::frontend;
use vox_cli::templates;

#[test]
fn tanstack_start_programmatic_layout_writes_reexport_and_skips_index_route() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ts_out = tmp.path().join("ts_out");
    let app = tmp.path().join("app");
    fs::create_dir_all(&ts_out).expect("ts_out");
    fs::write(
        ts_out.join("VoxTanStackRouter.tsx"),
        "// stub\nexport const voxRouteTree = {} as never;\n",
    )
    .expect("vox");
    fs::write(
        ts_out.join("Home.tsx"),
        "export function Home() { return null; }\n",
    )
    .expect("home");

    frontend::scaffold_react_app(&app, &ts_out, true).expect("scaffold");

    assert!(
        app.join("src/routeTree.gen.ts").is_file(),
        "routeTree.gen.ts missing"
    );
    let rt = fs::read_to_string(app.join("src/routeTree.gen.ts")).expect("read routeTree.gen");
    assert!(
        rt.contains("VoxTanStackRouter"),
        "expected re-export from VoxTanStackRouter, got: {rt}"
    );
    assert!(
        !app.join("src/routes/index.tsx").exists(),
        "programmatic Start must not write routes/index.tsx"
    );
    assert!(app.join("src/router.tsx").is_file());
    assert!(app.join("src/routes/__root.tsx").is_file());
}

#[test]
fn tanstack_start_file_route_fallback_writes_index() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ts_out = tmp.path().join("ts_out");
    let app = tmp.path().join("app");
    fs::create_dir_all(&ts_out).expect("ts_out");
    fs::write(
        ts_out.join("Chat.tsx"),
        "export function Chat() { return null; }\n",
    )
    .expect("chat");

    frontend::scaffold_react_app(&app, &ts_out, true).expect("scaffold");

    assert!(app.join("src/routes/index.tsx").is_file());
    let idx = fs::read_to_string(app.join("src/routes/index.tsx")).expect("idx");
    assert!(idx.contains("Chat"));
}

#[test]
fn route_tree_reexport_template_contains_router_type_import() {
    let s = templates::tanstack_start_route_tree_gen_reexport();
    assert!(s.contains("voxRouteTree"));
    assert!(s.contains("./router"));
}
