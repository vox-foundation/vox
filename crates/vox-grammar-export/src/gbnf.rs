pub fn emit_gbnf() -> String {
    let mut grammar = String::new();

    // GBNF avoids left-recursion, so binary_expr is rewritten iteratively
    grammar.push_str("root ::= expr\n");

    // Expressions
    grammar.push_str("expr ::= literal | record_lit | tuple_lit | ident | call_expr | math_expr\n");

    // Literals
    grammar.push_str("literal ::= int_lit | float_lit | string_lit | bool_lit\n");
    grammar.push_str("int_lit ::= \"-\"? [0-9]+\n");
    grammar.push_str("float_lit ::= \"-\"? [0-9]+ \".\" [0-9]+\n");
    grammar.push_str("string_lit ::= \"\\\"\" [^\\\"]* \"\\\"\"\n");
    grammar.push_str("bool_lit ::= \"true\" | \"false\"\n");

    // Records
    grammar.push_str("record_lit ::= \"{\" (field ( \",\" field )*)? \"}\"\n");
    grammar.push_str("field ::= ident \":\" expr\n");

    // Tuples
    grammar.push_str("tuple_lit ::= \"(\" (expr ( \",\" expr )*)? \")\"\n");

    // Call and Math (No Left Recursion)
    grammar.push_str("call_expr ::= ident \"(\" (expr ( \",\" expr )*)? \")\"\n");
    grammar.push_str("math_expr ::= (literal | ident) ws bin_op ws expr\n");
    grammar.push_str("bin_op ::= \"+\" | \"-\" | \"*\" | \"/\" | \"==\" | \"!=\" | \"<\" | \">\" | \"<=\" | \">=\"\n");

    grammar.push_str("ident ::= [a-zA-Z] [a-zA-Z0-9_]*\n");
    grammar.push_str("ws ::= [ \\t\\n]*\n");

    grammar
}
