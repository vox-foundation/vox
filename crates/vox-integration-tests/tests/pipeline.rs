//! End-to-end integration tests for the Vox compiler pipeline.
//! These tests lex → parse → typecheck → codegen the chatbot example
//! and verify the output is correct.
#![allow(unsafe_code)] // `std::env::{set_var,remove_var}` for opt-in Express codegen tests

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use vox_compiler::codegen_ts::{CodegenOptions, generate, generate_with_options};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::typecheck_module;
use vox_lsp::bounded_fs::read_utf8_path_capped;

/// Serializes all tests that read or write `VOX_EMIT_EXPRESS_SERVER`.
/// Without this, parallel test runners observe the env-var mid-mutation,
/// causing `assert!(!server.ts)` tests to see a stale `=1` value.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Sets `VOX_EMIT_EXPRESS_SERVER=1` for the duration of `f`, then restores the prior value.
/// Holds [`ENV_MUTEX`] for the entire call so parallel tests see a stable env.
fn with_express_server_enabled<R>(f: impl FnOnce() -> R) -> R {
    let _env_guard = ENV_MUTEX.lock().expect("ENV_MUTEX poisoned");
    const KEY: &str = "VOX_EMIT_EXPRESS_SERVER";
    struct Guard {
        prev: Option<OsString>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(KEY, v) },
                None => unsafe { std::env::remove_var(KEY) },
            }
        }
    }
    let prev = std::env::var_os(KEY);
    unsafe {
        std::env::set_var(KEY, "1");
    }
    let _guard = Guard { prev };
    f()
}

/// Call `generate()` while holding [`ENV_MUTEX`], ensuring the env-var is NOT set.
/// Prevents `codegen_server_has_express_route_with_await` from racing past "without express" tests.
macro_rules! generate_without_express {
    ($module:expr) => {{
        let _env_guard = ENV_MUTEX.lock().expect("ENV_MUTEX poisoned");
        let hir = vox_compiler::hir::lower_module($module);
        generate(&hir).expect("Should generate without errors")
    }};
}

const CHATBOT_SRC: &str = r#"import react.use_state

type ChatResult =
    | Ok(text: str)
    | Error(message: str)

@component fn Chat() to Element {
    let (messages, set_messages) = use_state([{role: "bot", text: ""}])
    let (input, set_input) = use_state("")
    let send = fn(msg) set_messages(messages.append({role: "user", text: msg}))
    <div class="chat-container">
        <h1>"Vox Chatbot"</h1>
        <button on_click={fn(_e) send(input)}>"Send"</button>
    </div>
}

actor Claude {
    on send(msg: str) to ChatResult {
        Ok("ok")
    }
}

http post "/api/chat" to ChatResult {
    let body = request.json()
    let prompt = body.message
    let response = spawn(Claude).send(prompt)
    ret response
}
"#;

#[test]
fn pipeline_lex_produces_tokens() {
    let tokens = lex(CHATBOT_SRC);
    assert!(
        tokens.len() > 100,
        "Expected many tokens, got {}",
        tokens.len()
    );
}

#[test]
fn pipeline_parse_produces_five_declarations() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).expect("Should parse without errors");
    // import, type, component, actor, http route
    assert_eq!(
        module.declarations.len(),
        5,
        "import + type + component + actor + route"
    );
}

#[test]
fn pipeline_typecheck_has_no_errors() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).unwrap();
    let diagnostics = typecheck_module(&module, "");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == vox_compiler::typeck::diagnostics::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Should have no type errors: {:?}",
        errors
    );
}

#[test]
fn pipeline_codegen_produces_two_ts_files_without_express() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).unwrap();
    let output = generate_without_express!(&module);
    assert_eq!(
        output.files.len(),
        2,
        "types.ts + Chat.tsx (Express server.ts is opt-in via VOX_EMIT_EXPRESS_SERVER)"
    );

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(filenames.contains(&"types.ts"), "Should produce types.ts");
    assert!(filenames.contains(&"Chat.tsx"), "Should produce Chat.tsx");
    assert!(
        !filenames.contains(&"server.ts"),
        "server.ts should not be emitted unless VOX_EMIT_EXPRESS_SERVER=1"
    );
}

