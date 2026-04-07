use crate::lexer::token::Token;
use logos::Logos;

/// A located token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub token: Token,
    pub span: std::ops::Range<usize>,
}

/// Lex source code into a flat vector of spanned tokens.
///
/// Block structure is delimited by `{` / `}` tokens — **no** synthetic
/// `Indent`/`Dedent` tokens are emitted. Comments are stripped. A final
/// [`Token::Eof`] sentinel is always appended.
pub fn lex(source: &str) -> Vec<Spanned> {
    let mut result: Vec<Spanned> = Token::lexer(source)
        .spanned()
        .filter_map(|(result, span)| match result {
            Ok(token) => {
                if matches!(token, Token::Comment) {
                    None // strip comments
                } else {
                    Some(Spanned { token, span })
                }
            }
            Err(_) => None, // skip unrecognized characters
        })
        .collect();

    let eof_pos = source.len();
    result.push(Spanned {
        token: Token::Eof,
        span: eof_pos..eof_pos,
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::Token;

    fn lex_tokens(source: &str) -> Vec<Token> {
        lex(source).into_iter().map(|s| s.token).collect()
    }

    #[test]
    fn test_simple_let_binding() {
        let tokens = lex_tokens("let x = 5");
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Ident("x".into()),
                Token::Eq,
                Token::IntLit(5),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = lex_tokens("fn let mut if else match for in to ret type import");
        let expected = vec![
            Token::Fn,
            Token::Let,
            Token::Mut,
            Token::If,
            Token::Else,
            Token::Match,
            Token::For,
            Token::In,
            Token::To,
            Token::Ret,
            Token::TypeKw,
            Token::Import,
            Token::Eof,
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_phonetic_operators() {
        let tokens = lex_tokens("and or not is isnt true false");
        let expected = vec![
            Token::And,
            Token::Or,
            Token::Not,
            Token::Is,
            Token::Isnt,
            Token::True,
            Token::False,
            Token::Eof,
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_string_literals() {
        let tokens = lex_tokens(r#""hello world""#);
        assert_eq!(
            tokens,
            vec![Token::StringLit("hello world".into()), Token::Eof]
        );
    }

    #[test]
    fn test_numeric_literals() {
        let tokens = lex_tokens("42 2.75");
        assert_eq!(
            tokens,
            vec![Token::IntLit(42), Token::FloatLit(2.75), Token::Eof]
        );
    }

    #[test]
    fn test_identifiers() {
        let tokens = lex_tokens("foo bar_baz MyType Result");
        assert_eq!(
            tokens,
            vec![
                Token::Ident("foo".into()),
                Token::Ident("bar_baz".into()),
                Token::TypeIdent("MyType".into()),
                Token::TypeIdent("Result".into()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_decorators() {
        let tokens = lex_tokens("@component @mcp.tool @mcp.resource @mobile.native @island");
        assert_eq!(
            tokens,
            vec![
                Token::AtComponent,
                Token::AtMcpTool,
                Token::AtMcpResource,
                Token::AtMobileNative,
                Token::AtIsland,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_symbols() {
        let tokens = lex_tokens("( ) [ ] { } : ? , . = -> |> | < >");
        let expected = vec![
            Token::LParen,
            Token::RParen,
            Token::LBracket,
            Token::RBracket,
            Token::LBrace,
            Token::RBrace,
            Token::Colon,
            Token::Question,
            Token::Comma,
            Token::Dot,
            Token::Eq,
            Token::Arrow,
            Token::PipeOp,
            Token::Bar,
            Token::Lt,
            Token::Gt,
            Token::Eof,
        ];
        assert_eq!(tokens, expected);
    }

    /// Brace syntax: no Indent/Dedent in output — braces carry block structure.
    #[test]
    fn test_brace_block_no_indent_tokens() {
        let source = "fn foo() to int { ret 5 }";
        let tokens = lex_tokens(source);
        assert!(
            tokens.contains(&Token::Eof),
            "lexer output must include trailing Eof"
        );
        // The token stream for a brace block should be:
        // Fn, Ident(foo), LParen, RParen, To, Ident(int), LBrace, Ret, IntLit(5), RBrace, Eof
        assert!(tokens.contains(&Token::LBrace), "Should have LBrace");
        assert!(tokens.contains(&Token::RBrace), "Should have RBrace");
        assert!(tokens.contains(&Token::Ret), "Should have Ret");
    }

    /// Newlines are now cosmetic — they are emitted but not structurally significant.
    #[test]
    fn test_newlines_emitted_but_not_structural() {
        let source = "fn foo() to int {\n    ret 5\n}";
        let tokens = lex_tokens(source);
        // Should contain Newline tokens but no Indent/Dedent
        assert!(
            tokens.contains(&Token::Newline),
            "Newlines should be emitted"
        );
        assert!(tokens.contains(&Token::LBrace), "Should have LBrace");
        assert!(tokens.contains(&Token::RBrace), "Should have RBrace");
    }

    #[test]
    fn test_jsx_tokens() {
        let tokens = lex_tokens("<div></div>");
        assert!(tokens.contains(&Token::Lt));
        assert!(tokens.contains(&Token::JsxCloseStart));
    }

    #[test]
    fn test_component_decorator() {
        let tokens = lex_tokens("@component fn Chat() to Element {");
        assert_eq!(tokens[0], Token::AtComponent);
        assert_eq!(tokens[1], Token::Fn);
        assert_eq!(tokens[2], Token::TypeIdent("Chat".to_string()));
        // Opening brace is now the block delimiter
        assert!(tokens.contains(&Token::LBrace));
    }

    #[test]
    fn test_match_expression() {
        let source = "match x {\n    Ok(r) -> r\n    Error(e) -> e\n}";
        let tokens = lex_tokens(source);
        assert!(tokens.contains(&Token::Match));
        assert!(tokens.contains(&Token::Arrow));
        assert!(tokens.contains(&Token::LBrace));
        assert!(tokens.contains(&Token::RBrace));
    }

    #[test]
    fn test_http_route() {
        let tokens = lex_tokens("http post \"/api/chat\" to Result {");
        assert_eq!(tokens[0], Token::Http);
        assert_eq!(tokens[1], Token::Ident("post".into()));
    }

    #[test]
    fn test_double_slash_line_comment_skipped() {
        let source = "let x = 1 // trailing\nlet y = 2";
        let tokens = lex(source);
        let kinds: Vec<&Token> = tokens.iter().map(|s| &s.token).collect();
        assert!(
            !kinds.iter().any(|t| **t == Token::Comment),
            "Comment tokens should be filtered: {:?}",
            kinds
        );
        assert!(matches!(kinds.last(), Some(t) if **t == Token::Eof));
    }

    #[test]
    fn test_agent_environment_tokens() {
        let tokens = lex_tokens("agent environment migrate");
        assert!(tokens.contains(&Token::Agent));
        assert!(tokens.contains(&Token::Environment));
        assert!(tokens.contains(&Token::Migrate));
    }

    #[test]
    fn test_pipe_operator() {
        let tokens = lex_tokens("x |> transform |> render");
        let pipe_count = tokens.iter().filter(|t| **t == Token::PipeOp).count();
        assert_eq!(pipe_count, 2);
    }

    #[test]
    fn test_chatbot_tokenizes() {
        // Brace-syntax chatbot source
        let source = r#"import react.use_state, network.HTTP, llm.Claude

@component fn Chat() to Element {
    let (msgs, set_msgs) = use_state([])
    let (input_val, set_input) = use_state("")
    fn send(_) to Unit {
        set_msgs(msgs.append({role: "user", text: input_val}))
        match HTTP.post("/api/chat", json={input: input_val}) {
            Ok(r) -> set_msgs(msgs.append({role: "ai", text: r.text}))
            Error(e) -> set_msgs(msgs.append({role: "error", text: e.message}))
        }
    }
    <div class="flex flex-col h-screen bg-gray-900 text-white">
        <button on_click={send}>"Send"</button>
    </div>
}

http post "/api/chat" to Result {
    ret spawn(Claude).send(request.json().input)
}"#;

        let tokens = lex(source);
        assert!(!tokens.is_empty());
        assert_eq!(tokens.last().unwrap().token, Token::Eof);
        let token_types: Vec<&Token> = tokens.iter().map(|s| &s.token).collect();
        assert!(token_types.contains(&&Token::Import));
        assert!(token_types.contains(&&Token::AtComponent));
        assert!(token_types.contains(&&Token::Fn));
        assert!(token_types.contains(&&Token::Match));
        assert!(token_types.contains(&&Token::Http));
        assert!(token_types.contains(&&Token::Ident("post".into())));
        assert!(token_types.contains(&&Token::Spawn));
        assert!(token_types.contains(&&Token::LBrace));
        assert!(token_types.contains(&&Token::RBrace));
    }

    #[test]
    fn test_durable_execution_keywords() {
        let tokens = lex_tokens("activity with workflow");
        assert_eq!(
            tokens,
            vec![Token::Activity, Token::With, Token::Workflow, Token::Eof,]
        );
    }
}
