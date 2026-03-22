//! `@island` declaration parsing.

use vox_ast::decl::{Decl, IslandDecl};
use vox_lexer::lex;
use vox_parser::parse;

#[test]
fn parse_island_with_props() {
    let src = r#"
@island TodoCard {
  title: str
  done?: bool
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).expect("parse");
    assert_eq!(module.declarations.len(), 1);
    let Decl::Island(IslandDecl { name, props, .. }) = &module.declarations[0] else {
        panic!("expected Island decl");
    };
    assert_eq!(name, "TodoCard");
    assert_eq!(props.len(), 2);
    assert_eq!(props[0].name, "title");
    assert!(!props[0].is_optional);
    assert_eq!(props[1].name, "done");
    assert!(props[1].is_optional);
}