#[test]
fn codegen_types_has_tagged_union() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let types = output.files.iter().find(|(n, _)| n == "types.ts").unwrap();
    assert!(types.1.contains("_tag: \"Ok\""), "Should have Ok tag");
    assert!(types.1.contains("_tag: \"Error\""), "Should have Error tag");
    assert!(
        types.1.contains("export type ChatResult"),
        "Should export ChatResult"
    );
}

#[test]
fn codegen_component_has_use_state() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let chat = output.files.iter().find(|(n, _)| n == "Chat.tsx").unwrap();
    assert!(chat.1.contains("useState"), "Should use useState hook");
    assert!(
        chat.1.contains("export function Chat"),
        "Should export Chat component"
    );
}

#[test]
fn codegen_server_has_express_route_with_await() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = with_express_server_enabled(|| generate(&hir).unwrap());

    let server = output.files.iter().find(|(n, _)| n == "server.ts").unwrap();
    assert!(
        server.1.contains("app.post(\"/api/chat\""),
        "Should have POST route"
    );
    assert!(server.1.contains("express"), "Should import express");
    assert!(
        server.1.contains("ClaudeActor"),
        "Should have Claude actor class"
    );
    assert!(
        server.1.contains("await new ClaudeActor().send("),
        "Actor .send() must be awaited"
    );
}

#[test]
fn codegen_jsx_text_content_not_interpolated() {
    let tokens = lex(CHATBOT_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let chat = output.files.iter().find(|(n, _)| n == "Chat.tsx").unwrap();
    // Text content like "Vox" and "Chatbot" inside <h1> should appear as plain text,
    // NOT as {Vox} or {Chatbot} JSX expressions
    assert!(
        !chat.1.contains("{Vox}"),
        "Bare text should not be wrapped in braces"
    );
    assert!(
        !chat.1.contains("{Chatbot}"),
        "Bare text should not be wrapped in braces"
    );
    assert!(
        !chat.1.contains("{Send}"),
        "Button text should not be wrapped in braces"
    );
}

// --- TS codegen for activities ---

#[test]
fn codegen_ts_activity_produces_activities_file() {
    let src = r#"
type MyRes = | Ok(v: str) | Error

activity send_email(recipient: str, subject: str) to Result[str] {
    ret Ok(recipient)
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"activities.ts"),
        "Should produce activities.ts, got: {:?}",
        filenames
    );
}

#[test]
fn codegen_ts_activity_has_async_function() {
    let src = r#"
type MyRes = | Ok(v: str) | Error

activity fetch_data(url: str) to Result[str] {
    ret Ok(url)
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let activities = output
        .files
        .iter()
        .find(|(n, _)| n == "activities.ts")
        .unwrap();
    assert!(
        activities.1.contains("export async function fetch_data("),
        "Should have async function"
    );
    assert!(
        activities.1.contains("url: string"),
        "Should have typed parameter"
    );
    assert!(
        activities.1.contains("Promise<"),
        "Should have Promise return type"
    );
}

#[test]
fn codegen_ts_activity_has_runtime_helper() {
    let src = r#"
type MyRes = | Ok(v: str) | Error

activity do_work() to Result[str] {
    ret Ok("done")
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let activities = output
        .files
        .iter()
        .find(|(n, _)| n == "activities.ts")
        .unwrap();
    assert!(
        activities.1.contains("executeActivity"),
        "Should include executeActivity helper"
    );
    assert!(
        activities.1.contains("ActivityOptions"),
        "Should include ActivityOptions interface"
    );
    assert!(
        activities.1.contains("parseDuration"),
        "Should include parseDuration helper"
    );
}

// --- TS codegen for tables ---

#[test]
fn codegen_ts_table_produces_schema_file() {
    let src = r#"
@table type Task {
    title: str
    done: bool
    priority: int
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"schema.ts"),
        "Should produce schema.ts, got: {:?}",
        filenames
    );

    let schema = output.files.iter().find(|(n, _)| n == "schema.ts").unwrap();
    assert!(
        schema.1.contains("export interface Task {"),
        "Should have Task interface"
    );
    assert!(schema.1.contains("_id: number"), "Should have _id field");
    assert!(
        schema.1.contains("title: string"),
        "Should have title field"
    );
    assert!(schema.1.contains("done: boolean"), "Should have done field");
    assert!(
        schema.1.contains("priority: number"),
        "Should have priority field"
    );
}

// --- @v0 codegen tests ---

#[test]
fn codegen_v0_placeholder_from_prompt() {
    let src = r#"@v0 "A stats dashboard with charts" fn Stats() to Element"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"Stats.tsx"),
        "Should produce Stats.tsx, got: {:?}",
        filenames
    );

    let stats = output.files.iter().find(|(n, _)| n == "Stats.tsx").unwrap();
    assert!(
        stats.1.contains("@v0 generated"),
        "Should contain @v0 marker"
    );
    assert!(
        stats.1.contains("A stats dashboard with charts"),
        "Should contain the prompt"
    );
    assert!(
        stats.1.contains("export function Stats()"),
        "Should export component function"
    );
}

