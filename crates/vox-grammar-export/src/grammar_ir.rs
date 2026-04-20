//! Shared Intermediate Representation for Vox Grammars.
//!
//! This module provides the core data structures used by constrained inference
//! backends (Earley, PDA) and grammar exporters (XGrammar-2).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single production rule: `name = symbols ;`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Production {
    pub name: String,
    pub symbols: Vec<Symbol>,
}

/// A symbol in a production RHS.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Symbol {
    /// Reference to another production by name.
    NonTerminal(String),
    /// A literal string that must appear verbatim.
    Terminal(String),
    /// Matches any single identifier character `[a-zA-Z_][a-zA-Z0-9_]*`.
    IdentClass,
    /// Matches any digit sequence.
    DigitClass,
}

/// Parsed grammar ready for Earley/PDA recognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grammar {
    pub productions: Vec<Production>,
    /// Map from non-terminal name to indices into `productions`.
    pub index: HashMap<String, Vec<usize>>,
    /// The start symbol (first production name).
    pub start: String,
}

impl Grammar {
    /// Build a [`Grammar`] from the Vox EBNF string.
    ///
    /// This is a lightweight parser for the EBNF subset used by `emit_ebnf()`.
    /// It extracts production names and their RHS symbols, converting EBNF
    /// notation into flat production lists.
    pub fn from_ebnf(ebnf: &str) -> Result<Self, String> {
        let mut productions = Vec::new();

        for line in ebnf.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("(*") {
                continue;
            }
            let Some(eq_pos) = line.find('=') else {
                continue;
            };
            if line[eq_pos + 1..].starts_with('=') {
                continue;
            }
            let name = line[..eq_pos].trim().to_string();
            if name.is_empty() || name.contains('"') {
                continue;
            }

            let rhs = line[eq_pos + 1..].trim().trim_end_matches(';').trim();

            let mut extra_prods = Vec::new();
            for alt in split_alternatives(rhs) {
                let symbols = parse_symbols_with_expansion(alt.trim(), &mut extra_prods);
                productions.push(Production {
                    name: name.clone(),
                    symbols,
                });
            }
            productions.extend(extra_prods);
        }

        if productions.is_empty() {
            return Err("EBNF produced zero productions".into());
        }

        let start = productions[0].name.clone();
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, p) in productions.iter().enumerate() {
            index.entry(p.name.clone()).or_default().push(i);
        }

        Ok(Grammar {
            productions,
            index,
            start,
        })
    }
}

fn split_alternatives(rhs: &str) -> Vec<&str> {
    let mut alts = Vec::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut last = 0;

    for (i, c) in rhs.char_indices() {
        match c {
            '"' | '\'' => in_quote = !in_quote,
            '(' | '[' | '{' if !in_quote => depth += 1,
            ')' | ']' | '}' if !in_quote => depth -= 1,
            '|' if !in_quote && depth == 0 => {
                alts.push(&rhs[last..i]);
                last = i + 1;
            }
            _ => {}
        }
    }
    alts.push(&rhs[last..]);
    alts
}

static SYNTH_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn next_synth_name(prefix: &str) -> String {
    let n = SYNTH_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("_{prefix}_{n}")
}

fn parse_symbols_with_expansion(alt: &str, extra_prods: &mut Vec<Production>) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut chars = alt.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | ',' | ';' => {
                chars.next();
            }
            '"' => {
                chars.next();
                let mut lit = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '"' {
                        chars.next();
                        break;
                    }
                    lit.push(ch);
                    chars.next();
                }
                if !lit.is_empty() {
                    symbols.push(Symbol::Terminal(lit));
                }
            }
            '\'' => {
                chars.next();
                let mut lit = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '\'' {
                        chars.next();
                        break;
                    }
                    lit.push(ch);
                    chars.next();
                }
                if !lit.is_empty() {
                    symbols.push(Symbol::Terminal(lit));
                }
            }
            '[' => {
                chars.next();
                let inner = collect_until(&mut chars, ']');
                let inner_syms = parse_symbols_with_expansion(&inner, extra_prods);
                if !inner_syms.is_empty() {
                    let name = next_synth_name("opt");
                    extra_prods.push(Production {
                        name: name.clone(),
                        symbols: inner_syms,
                    });
                    extra_prods.push(Production {
                        name: name.clone(),
                        symbols: vec![],
                    });
                    symbols.push(Symbol::NonTerminal(name));
                }
            }
            '{' => {
                chars.next();
                let inner = collect_until(&mut chars, '}');
                let inner_syms = parse_symbols_with_expansion(&inner, extra_prods);
                if !inner_syms.is_empty() {
                    let name = next_synth_name("rep");
                    let mut rec_syms = inner_syms;
                    rec_syms.push(Symbol::NonTerminal(name.clone()));
                    extra_prods.push(Production {
                        name: name.clone(),
                        symbols: rec_syms,
                    });
                    extra_prods.push(Production {
                        name: name.clone(),
                        symbols: vec![],
                    });
                    symbols.push(Symbol::NonTerminal(name));
                }
            }
            '(' => {
                chars.next();
                let inner = collect_until(&mut chars, ')');
                let inner_syms = parse_symbols_with_expansion(&inner, extra_prods);
                symbols.extend(inner_syms);
            }
            _ if c.is_alphanumeric() || c == '_' => {
                let mut word = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        word.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match word.as_str() {
                    "letter" | "any_char" | "text" => symbols.push(Symbol::IdentClass),
                    "digit" => symbols.push(Symbol::DigitClass),
                    _ => symbols.push(Symbol::NonTerminal(word)),
                }
            }
            _ => {
                chars.next();
            }
        }
    }
    symbols
}

fn collect_until(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, closer: char) -> String {
    let mut buf = String::new();
    let mut depth = 0i32;
    let opener = match closer {
        ']' => '[',
        '}' => '{',
        ')' => '(',
        _ => closer,
    };
    while let Some(&c) = chars.peek() {
        if c == closer && depth == 0 {
            chars.next();
            break;
        }
        if c == opener {
            depth += 1;
        }
        if c == closer {
            depth -= 1;
        }
        buf.push(c);
        chars.next();
    }
    buf
}
