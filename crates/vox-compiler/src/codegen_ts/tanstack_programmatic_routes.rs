//! Programmatic TanStack Router trees from Vox `routes:` (`App.tsx` / `VoxTanStackRouter.tsx`).

use std::collections::BTreeSet;

use crate::ast::decl::RoutesDecl;
use crate::hir::HirModule;

#[must_use]
fn sorted_screen_imports(routes_decl: &RoutesDecl, hir: &HirModule) -> Vec<String> {
    let mut names = BTreeSet::new();
    for e in &routes_decl.entries {
        names.insert(e.component_name.clone());
    }
    if let Some(l) = hir.loadings.first() {
        names.insert(l.0.func.name.clone());
    }
    names.into_iter().collect()
}

/// TanStack Router `path` option for a Vox `routes:` entry (`/` → root index).
#[must_use]
pub fn tanstack_path_literal(vox_path: &str) -> String {
    let t = vox_path.trim();
    if t == "/" || t.is_empty() {
        return "'/'".to_string();
    }
    let rest = t.trim_start_matches('/');
    let esc = rest.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{esc}'")
}

/// Stable `const` name for each `createRoute` (`route_0_chat`, etc.).
#[must_use]
pub fn tanstack_route_var_id(index: usize, path: &str) -> String {
    let mut s: String = path
        .trim()
        .trim_start_matches('/')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if s.is_empty() {
        s = "index".to_string();
    }
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s = format!("p_{s}");
    }
    format!("route_{index}_{s}")
}

/// Append `App.tsx` (SPA) and/or `VoxTanStackRouter.tsx` (Start) for each `routes:` block.
pub fn push_route_tree_files(
    files: &mut Vec<(String, String)>,
    hir: &HirModule,
    tanstack_start: bool,
) {
    for hir_routes in &hir.client_routes {
        let routes_decl = &hir_routes.0;
        let pending = hir.loadings.first().map(|l| l.0.func.name.clone());
        if tanstack_start {
            let mut s = String::new();
            s.push_str(
                "// Programmatic route tree for TanStack Start — export voxRouteTree; SPA shell provides the provider.\n",
            );
            s.push_str("import React from \"react\";\n");
            s.push_str(
                "import {\n  Outlet,\n  createRootRoute,\n  createRoute,\n} from \"@tanstack/react-router\";\n",
            );
            for name in sorted_screen_imports(routes_decl, hir) {
                s.push_str(&format!("import {{ {name} }} from \"./{name}.tsx\";\n",));
            }
            s.push_str("\nconst rootRoute = createRootRoute({\n");
            s.push_str("  component: () => <Outlet />,\n");
            s.push_str("});\n\n");

            for (i, entry) in routes_decl.entries.iter().enumerate() {
                let id = tanstack_route_var_id(i, &entry.path);
                let path_lit = tanstack_path_literal(&entry.path);
                let pend = pending
                    .as_ref()
                    .map(|p| format!("  pendingComponent: {p},\n"))
                    .unwrap_or_default();
                s.push_str(&format!(
                    "const {id} = createRoute({{\n  getParentRoute: () => rootRoute,\n  path: {path_lit},\n{pend}  component: {},\n}});\n\n",
                    entry.component_name
                ));
            }

            let child_ids: Vec<String> = routes_decl
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| tanstack_route_var_id(i, &e.path))
                .collect();
            s.push_str("const routeTree = rootRoute.addChildren([");
            s.push_str(&child_ids.join(", "));
            s.push_str("]);\n\n");
            s.push_str("export const voxRouteTree = routeTree;\n");
            files.push(("VoxTanStackRouter.tsx".to_string(), s));
        } else {
            let mut app = String::new();
            app.push_str("import React from \"react\";\n");
            app.push_str(
                "import {\n  Outlet,\n  RouterProvider,\n  createRootRoute,\n  createRoute,\n  createRouter,\n} from \"@tanstack/react-router\";\n",
            );
            for name in sorted_screen_imports(routes_decl, hir) {
                app.push_str(&format!("import {{ {name} }} from \"./{name}.tsx\";\n",));
            }
            app.push_str("\nconst rootRoute = createRootRoute({\n");
            app.push_str("  component: () => <Outlet />,\n");
            app.push_str("});\n\n");

            for (i, entry) in routes_decl.entries.iter().enumerate() {
                let id = tanstack_route_var_id(i, &entry.path);
                let path_lit = tanstack_path_literal(&entry.path);
                let pend = pending
                    .as_ref()
                    .map(|p| format!("  pendingComponent: {p},\n"))
                    .unwrap_or_default();
                app.push_str(&format!(
                    "const {id} = createRoute({{\n  getParentRoute: () => rootRoute,\n  path: {path_lit},\n{pend}  component: {},\n}});\n\n",
                    entry.component_name
                ));
            }

            let child_ids: Vec<String> = routes_decl
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| tanstack_route_var_id(i, &e.path))
                .collect();
            app.push_str("const routeTree = rootRoute.addChildren([");
            app.push_str(&child_ids.join(", "));
            app.push_str("]);\n\n");
            app.push_str("const router = createRouter({ routeTree });\n\n");
            app.push_str("export default function App(): React.ReactElement {\n");
            app.push_str("  return <RouterProvider router={router} />;\n");
            app.push_str("}\n");
            files.push(("App.tsx".to_string(), app));
        }
    }
}