#[test]
fn codegen_v0_placeholder_from_image() {
    let src = r#"@v0 from "design.png" fn Dashboard() to Element"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let dash = output
        .files
        .iter()
        .find(|(n, _)| n == "Dashboard.tsx")
        .unwrap();
    assert!(
        dash.1.contains("From image: design.png"),
        "Should reference the image path"
    );
    assert!(
        dash.1.contains("export function Dashboard()"),
        "Should export component function"
    );
}

// --- @table / @index end-to-end pipeline tests ---

const DATA_LAYER_SRC: &str = r#"@table type Task {
    title: str
    done: bool
    priority: int
}

@index Task.by_done on (done, priority)
"#;

#[test]
fn pipeline_table_parse_produces_declarations() {
    let tokens = lex(DATA_LAYER_SRC);
    let module = parse(tokens).expect("Should parse @table source");
    assert_eq!(module.declarations.len(), 2, "table + index");
}

#[test]
fn pipeline_table_typecheck_no_errors() {
    let tokens = lex(DATA_LAYER_SRC);
    let module = parse(tokens).unwrap();
    let diagnostics = typecheck_module(&module, "");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == vox_compiler::typeck::diagnostics::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Should have no type errors for @table: {:?}",
        errors
    );
}

#[test]
fn pipeline_table_hir_lowering() {
    let tokens = lex(DATA_LAYER_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);

    assert_eq!(hir.tables.len(), 1, "one table");
    assert_eq!(hir.tables[0].name, "Task");
    assert_eq!(hir.tables[0].fields.len(), 3);
    assert_eq!(hir.indexes.len(), 1, "one index");
    assert_eq!(hir.indexes[0].table_name, "Task");
    assert_eq!(hir.indexes[0].index_name, "by_done");
}

#[test]
fn pipeline_table_rust_codegen_e2e() {
    let tokens = lex(DATA_LAYER_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_rust::generate(&hir, "test_data").unwrap();

    let lib_rs = output.files.get("src/lib.rs").expect("lib.rs");
    assert!(lib_rs.contains("pub struct Task {"), "struct emitted");
    assert!(lib_rs.contains("pub _id: Option<i64>,"), "_id field");
    assert!(lib_rs.contains("pub title: String,"), "title field");

    let main_rs = output.files.get("src/main.rs").expect("main.rs");
    assert!(
        main_rs.contains("CREATE TABLE IF NOT EXISTS task"),
        "DDL in main"
    );
    assert!(
        main_rs.contains("CREATE INDEX IF NOT EXISTS idx_task_by_done"),
        "index DDL"
    );
    assert!(
        main_rs.contains("let db = Arc::new(codex)"),
        "Codex should be wrapped in Arc for axum Extension"
    );
}

// --- routes codegen test ---

#[test]
fn codegen_routes_produces_app_tsx() {
    let src = "routes {\n    \"/\" to home\n    \"/about\" to about\n}";
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"App.tsx"),
        "Should produce App.tsx, got: {:?}",
        filenames
    );

    let app = output.files.iter().find(|(n, _)| n == "App.tsx").unwrap();
    assert!(
        app.1.contains("@tanstack/react-router"),
        "Should import @tanstack/react-router"
    );
    assert!(
        app.1.contains("RouterProvider"),
        "Should use RouterProvider"
    );
    assert!(app.1.contains("path: '/'"), "Should have root route path");
    assert!(
        app.1.contains("path: 'about'"),
        "Should have /about as TanStack path segment"
    );
    assert!(
        app.1.contains("import { home }"),
        "Should import home component"
    );
    assert!(
        app.1.contains("import { about }"),
        "Should import about component"
    );
}

