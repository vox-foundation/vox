#![allow(missing_docs)]

use vox_compiler::codegen_rust::emit::emit_lib;
use vox_compiler::hir::lower_module;
/// Integration tests for Rust code generation of durable execution features.
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

fn codegen_rust(src: &str) -> String {
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    emit_lib(&hir)
}

/// `activity` keyword is tombstoned (TASK-2.6); parsing source that uses it must fail.
#[test]
fn codegen_activity_emits_async_fn() {
    let src = r#"
activity send_email(recipient: str, subject: str) to Result[str] {
    return Ok(recipient)
}
"#;
    assert!(
        parse(lex(src)).is_err(),
        "tombstoned `activity` keyword should produce a parse error"
    );
}

/// `activity` + `workflow` keywords are both tombstoned (TASK-2.6).
#[test]
fn codegen_with_expression_emits_execute_activity() {
    let activity_src = r#"activity fetch_data() to Result[str] { return Ok("data") }"#;
    let workflow_src = r#"workflow main_flow() to Result[str] { return Ok("done") }"#;
    assert!(
        parse(lex(activity_src)).is_err(),
        "tombstoned `activity` keyword should produce a parse error"
    );
    assert!(
        parse(lex(workflow_src)).is_err(),
        "tombstoned `workflow` keyword should produce a parse error"
    );
}

/// `activity` keyword is tombstoned (TASK-2.6); plain `fn` is the canonical form.
#[test]
fn codegen_activity_without_with_is_plain_call() {
    let tombstoned_src = r#"activity do_work(input: str) to Result[str] { return Ok(input) }"#;
    assert!(
        parse(lex(tombstoned_src)).is_err(),
        "tombstoned `activity` keyword should produce a parse error"
    );

    // The canonical equivalent compiles and codegens without error.
    let canonical_src = r#"
fn do_work(input: str) to str {
    return input
}
fn main() to str {
    let result = do_work("test")
    return result
}
"#;
    let output = codegen_rust(canonical_src);
    assert!(
        output.contains("fn do_work("),
        "canonical fn form should be emitted"
    );
}

#[test]
fn codegen_with_all_options() {
    let src = r#"
fn f() to int {
    let x = 1 with { retries: 5, timeout: "30s", activity_id: "unique-xyz", backoff_multiplier: 2 }
    return x
}
"#;
    let output = codegen_rust(src);
    assert!(output.contains("with_retries(5"), "Should emit retries");
    assert!(
        output.contains("parse_duration(\"30s\")"),
        "Should emit timeout"
    );
    assert!(
        output.contains("with_activity_id(\"unique-xyz\""),
        "Should emit activity_id"
    );
    assert!(
        output.contains("with_backoff_multiplier(2"),
        "Should emit backoff_multiplier"
    );
}

// --- Table and Index codegen tests ---

#[test]
fn codegen_table_emits_struct() {
    let src = r#"
@table type Task {
    title: str
    done: bool
    priority: int
}
"#;
    let output = codegen_rust(src);
    assert!(
        output.contains("pub struct Task"),
        "Should emit struct for @table"
    );
    assert!(
        output.contains("pub _id: Option<i64>"),
        "Should have auto _id field"
    );
    assert!(
        output.contains("pub title: String"),
        "Should have title field"
    );
    assert!(output.contains("pub done: bool"), "Should have done field");
    assert!(
        output.contains("pub priority: i64"),
        "Should have priority field"
    );
}

