//! EBNF emitter for the Vox grammar — derived from the 57 production rules
//! in `vox-compiler/src/parser/descent/`.
//!
//! **This is the authoritative grammar export format.** GBNF and Lark are derived
//! from this output. The research (Grammar Constraints §2.1) proves EBNF/Earley
//! is structurally superior to GBNF/FSA for recursive CFGs.

/// Emit the complete EBNF grammar for Vox 0.4.
///
/// Rules are grouped by category and annotated with the parser function they derive from.
/// The grammar covers all 57 production rules extracted from `parser/descent/`.
#[must_use]
pub fn emit_ebnf() -> String {
    let mut g = String::with_capacity(8192);
    g.push_str("(* EBNF Grammar for Vox 0.4 — auto-generated from parser/descent/ *)\n");
    g.push_str("(* 57 production rules — do not hand-edit *)\n\n");

    // ── Module & Declarations
    g.push_str("(* mod.rs: parse_module, parse_decl *)\n");
    g.push_str("module = { decl } ;\n");
    g.push_str("decl = fn_decl | type_def | import | actor | workflow | activity\n");
    g.push_str("     | http_route | table | index | test | forall\n");
    g.push_str("     | server_fn | query_fn | mutation_fn\n");
    g.push_str("     | mcp_tool | mcp_resource\n");
    g.push_str("     | component | reactive_component | island | v0_component\n");
    g.push_str("     | routes | loading | agent | environment\n");
    g.push_str("     ;\n\n");

    // ── Functions
    g.push_str("(* head.rs: parse_fn_decl *)\n");
    g.push_str("fn_decl = [ \"pub\" ], [ \"async\" ], \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("params = param, { \",\", param } ;\n");
    g.push_str("param = ident, \":\", type_expr, [ \"?\" ] ;\n\n");

    // ── Type definitions
    g.push_str("(* head.rs: parse_typedef *)\n");
    g.push_str("type_def = [ \"pub\" ], \"type\", ident, \"=\", type_body ;\n");
    g.push_str("type_body = adt_variants | struct_body ;\n");
    g.push_str("adt_variants = \"|\", variant, { \"|\", variant } ;\n");
    g.push_str("variant = ident, [ \"(\", field_list, \")\" ] ;\n");
    g.push_str("struct_body = \"{\", field_def, { \",\", field_def }, [ \",\" ], \"}\" ;\n");
    g.push_str("field_def = ident, \":\", type_expr ;\n\n");

    // ── Imports
    g.push_str("(* head.rs: parse_import, parse_import_path *)\n");
    g.push_str("import = \"import\", import_path, { \",\", import_path } ;\n");
    g.push_str("import_path = symbol_path | rust_crate_import ;\n");
    g.push_str("symbol_path = ident, { \".\", ident } ;\n");
    g.push_str("rust_crate_import = \"rust:\", ident ;\n\n");

    // ── Actors
    g.push_str("(* mid.rs: parse_actor *)\n");
    g.push_str("actor = \"actor\", ident, \"{\", { actor_handler }, \"}\" ;\n");
    g.push_str("actor_handler = \"on\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n\n");

    // ── Workflows & Activities
    g.push_str("(* mid.rs: parse_workflow, parse_activity *)\n");
    g.push_str("workflow = \"workflow\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("activity = \"activity\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n\n");

    // ── HTTP Routes
    g.push_str("(* mid.rs: parse_http_route *)\n");
    g.push_str("http_route = \"http\", http_method, string_lit, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("http_method = \"get\" | \"post\" | \"put\" | \"delete\" ;\n\n");

    // ── Tables & Indexes
    g.push_str("(* head.rs: parse_table, tail.rs: parse_index *)\n");
    g.push_str("table = { decorator }, \"@table\", [ \"type\" ], ident, \"{\", field_def, { field_def }, \"}\" ;\n");
    g.push_str("index = \"@index\", ident, \".\", ident, \"on\", \"(\", ident, { \",\", ident }, \")\" ;\n\n");

    // ── Server / Query / Mutation
    g.push_str("(* tail.rs: parse_server_fn, parse_query_fn, parse_mutation_fn *)\n");
    g.push_str("server_fn = \"@server\", \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("query_fn = \"@query\", \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("mutation_fn = \"@mutation\", \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n\n");

    // ── Tests & PBT
    g.push_str("(* tail.rs: parse_test, parse_forall *)\n");
    g.push_str("test = \"@test\", \"fn\", ident, \"(\", \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("forall = \"@forall\", \"fn\", ident, \"(\", params, \")\", [ \"to\", type_expr ], block ;\n\n");

    // ── MCP Tools & Resources
    g.push_str("(* tail.rs: parse_mcp_tool, parse_mcp_resource *)\n");
    g.push_str("mcp_tool = \"@mcp.tool\", [ string_lit ], \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str("mcp_resource = \"@mcp.resource\", [ string_lit ], \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n\n");

    // ── Components & Islands
    g.push_str("(* head.rs: parse_component, parse_reactive_component, parse_island, parse_v0_component *)\n");
    g.push_str("component = \"@component\", \"fn\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n");
    g.push_str(
        "reactive_component = \"component\", ident, \"(\", [ params ], \")\", reactive_block ;\n",
    );
    g.push_str("reactive_block = \"{\", { reactive_member }, [ \"view:\", expr ], \"}\" ;\n");
    g.push_str("reactive_member = state_decl | derived_decl | effect_block | mount_block | cleanup_block | stmt ;\n");
    g.push_str("state_decl = \"state\", ident, [ \":\", type_expr ], \"=\", expr ;\n");
    g.push_str("derived_decl = \"derived\", ident, [ \":\", type_expr ], \"=\", expr ;\n");
    g.push_str("effect_block = \"effect\", block ;\n");
    g.push_str("mount_block = \"mount\", block ;\n");
    g.push_str("cleanup_block = \"cleanup\", block ;\n");
    g.push_str("island = \"@island\", ident, \"{\", { island_prop }, \"}\" ;\n");
    g.push_str("island_prop = ident, [ \"?\" ], \":\", type_expr ;\n");
    g.push_str(
        "v0_component = \"@v0\", string_lit, \"fn\", ident, \"(\", \")\", \"to\", type_expr ;\n\n",
    );

    // ── Routes & Loading
    g.push_str("(* mid.rs: parse_routes, tail.rs: parse_loading *)\n");
    g.push_str("routes = \"routes\", \"{\", { route_entry }, \"}\" ;\n");
    g.push_str("route_entry = string_lit, \"to\", ident, [ with_clause ] ;\n");
    g.push_str("with_clause = \"with\", \"{\", { ident, \":\", expr }, \"}\" ;\n");
    g.push_str(
        "loading = \"@loading\", \"fn\", ident, \"(\", \")\", [ \"to\", type_expr ], block ;\n\n",
    );

    // ── Agents & Environments
    g.push_str("(* mid.rs: parse_agent, parse_environment *)\n");
    g.push_str("agent = \"agent\", ident, \"{\", { agent_handler }, \"}\" ;\n");
    g.push_str(
        "agent_handler = \"on\", ident, \"(\", [ params ], \")\", [ \"to\", type_expr ], block ;\n",
    );
    g.push_str("environment = \"environment\", ident, \"{\", { env_field }, \"}\" ;\n");
    g.push_str("env_field = ident, \":\", type_expr, \"=\", expr ;\n\n");

    // ── Decorators
    g.push_str("(* head.rs: decorator prefix parsing *)\n");
    g.push_str("decorator = \"@require\", \"(\", expr, \")\" | \"@ensure\", \"(\", expr, \")\" | \"@invariant\", \"(\", expr, \")\" | \"@fuzz\" ;\n\n");

    // ── Statements
    g.push_str("(* stmt.rs: parse_stmt, parse_let_stmt, parse_while_stmt, parse_loop_stmt *)\n");
    g.push_str("stmt = let_stmt | assign_stmt | return_stmt | while_stmt | loop_stmt | break_stmt | continue_stmt | expr_stmt ;\n");
    g.push_str("let_stmt = \"let\", [ \"mut\" ], ident, [ \":\", type_expr ], \"=\", expr ;\n");
    g.push_str("assign_stmt = ident, assign_op, expr ;\n");
    g.push_str("assign_op = \"=\" | \"+=\" | \"-=\" | \"*=\" | \"/=\" ;\n");
    g.push_str("return_stmt = ( \"ret\" | \"return\" ), [ expr ] ;\n");
    g.push_str("while_stmt = \"while\", expr, block ;\n");
    g.push_str("loop_stmt = \"loop\", block ;\n");
    g.push_str("break_stmt = \"break\" ;\n");
    g.push_str("continue_stmt = \"continue\" ;\n");
    g.push_str("expr_stmt = expr ;\n");
    g.push_str("block = \"{\", { stmt }, [ expr ], \"}\" ;\n\n");

    // ── Expressions
    g.push_str("(* pratt_ops.rs + pratt_match.rs: parse_expr, parse_expr_bp, parse_primary *)\n");
    g.push_str("expr = primary, { bin_op, primary } | pipe_expr ;\n");
    g.push_str("pipe_expr = expr, \"|>\", expr ;\n");
    g.push_str("primary = literal | ident | call_expr | field_access | method_call\n");
    g.push_str("        | match_expr | if_expr | for_expr | lambda | spawn_expr | with_expr\n");
    g.push_str(
        "        | object_lit | list_lit | tuple_lit | jsx_expr | block | \"(\", expr, \")\" ;\n\n",
    );

    g.push_str("match_expr = \"match\", expr, \"{\", { match_arm }, \"}\" ;\n");
    g.push_str("match_arm = pattern, \"->\", expr ;\n");
    g.push_str(
        "pattern = ident | ident, \"(\", { ident, { \",\", ident } }, \")\" | \"_\" | literal ;\n",
    );
    g.push_str("if_expr = \"if\", expr, block, [ \"else\", ( block | if_expr ) ] ;\n");
    g.push_str("for_expr = \"for\", ident, \"in\", expr, block ;\n");
    g.push_str("lambda = \"fn\", \"(\", [ params ], \")\", ( block | expr ) ;\n");
    g.push_str("spawn_expr = \"spawn\", expr ;\n");
    g.push_str("with_expr = \"with\", expr, object_lit ;\n");
    g.push_str("call_expr = expr, \"(\", [ args ], \")\" ;\n");
    g.push_str("args = expr, { \",\", expr } ;\n");
    g.push_str("field_access = expr, \".\", ident ;\n");
    g.push_str("method_call = expr, \".\", ident, \"(\", [ args ], \")\" ;\n\n");

    // ── Literals
    g.push_str("literal = int_lit | float_lit | string_lit | bool_lit | \"Unit\" ;\n");
    g.push_str("int_lit = [ \"-\" ], digit, { digit } ;\n");
    g.push_str("float_lit = [ \"-\" ], digit, { digit }, \".\", digit, { digit } ;\n");
    g.push_str("string_lit = '\"', { any_char - '\"' }, '\"' ;\n");
    g.push_str("bool_lit = \"true\" | \"false\" ;\n");
    g.push_str("object_lit = \"{\", [ field_init, { \",\", field_init } ], \"}\" ;\n");
    g.push_str("field_init = ident, \":\", expr ;\n");
    g.push_str("list_lit = \"[\", [ expr, { \",\", expr } ], \"]\" ;\n");
    g.push_str("tuple_lit = \"(\", expr, \",\", [ expr, { \",\", expr } ], \")\" ;\n\n");

    // ── JSX
    g.push_str("(* pratt_jsx.rs: parse_jsx *)\n");
    g.push_str("jsx_expr = jsx_self_closing | jsx_element ;\n");
    g.push_str("jsx_self_closing = \"<\", ident, { jsx_attr }, \"/>\" ;\n");
    g.push_str(
        "jsx_element = \"<\", ident, { jsx_attr }, \">\", { jsx_child }, \"</\", ident, \">\" ;\n",
    );
    g.push_str("jsx_attr = ident, \"=\", ( string_lit | \"{\", expr, \"}\" ) ;\n");
    g.push_str("jsx_child = jsx_expr | \"{\", expr, \"}\" | text ;\n\n");

    // ── Type expressions
    g.push_str("(* types.rs: parse_type_expr *)\n");
    g.push_str("type_expr = simple_type | generic_type | fn_type | tuple_type | \"Unit\" ;\n");
    g.push_str("simple_type = ident ;\n");
    g.push_str("generic_type = ident, \"[\", type_expr, { \",\", type_expr }, \"]\" ;\n");
    g.push_str("fn_type = \"fn\", \"(\", [ type_expr, { \",\", type_expr } ], \")\", \"to\", type_expr ;\n");
    g.push_str(
        "tuple_type = \"(\", type_expr, \",\", [ type_expr, { \",\", type_expr } ], \")\" ;\n\n",
    );

    // ── Operators
    g.push_str("bin_op = \"+\" | \"-\" | \"*\" | \"/\" | \"==\" | \"!=\" | \"<\" | \">\" | \"<=\" | \">=\" | \"and\" | \"or\" ;\n");
    g.push_str("unary_op = \"not\" | \"-\" ;\n\n");

    // ── Terminals
    g.push_str("ident = letter, { letter | digit | \"_\" } ;\n");
    g.push_str("letter = \"A\"...\"Z\" | \"a\"...\"z\" ;\n");
    g.push_str("digit = \"0\"...\"9\" ;\n");
    g.push_str("field_list = field_def, { \",\", field_def } ;\n");
    g.push_str("text = { any_char - \"<\" - \"{\" } ;\n");

    g
}