#[test]
fn codegen_routes_with_loading_emits_pending_component() {
    let src = r#"@loading fn Spinner() to Element { ret <div>"wait"</div> }

routes {
    "/" to home
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let app = output.files.iter().find(|(n, _)| n == "App.tsx").unwrap();
    assert!(
        app.1.contains("pendingComponent: Spinner"),
        "TanStack createRoute should reference @loading component; got:\n{}",
        app.1
    );
    assert!(
        app.1.contains("Spinner"),
        "Should import Spinner alongside route targets; got:\n{}",
        app.1
    );
    assert!(
        output.files.iter().any(|(n, _)| n == "Spinner.tsx"),
        "Should emit Spinner.tsx"
    );
}

#[test]
fn codegen_tanstack_start_emits_vox_router_without_nested_provider() {
    let src = "routes {\n    \"/\" to home\n    \"/about\" to about\n}";
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate_with_options(
        &hir,
        CodegenOptions {
            tanstack_start: true,
        },
    )
    .unwrap();

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"VoxTanStackRouter.tsx"),
        "Should produce VoxTanStackRouter.tsx, got: {:?}",
        filenames
    );
    assert!(
        !filenames.contains(&"App.tsx"),
        "TanStack Start mode should not emit App.tsx, got: {:?}",
        filenames
    );

    let vox = output
        .files
        .iter()
        .find(|(n, _)| n == "VoxTanStackRouter.tsx")
        .unwrap();
    assert!(
        vox.1.contains("export const voxRouteTree"),
        "Should export voxRouteTree"
    );
    assert!(
        !vox.1.contains("RouterProvider"),
        "Start route module must not embed RouterProvider"
    );
    assert!(
        vox.1.contains("@tanstack/react-router"),
        "Should import TanStack Router"
    );
}

// --- bind={} reactive binding test ---

#[test]
fn codegen_bind_expands_to_value_onchange() {
    let src = r#"@component fn LoginForm() to Element {
    let (email, set_email) = use_state("")
    ret <input bind={email} />
}"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let component = output
        .files
        .iter()
        .find(|(n, _)| n == "LoginForm.tsx")
        .unwrap();
    assert!(
        component.1.contains("value={email}"),
        "bind should expand to value prop, got:\n{}",
        component.1
    );
    assert!(
        component.1.contains("onChange="),
        "bind should expand to onChange handler"
    );
    assert!(
        component.1.contains("set_email"),
        "setter should be derived from ident name (set_email)"
    );
    assert!(
        component.1.contains("e.target.value"),
        "onChange should use e.target.value"
    );
}

// --- use_effect hook mapping test ---

#[test]
fn codegen_use_effect_maps_to_react_hook() {
    let src = r#"@component fn Timer() to Element {
    let (count, set_count) = use_state(0)
    use_effect(fn(_x) count)
    ret <div>{count}</div>
}"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();

    let component = output.files.iter().find(|(n, _)| n == "Timer.tsx").unwrap();
    assert!(
        component.1.contains("useEffect"),
        "use_effect should map to useEffect, got:\n{}",
        component.1
    );
    assert!(
        component.1.contains("import React, {"),
        "Should import from react"
    );
    assert!(
        component.1.contains("useEffect") && component.1.contains("useState"),
        "Both hooks should be in imports"
    );
}

// --- Phase 5F: Full-stack dashboard integration test ---