#[test]
fn codegen_table_emits_ddl() {
    use vox_compiler::codegen_rust::emit::emit_table_ddl;

    let src = r#"
@table type Task {
    title: str
    done: bool
    priority: int
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    assert_eq!(hir.tables.len(), 1, "Should have 1 table");

    let ddl = emit_table_ddl(&hir.tables[0]);
    assert!(
        ddl.contains("CREATE TABLE IF NOT EXISTS task"),
        "DDL should create table 'task'"
    );
    assert!(
        ddl.contains("_id INTEGER PRIMARY KEY AUTOINCREMENT"),
        "DDL should have _id PK"
    );
    assert!(
        ddl.contains("title TEXT NOT NULL"),
        "DDL should have title column"
    );
    assert!(
        ddl.contains("done INTEGER NOT NULL"),
        "DDL: bool maps to INTEGER"
    );
    assert!(
        ddl.contains("priority INTEGER NOT NULL"),
        "DDL: int maps to INTEGER"
    );
}

#[test]
fn codegen_index_emits_ddl() {
    use vox_compiler::codegen_rust::emit::emit_index_ddl;

    let src = r#"
@table type Task {
    title: str
    done: bool
    priority: int
}

@index Task.by_done_priority on (done, priority)
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    assert_eq!(hir.indexes.len(), 1, "Should have 1 index");

    let ddl = emit_index_ddl(&hir.indexes[0]);
    assert!(
        ddl.contains("CREATE INDEX IF NOT EXISTS idx_task_by_done_priority"),
        "DDL should create index"
    );
    assert!(ddl.contains("ON task"), "DDL should reference table");
    assert!(ddl.contains("(done, priority)"), "DDL should list columns");
}

// --- MCP server codegen tests ---

#[test]
fn codegen_mcp_tool_hir_lowering() {
    let src = r#"
@mcp.tool "Get the weather for a city" fn get_weather(city: str) to str {
    return city
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);

    assert_eq!(hir.mcp_tools.len(), 1, "Should have 1 MCP tool");
    assert!(
        hir.functions.is_empty(),
        "MCP tools should NOT also appear in functions list"
    );
    assert_eq!(hir.mcp_tools[0].description, "Get the weather for a city");
    assert_eq!(hir.mcp_tools[0].func.name, "get_weather");
    assert_eq!(hir.mcp_tools[0].func.params.len(), 1);
}

#[test]
fn codegen_mcp_server_produces_file() {
    let src = r#"
@mcp.tool "Get the weather for a city" fn get_weather(city: str) to str {
    return city
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    let output = vox_compiler::codegen_rust::generate(&hir, "my_mcp_tools").unwrap();

    assert!(
        output.files.contains_key("src/mcp_server.rs"),
        "Should produce mcp_server.rs"
    );

    let mcp = output.files.get("src/mcp_server.rs").unwrap();
    assert!(
        mcp.contains("fn dispatch_tool"),
        "Should have dispatch function"
    );
    assert!(
        mcp.contains("fn tool_list"),
        "Should have tool_list function"
    );
    assert!(mcp.contains("fn main"), "Should have main entry point");
    assert!(
        mcp.contains("\"get_weather\""),
        "Should reference tool name"
    );
    assert!(
        mcp.contains("Get the weather for a city"),
        "Should include description"
    );
    assert!(mcp.contains("\"initialize\""), "Should handle initialize");
    assert!(mcp.contains("\"tools/list\""), "Should handle tools/list");
    assert!(mcp.contains("\"tools/call\""), "Should handle tools/call");
}

#[test]
fn codegen_mcp_server_input_schema() {
    let src = r#"
@mcp.tool "Add two numbers" fn add(a: int, b: int) to int {
    return a
}

@mcp.tool "Greet someone" fn greet(name: str) to str {
    return name
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);

    assert_eq!(hir.mcp_tools.len(), 2, "Should have 2 MCP tools");

    let mcp = vox_compiler::codegen_rust::emit::emit_mcp_server(&hir, "my_tools");
    assert!(
        mcp.contains("\"integer\""),
        "int params should map to JSON 'integer' type"
    );
    assert!(
        mcp.contains("\"string\""),
        "str params should map to JSON 'string' type"
    );
    assert!(
        mcp.contains("as_i64"),
        "int params should use as_i64 for extraction"
    );
    assert!(
        mcp.contains("as_str"),
        "str params should use as_str for extraction"
    );
}

#[test]
fn codegen_no_mcp_server_when_no_tools() {
    let src = r#"
fn hello(name: str) to str {
    return name
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    let output = vox_compiler::codegen_rust::generate(&hir, "test_no_mcp").unwrap();

    assert!(
        !output.files.contains_key("src/mcp_server.rs"),
        "Should NOT produce mcp_server.rs when no @mcp.tool / @mcp.resource"
    );
}

#[test]
fn codegen_mcp_resource_emits_resources_handlers() {
    let src = r#"
@mcp.resource("demo://x", "A demo resource") fn demo_res() to str {
    return "ok"
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    assert_eq!(hir.mcp_resources.len(), 1);
    assert_eq!(hir.mcp_resources[0].uri, "demo://x");

    let output = vox_compiler::codegen_rust::generate(&hir, "with_res").unwrap();
    assert!(output.files.contains_key("src/mcp_server.rs"));
    let mcp = output.files.get("src/mcp_server.rs").unwrap();
    assert!(mcp.contains("resources/list"), "expected resources/list");
    assert!(mcp.contains("resources/read"), "expected resources/read");
    assert!(
        mcp.contains("dispatch_resource"),
        "expected resource dispatch"
    );
    assert!(mcp.contains("demo://x"), "expected URI in dispatch");

    let cargo = output.files.get("Cargo.toml").expect("Cargo.toml");
    assert!(
        cargo.contains("[[bin]]") && cargo.contains("mcp_server"),
        "MCP binary should be declared in Cargo.toml:\n{cargo}"
    );
}

#[test]
fn codegen_mcp_tool_list_schema_supports_list_param() {
    let src = r#"
@mcp.tool "Echo ids" fn echo_ids(items: list[str]) to str {
    return "ok"
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("Should parse");
    let hir = lower_module(&module);
    let mcp = vox_compiler::codegen_rust::emit::emit_mcp_server(&hir, "lst");
    assert!(
        mcp.contains("\"type\": \"array\"") && mcp.contains("\"items\""),
        "list[str] should emit array schema:\n{mcp}"
    );
}
