use logos::Logos;

/// All tokens in the Vox language.
/// Keywords are phonetically distinct English words.
/// Operators use English keywords (and, or, not, is, isnt) instead of symbols.
///
/// Block structure is delimited by `{` / `}` (`LBrace` / `RBrace`).
/// Indentation is cosmetic only; the lexer does **not** emit `Indent` or `Dedent` tokens.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")] // skip horizontal whitespace
pub enum Token {
    // ── Keywords ──────────────────────────────────────────────
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("async")]
    Async,
    #[token("mut")]
    Mut,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("loop")]
    Loop,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("match")]
    Match,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("to")]
    To,
    #[token("return")]
    Return,
    #[token("type")]
    TypeKw,
    #[token("dec")]
    Dec,
    #[token("import")]
    Import,
    #[token("actor")]
    Actor,
    #[token("workflow")]
    Workflow,
    #[token("activity")]
    Activity,
    #[token("spawn")]
    Spawn,
    #[token("http")]
    Http,
    #[token("pub")]
    Pub,
    #[token("with")]
    With,
    #[token("on")]
    On,
    #[token("state")]
    State,
    #[token("derived")]
    Derived,
    #[token("effect")]
    Effect,
    #[token("mount")]
    Mount,
    #[token("cleanup")]
    Cleanup,
    #[token("view")]
    View,
    #[token("component")]
    Component,
    #[token("agent")]
    Agent,
    #[token("migrate")]
    Migrate,
    #[token("env")]
    Env,

    // ── Phonetic Operators ────────────────────────────────────
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("is")]
    Is,
    #[token("isnt")]
    Isnt,
    #[token("true")]
    True,
    #[token("false")]
    False,

    // ── Decorators ────────────────────────────────────────────
    #[token("@component")]
    AtComponent,
    #[token("@tool")]
    AtTool,
    #[token("@mcp.tool")]
    AtMcpTool,
    #[token("@resource")]
    AtResource,
    #[token("@mcp.resource")]
    AtMcpResource,
    #[token("@test")]
    AtTest,
    #[token("@endpoint")]
    AtEndpoint,
    #[token("@server")]
    AtServer,
    #[token("@query")]
    AtQuery,
    #[token("@mutation")]
    AtMutation,
    #[token("@table")]
    AtTable,
    #[token("@index")]
    AtIndex,
    #[token("@native")]
    AtNative,
    #[token("@loading")]
    AtLoading,
    #[token("@require")]
    AtRequire,
    #[token("@ensure")]
    AtEnsure,
    #[token("@invariant")]
    AtInvariant,
    #[token("@forall")]
    AtForall,
    #[token("@fuzz")]
    AtFuzz,
    #[token("@pure")]
    AtPure,
    /// `@reactive` — opt-in marker on a free `fn` declaring that its body's
    /// reactive-binding reads should be tracked across calls by the auto-dep
    /// inference pass (Phase E of the Svelte-mineable features plan).
    #[token("@reactive")]
    AtReactive,
    /// `fragment` — typed parametric markup primitive (ADR-033). Body shape
    /// mirrors `view:` (single markup expression). Parsed in Phase F slice 1;
    /// codegen gated on Phase 6 (TASK-6.1) typed primitive surface.
    #[token("fragment")]
    Fragment,
    #[token("@scheduled")]
    AtScheduled,
    #[token("@deprecated")]
    AtDeprecated,
    #[token("@v0")]
    AtV0,
    #[token("@ai")]
    AtAi,

    // ── Symbols ───────────────────────────────────────────────
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    /// Opens a block or an object literal.
    #[token("{")]
    LBrace,
    /// Closes a block or an object literal.
    #[token("}")]
    RBrace,
    #[token(":")]
    Colon,
    #[token("?")]
    Question,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("|>")]
    PipeOp,
    #[token("|")]
    Bar,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Lte,
    #[token(">=")]
    Gte,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("_")]
    Underscore,

    // ── JSX-specific ──────────────────────────────────────────
    #[token("</")]
    JsxCloseStart,
    #[token("/>")]
    JsxSelfClose,
    /// Fragment open `<>` — shorthand for `<React.Fragment>`.
    #[token("<>")]
    JsxFragmentOpen,
    /// Fragment close `</>` — shorthand for `</React.Fragment>`.
    #[token("</>")]
    JsxFragmentClose,

    // ── Literals ──────────────────────────────────────────────
    #[regex(r"[0-9]+\.[0-9]+(dec)?", |lex| {
        let s = lex.slice();
        if s.ends_with("dec") {
            None // Handled by DecLit
        } else {
            s.parse::<f64>().ok()
        }
    })]
    FloatLit(f64),

    #[regex(r"[0-9]+(\.[0-9]+)?dec", |lex| {
        let s = lex.slice();
        Some(s[..s.len()-3].to_string())
    })]
    DecLit(String),

    #[regex(r"[0-9]+", priority = 2, callback = |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),

    #[regex(r#""([^"\\]|\\.)*""#, allow_greedy = true, callback = |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len()-1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n')  => out.push('\n'),
                    Some('t')  => out.push('\t'),
                    Some('r')  => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('"')  => out.push('"'),
                    Some('\'') => out.push('\''),
                    Some('0')  => out.push('\0'),
                    Some(c)    => { out.push('\\'); out.push(c); }
                    None       => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        Some(out)
    })]
    StringLit(String),

    #[regex(r#"'([^'\\]|\\.)*'"#, allow_greedy = true, callback = |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len()-1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n')  => out.push('\n'),
                    Some('t')  => out.push('\t'),
                    Some('r')  => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('"')  => out.push('"'),
                    Some('\'') => out.push('\''),
                    Some('0')  => out.push('\0'),
                    Some(c)    => { out.push('\\'); out.push(c); }
                    None       => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        Some(out)
    })]
    SingleStringLit(String),

    // ── Identifiers ───────────────────────────────────────────
    /// Lower-case identifiers (variables, functions).
    #[regex(r"[a-z_][a-zA-Z0-9_]*", priority = 1, callback = |lex| lex.slice().to_string())]
    Ident(String),

    /// Upper-case identifiers (types, constructors).
    #[regex(r"[A-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    TypeIdent(String),

    // ── Comments ──────────────────────────────────────────────
    /// Line comments: `// …` (JS-style) and `# …` (shell / Vox fixture headers).
    #[regex(r"//[^\r\n]*|#[^\r\n]*", allow_greedy = true, priority = 3)]
    Comment,

    // ── Newlines ─────────────────────────────────────────────
    /// Newline character. Used as a statement separator inside blocks.
    /// Not structural (does not define block nesting — braces do).
    #[regex(r"\n|\r\n")]
    Newline,

    // ── Sentinel ─────────────────────────────────────────────
    /// End-of-file sentinel, injected by [`crate::lexer::cursor::lex`].
    Eof,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Async => write!(f, "async"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Mut => write!(f, "mut"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Match => write!(f, "match"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::To => write!(f, "to"),
            Token::Return => write!(f, "return"),
            Token::TypeKw => write!(f, "type"),
            Token::Dec => write!(f, "dec"),
            Token::Import => write!(f, "import"),
            Token::Actor => write!(f, "actor"),
            Token::Workflow => write!(f, "workflow"),
            Token::Activity => write!(f, "activity"),
            Token::Spawn => write!(f, "spawn"),
            Token::Http => write!(f, "http"),
            Token::Pub => write!(f, "pub"),
            Token::With => write!(f, "with"),
            Token::On => write!(f, "on"),
            Token::State => write!(f, "state"),
            Token::Derived => write!(f, "derived"),
            Token::Effect => write!(f, "effect"),
            Token::Mount => write!(f, "mount"),
            Token::Cleanup => write!(f, "cleanup"),
            Token::View => write!(f, "view"),
            Token::Component => write!(f, "component"),
            Token::Agent => write!(f, "agent"),
            Token::Migrate => write!(f, "migrate"),
            Token::Env => write!(f, "env"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::Not => write!(f, "not"),
            Token::Is => write!(f, "is"),
            Token::Isnt => write!(f, "isnt"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::AtComponent => write!(f, "@component"),
            Token::AtTool => write!(f, "@tool"),
            Token::AtMcpTool => write!(f, "@mcp.tool"),
            Token::AtResource => write!(f, "@resource"),
            Token::AtMcpResource => write!(f, "@mcp.resource"),
            Token::AtTest => write!(f, "@test"),
            Token::AtEndpoint => write!(f, "@endpoint"),
            Token::AtServer => write!(f, "@server"),
            Token::AtQuery => write!(f, "@query"),
            Token::AtMutation => write!(f, "@mutation"),
            Token::AtTable => write!(f, "@table"),
            Token::AtIndex => write!(f, "@index"),
            Token::AtNative => write!(f, "@native"),
            Token::AtLoading => write!(f, "@loading"),
            Token::AtRequire => write!(f, "@require"),
            Token::AtEnsure => write!(f, "@ensure"),
            Token::AtInvariant => write!(f, "@invariant"),
            Token::AtForall => write!(f, "@forall"),
            Token::AtFuzz => write!(f, "@fuzz"),
            Token::AtPure => write!(f, "@pure"),
            Token::AtReactive => write!(f, "@reactive"),
            Token::Fragment => write!(f, "fragment"),
            Token::AtScheduled => write!(f, "@scheduled"),
            Token::AtDeprecated => write!(f, "@deprecated"),
            Token::AtV0 => write!(f, "@v0"),
            Token::AtAi => write!(f, "@ai"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Colon => write!(f, ":"),
            Token::Question => write!(f, "?"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::Eq => write!(f, "="),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::PlusEq => write!(f, "+="),
            Token::MinusEq => write!(f, "-="),
            Token::StarEq => write!(f, "*="),
            Token::SlashEq => write!(f, "/="),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::PipeOp => write!(f, "|>"),
            Token::Bar => write!(f, "|"),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::Lte => write!(f, "<="),
            Token::Gte => write!(f, ">="),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Underscore => write!(f, "_"),
            Token::JsxCloseStart => write!(f, "</"),
            Token::JsxSelfClose => write!(f, "/>"),
            Token::JsxFragmentOpen => write!(f, "<>"),
            Token::JsxFragmentClose => write!(f, "</>"),
            Token::IntLit(v) => write!(f, "{v}"),
            Token::FloatLit(v) => write!(f, "{v}"),
            Token::StringLit(s) => write!(f, "\"{s}\""),
            Token::SingleStringLit(s) => write!(f, "'{s}'"),
            Token::DecLit(s) => write!(f, "{s}dec"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::TypeIdent(s) => write!(f, "{s}"),
            Token::Comment => write!(f, "<comment>"),
            Token::Newline => write!(f, "<newline>"),
            Token::While => write!(f, "while"),
            Token::Loop => write!(f, "loop"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Eof => write!(f, "<eof>"),
        }
    }
}