#[test]
fn dashboard_full_pipeline_e2e() {
    let src = "type Message = | User(text: str) | Bot(text: str)\n\n@v0 \"A metrics dashboard with KPIs\" fn Dashboard() to Element\n\n@component fn ChatWidget() to Element {\n    let (messages, set_messages) = use_state([])\n    let (input, set_input) = use_state(\"\")\n    ret <div class=\"chat\">\n        <input bind={input} />\n        <button on_click={fn(e) set_input(\"\")} >\"Send\"</button>\n    </div>\n}\n\nhttp get \"/api/stats\" to list[int] {\n    ret 42\n}\n\nroutes {\n    \"/\" to Dashboard\n    \"/chat\" to ChatWidget\n}";

    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let output = generate_without_express!(&module);

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();

    // All expected output files
    assert!(
        filenames.contains(&"types.ts"),
        "Should produce types.ts, got: {:?}",
        filenames
    );
    assert!(
        filenames.contains(&"Dashboard.tsx"),
        "Should produce Dashboard.tsx for @v0"
    );
    assert!(
        filenames.contains(&"ChatWidget.tsx"),
        "Should produce ChatWidget.tsx for @component"
    );
    assert!(
        !filenames.contains(&"server.ts"),
        "Express server.ts is opt-in (VOX_EMIT_EXPRESS_SERVER); http routes are served by Axum"
    );
    assert!(
        filenames.contains(&"App.tsx"),
        "Should produce App.tsx for routes:"
    );

    // @v0 placeholder
    let dash = output
        .files
        .iter()
        .find(|(n, _)| n == "Dashboard.tsx")
        .unwrap();
    assert!(
        dash.1.contains("@v0 generated component"),
        "Dashboard should be v0 placeholder"
    );
    assert!(
        dash.1.contains("KPIs"),
        "Dashboard should contain the prompt text"
    );

    // @component with bind={}
    let chat = output
        .files
        .iter()
        .find(|(n, _)| n == "ChatWidget.tsx")
        .unwrap();
    assert!(
        chat.1.contains("value={input}"),
        "bind should expand to value"
    );
    assert!(
        chat.1.contains("onChange="),
        "bind should expand to onChange"
    );
    assert!(
        chat.1.contains("set_input"),
        "bind setter should be set_input"
    );

    // routes -> App.tsx
    let app = output.files.iter().find(|(n, _)| n == "App.tsx").unwrap();
    assert!(app.1.contains("path: '/'"), "App should route /");
    assert!(app.1.contains("path: 'chat'"), "App should route /chat");
    assert!(
        app.1.contains("RouterProvider"),
        "App should use TanStack RouterProvider"
    );

    // types.ts
    let types = output.files.iter().find(|(n, _)| n == "types.ts").unwrap();
    assert!(
        types.1.contains("Message"),
        "types.ts should contain Message type"
    );
}

#[test]
fn chatbot_full_pipeline_e2e() {
    // This test builds the actual examples/chatbot.vox file
    // We assume the test runner is executed from workspace root or crate root
    // But TestProject usually handles tmp dir.
    // We need to read the file content manually if we use lex/parse directly?
    // Or just use the file path if we had a helper.
    // Since previous tests verify logic using inline strings, we'll read the file content here.

    let path = Path::new("fixtures/chatbot.vox");
    let src = read_utf8_path_capped(path)
        .or_else(|_| read_utf8_path_capped(Path::new("tests/fixtures/chatbot.vox")))
        .expect("Could not read chatbot.vox fixture");

    let tokens = vox_compiler::lexer::cursor::lex(&src);
    let module = vox_compiler::parser::parse(tokens).expect("Chatbot should parse");

    let diagnostics = vox_compiler::typeck::typecheck_module(&module, "");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == vox_compiler::typeck::diagnostics::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Chatbot should have no type errors: {:?}",
        errors
    );

    let output = generate_without_express!(&module);

    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();

    assert!(filenames.contains(&"Chat.tsx"), "Should produce Chat.tsx");
    assert!(
        filenames.contains(&"Chat.css"),
        "Should produce Chat.css (from style block)"
    );
    assert!(
        filenames.contains(&"App.tsx"),
        "Should produce App.tsx (from routes)"
    );

    let chat_css = output.files.iter().find(|(n, _)| n == "Chat.css").unwrap();
    assert!(
        chat_css.1.contains(".chat_container"),
        "CSS should contain .chat_container"
    );

    let chat_tsx = output.files.iter().find(|(n, _)| n == "Chat.tsx").unwrap();
    assert!(
        chat_tsx.1.contains("import \"./Chat.css\""),
        "Chat.tsx should import CSS"
    );
    assert!(
        chat_tsx.1.contains("set_messages"),
        "Should use set_messages"
    );
}

