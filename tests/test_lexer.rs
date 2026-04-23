use vox_compiler::lexer::lex;
fn main() {
    let source = "@mcp.tool";
    let tokens = lex(source);
    println!("{:#?}", tokens);
}