// --- TanStack Start scaffold (no Node): keep in sync with `vox-cli` `scaffold_tanstack_start_layout` ---

#[test]
fn tanstack_start_scaffold_programmatic_router_layout() {
    use std::fs;
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
    vox_cli::frontend::scaffold_react_app(&app, &ts_out, true).expect("scaffold");
    assert!(
        app.join("src/routeTree.gen.ts").is_file(),
        "routeTree.gen.ts missing"
    );
    assert!(
        !app.join("src/routes/index.tsx").exists(),
        "programmatic Start must not write routes/index.tsx"
    );
    assert!(app.join("src/router.tsx").is_file());
}

// --- New golden example integration tests (T031–T043) ---

// generics_option.vox
const GENERICS_OPTION_SRC: &str = r#"type Option =
    | Some(value: str)
    | None

type Result =
    | Ok(value: str)
    | Err(message: str)

fn unwrap_or(opt: Option, default: str) to str {
    match opt {
        Some(value) -> value
        None -> default
    }
}

@test fn test_unwrap_some() to Unit {
    let opt = Some("hello")
    let result = unwrap_or(opt, "default")
    assert(result is "hello")
}

@test fn test_ok_result() to Unit {
    let r = Ok("success")
    match r {
        Ok(value) -> assert(value is "success")
        Err(msg) -> assert(false)
    }
}
"#;

#[test]
fn pipeline_generics_option_parse() {
    let tokens = lex(GENERICS_OPTION_SRC);
    let module = parse(tokens).expect("generics_option should parse");
    // type Option + type Result + fn unwrap_or + @test x2
    assert_eq!(module.declarations.len(), 5, "2 types + 1 fn + 2 tests");
}

#[test]
fn pipeline_generics_option_codegen() {
    let tokens = lex(GENERICS_OPTION_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let types = output.files.iter().find(|(n, _)| n == "types.ts").unwrap();
    assert!(
        types.1.contains("export type Option"),
        "Should export Option"
    );
    assert!(types.1.contains("_tag: \"Some\""), "Should have Some tag");
    assert!(types.1.contains("_tag: \"None\""), "Should have None tag");
    assert!(
        types.1.contains("export type Result"),
        "Should export Result"
    );
    assert!(types.1.contains("_tag: \"Ok\""), "Should have Ok tag");
    assert!(types.1.contains("_tag: \"Err\""), "Should have Err tag");
}

// hooks_demo.vox
const HOOKS_DEMO_SRC: &str = r#"import react.use_state
import react.use_effect
import react.use_memo
import react.use_ref
import react.use_callback

@component fn HooksDemo() to Element {
    let (count, set_count) = use_state(0)
    let doubled = use_memo(fn(_x) count * 2)
    let input_ref = use_ref(0)
    let increment = use_callback(fn(_e) set_count(count + 1))
    use_effect(fn(_x) count)
    <div class="hooks_demo">
        <p>"Count: " {count}</p>
        <p>"Doubled: " {doubled}</p>
        <button on_click={increment}>"+"</button>
    </div>
}

routes {
    "/" to HooksDemo
}
"#;

#[test]
fn pipeline_hooks_demo_parse() {
    let tokens = lex(HOOKS_DEMO_SRC);
    let module = parse(tokens).expect("hooks_demo should parse");
    // 5 imports + 1 component + 1 routes
    assert_eq!(module.declarations.len(), 7);
}

#[test]
fn pipeline_hooks_demo_codegen() {
    let tokens = lex(HOOKS_DEMO_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let tsx = output
        .files
        .iter()
        .find(|(n, _)| n == "HooksDemo.tsx")
        .unwrap();
    assert!(tsx.1.contains("useState"), "Should use useState");
    assert!(tsx.1.contains("useEffect"), "Should use useEffect");
    assert!(tsx.1.contains("useMemo"), "Should use useMemo");
    assert!(tsx.1.contains("useRef"), "Should use useRef");
    assert!(tsx.1.contains("useCallback"), "Should use useCallback");
    assert!(tsx.1.contains("import React,"), "Should have React import");
}

// island_demo.vox
const ISLAND_DEMO_SRC: &str = r#"@island InteractiveChart {
    data: str
    title: str
    width?: int
}

@island SearchWidget {
    placeholder: str
    endpoint: str
}

@component fn IslandHost() to Element {
    <div class="island_host">
        <h1>"Islands Demo"</h1>
    </div>
}

routes {
    "/" to IslandHost
}
"#;

#[test]
fn pipeline_island_parse() {
    let tokens = lex(ISLAND_DEMO_SRC);
    let module = parse(tokens).expect("island_demo should parse");
    // 2 islands + 1 component + 1 routes
    assert_eq!(module.declarations.len(), 4);
}

#[test]
fn pipeline_island_codegen() {
    let tokens = lex(ISLAND_DEMO_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"vox-islands-meta.ts"),
        "Should produce vox-islands-meta.ts, got: {:?}",
        filenames
    );
    let meta = output
        .files
        .iter()
        .find(|(n, _)| n == "vox-islands-meta.ts")
        .unwrap();
    assert!(
        meta.1.contains("InteractiveChart"),
        "Island meta should list InteractiveChart"
    );
    assert!(
        meta.1.contains("SearchWidget"),
        "Island meta should list SearchWidget"
    );
}

// v0_component.vox
const V0_COMPONENT_SRC: &str = r#"@v0 "A modern analytics dashboard with KPI cards" fn Analytics() to Element

@v0 from "design/landing-mockup.png" fn LandingPage() to Element

routes {
    "/" to Analytics
    "/landing" to LandingPage
}
"#;

#[test]
fn pipeline_v0_parse() {
    let tokens = lex(V0_COMPONENT_SRC);
    let module = parse(tokens).expect("v0_component should parse");
    // 2 v0 + 1 routes
    assert_eq!(module.declarations.len(), 3);
}

#[test]
fn pipeline_v0_codegen() {
    let tokens = lex(V0_COMPONENT_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"Analytics.tsx"),
        "Should produce Analytics.tsx"
    );
    assert!(
        filenames.contains(&"LandingPage.tsx"),
        "Should produce LandingPage.tsx"
    );
    let analytics = output
        .files
        .iter()
        .find(|(n, _)| n == "Analytics.tsx")
        .unwrap();
    assert!(
        analytics.1.contains("@v0 generated"),
        "Analytics should be v0 placeholder"
    );
    let landing = output
        .files
        .iter()
        .find(|(n, _)| n == "LandingPage.tsx")
        .unwrap();
    assert!(
        landing.1.contains("landing-mockup.png"),
        "LandingPage should reference the image"
    );
}

// mcp_tool_demo.vox
const MCP_TOOL_SRC: &str = r#"type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base" fn search_knowledge(query: str) to SearchResult {
    Found("Result for: " + query, 95)
}

@mcp.tool "Get system status" fn system_status() to str {
    ret "healthy"
}
"#;

#[test]
fn pipeline_mcp_tool_parse() {
    let tokens = lex(MCP_TOOL_SRC);
    let module = parse(tokens).expect("mcp_tool_demo should parse");
    // 1 type + 2 mcp.tool
    assert_eq!(module.declarations.len(), 3);
    assert!(
        matches!(&module.declarations[1], vox_compiler::ast::decl::Decl::McpTool(m) if m.description == "Search the knowledge base"),
        "First tool should have correct description"
    );
}

// pattern_matching.vox
const PATTERN_MATCHING_SRC: &str = r#"type Shape =
    | Circle(radius: int)
    | Rectangle(width: int, height: int)
    | Triangle(base: int, height: int)

fn area(s: Shape) to int {
    match s {
        Circle(radius) -> radius * radius
        Rectangle(width, height) -> width * height
        Triangle(base, height) -> base * height
    }
}

@test fn test_circle_area() to Unit {
    let c = Circle(5)
    let a = area(c)
    assert(a is 25)
}

@test fn test_rectangle_area() to Unit {
    let r = Rectangle(4, 6)
    let a = area(r)
    assert(a is 24)
}
"#;

#[test]
fn pipeline_pattern_matching_parse() {
    let tokens = lex(PATTERN_MATCHING_SRC);
    let module = parse(tokens).expect("pattern_matching should parse");
    // 1 type + 1 fn + 2 tests
    assert_eq!(module.declarations.len(), 4);
}

#[test]
fn pipeline_pattern_matching_codegen() {
    let tokens = lex(PATTERN_MATCHING_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let types = output.files.iter().find(|(n, _)| n == "types.ts").unwrap();
    assert!(
        types.1.contains("export type Shape"),
        "Should export Shape type"
    );
    assert!(types.1.contains("_tag: \"Circle\""), "Should have Circle");
    assert!(
        types.1.contains("_tag: \"Rectangle\""),
        "Should have Rectangle"
    );
    assert!(
        types.1.contains("_tag: \"Triangle\""),
        "Should have Triangle"
    );
}

// multi_route_app.vox
const MULTI_ROUTE_SRC: &str = r#"import react.use_state

type Todo =
    | Active(text: str)
    | Completed(text: str)

@component fn Dashboard() to Element {
    let (visits, set_visits) = use_state(0)
    <div class="dashboard">
        <h1>"Dashboard"</h1>
        <button on_click={fn(_e) set_visits(visits + 1)}>"Track"</button>
    </div>
}

@component fn TodoList() to Element {
    let (items, set_items) = use_state([])
    let (draft, set_draft) = use_state("")
    <div class="todo_list">
        <input bind={draft} placeholder="New task..." />
        <button on_click={fn(_e) set_items(items.append({text: draft}))}>"Add"</button>
    </div>
}

@component fn About() to Element {
    <div class="about">
        <h1>"About"</h1>
    </div>
}

routes {
    "/" to Dashboard
    "/todos" to TodoList
    "/about" to About
}

http get "/api/todos" to str {
    ret "[]"
}

http post "/api/todos" to str {
    ret "created"
}

@server fn get_stats() to int {
    ret 42
}
"#;

#[test]
fn pipeline_multi_route_parse() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).expect("multi_route_app should parse");
    // 1 import + 1 type + 3 components + 1 routes + 2 http + 1 server fn = 9
    assert_eq!(module.declarations.len(), 9);
}

#[test]
fn pipeline_multi_route_codegen() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"Dashboard.tsx"),
        "Should produce Dashboard.tsx"
    );
    assert!(
        filenames.contains(&"TodoList.tsx"),
        "Should produce TodoList.tsx"
    );
    assert!(filenames.contains(&"About.tsx"), "Should produce About.tsx");
    assert!(
        filenames.contains(&"App.tsx"),
        "Should produce App.tsx for routes:"
    );
    assert!(
        filenames.contains(&"types.ts"),
        "Should produce types.ts for Todo"
    );
}

#[test]
fn pipeline_multi_route_rust_codegen() {
    let tokens = lex(MULTI_ROUTE_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_rust::generate(&hir, "multi_route_app").unwrap();
    let main_rs = output.files.get("src/main.rs").expect("main.rs");
    // Axum uses .route("/path", get(handler)) syntax
    assert!(
        main_rs.contains("\"/api/todos\""),
        "Should have /api/todos route in main.rs, got:\n{}",
        &main_rs[..main_rs.len().min(2000)]
    );
    assert!(
        main_rs.contains("handle_sf_get_stats"),
        "Should have server fn handler for get_stats"
    );
}
