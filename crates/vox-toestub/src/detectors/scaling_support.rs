//! Syn visitor and shared helpers for [`super::ScalingSurfacesDetector`].

use std::collections::HashMap;

use regex::Regex;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ImplItemFn, Item, ItemFn, ItemMod, Meta};

use crate::rules::{Finding, Language, Severity, SourceFile};

pub(super) fn recent_line_starts_for_loop(lines: &[String], idx: usize, window: usize) -> bool {
    let start = idx.saturating_sub(window);
    for line in lines.iter().take(idx).skip(start) {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        if t.starts_with("for ") {
            return true;
        }
    }
    false
}

pub(super) fn parse_rust_usize_literal(s: &str) -> Option<u64> {
    let clean: String = s.chars().filter(|c| *c != '_').collect();
    clean.parse().ok()
}

pub(super) fn env_unwrap_or_duplicate_findings(file: &SourceFile, re: &Regex) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    let mut in_test_block = false;
    let test_attr = Regex::new(r"#\[(?:cfg\(test\)|test)\]").expect("valid");

    for (i, line) in file.lines.iter().enumerate() {
        let line_num = i + 1;
        if test_attr.is_match(line) {
            in_test_block = true;
        }
        if in_test_block {
            let trimmed = line.trim();
            if (trimmed.starts_with("fn ") || trimmed.starts_with("mod "))
                && !trimmed.contains("test")
                && !line.starts_with(char::is_whitespace)
            {
                in_test_block = false;
            }
        }
        if in_test_block || line.trim_start().starts_with("//") {
            continue;
        }
        if !line.contains("std::env::var") {
            continue;
        }
        if let Some(c) = re.captures(line) {
            let lit = c.get(1).map(|m| m.as_str()).unwrap_or("");
            if lit.len() < 4 {
                continue;
            }
            map.entry(lit.to_string()).or_default().push(line_num);
        }
    }

    for (lit, lines) in map {
        if lines.len() < 2 {
            continue;
        }
        for &ln in &lines[1..] {
            out.push(Finding {
                rule_id: "scaling/env-default-duplication".to_string(),
                rule_name: "Scaling — duplicate env string default".to_string(),
                severity: Severity::Info,
                file: file.path.clone(),
                line: ln,
                column: 0,
                message: "Same `unwrap_or(\"…\")` default repeated on multiple `std::env::var` lines — centralize (const / policy / SSOT)"
                    .to_string(),
                suggestion: Some(format!("Literal default appears {}×: `{lit}`", lines.len())),
                context: file.context_around(ln, 1),
            });
        }
    }

    out
}

struct ScalingSynVisitor<'a> {
    file: &'a SourceFile,
    findings: Vec<Finding>,
    in_async: bool,
    in_test_module: bool,
    depth_test_mod: usize,
    crate_allow_blocking_fs: bool,
}

fn attr_is_cfg_test(a: &syn::Attribute) -> bool {
    match &a.meta {
        Meta::List(list) if list.path.is_ident("cfg") => list.tokens.to_string().contains("test"),
        _ => false,
    }
}

impl<'ast> Visit<'ast> for ScalingSynVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let is_test = node.attrs.iter().any(attr_is_cfg_test);
        if is_test {
            self.depth_test_mod += 1;
            self.in_test_module = true;
        }
        visit::visit_item_mod(self, node);
        if is_test {
            self.depth_test_mod -= 1;
            if self.depth_test_mod == 0 {
                self.in_test_module = false;
            }
        }
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if is_test_fn(&node.attrs) {
            visit::visit_item_fn(self, node);
            return;
        }
        let was = self.in_async;
        self.in_async = node.sig.asyncness.is_some();
        visit::visit_item_fn(self, node);
        self.in_async = was;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if is_test_fn(&node.attrs) {
            visit::visit_impl_item_fn(self, node);
            return;
        }
        let was = self.in_async;
        self.in_async = node.sig.asyncness.is_some();
        visit::visit_impl_item_fn(self, node);
        self.in_async = was;
    }

    fn visit_item(&mut self, node: &'ast Item) {
        if let Item::Fn(f) = node
            && !is_test_fn(&f.attrs)
        {
            let was = self.in_async;
            self.in_async = f.sig.asyncness.is_some();
            visit::visit_item_fn(self, f);
            self.in_async = was;
            return;
        }
        visit::visit_item(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        if self.in_async
            && !self.crate_allow_blocking_fs
            && !self.in_test_module
            && let Expr::Call(call) = node
        {
            if call_looks_like_std_fs_blocking(&call.func) {
                let span = call.span();
                let line = span.start().line;
                self.findings.push(Finding {
                    rule_id: "scaling/blocking-in-async".to_string(),
                    rule_name: "Scaling — blocking fs in async".to_string(),
                    severity: Severity::Info,
                    file: self.file.path.clone(),
                    line,
                    column: span.start().column,
                    message: "`std::fs` (or known blocking) call inside `async` — use `tokio::fs` or offload via `spawn_blocking`"
                        .to_string(),
                    suggestion: Some(
                        "Policy: `contracts/scaling/policy.yaml` per-crate overrides if intentional."
                            .to_string(),
                    ),
                    context: self.file.context_around(line, 2),
                });
            }
            if call_looks_like_thread_sleep(&call.func) {
                let span = call.span();
                let line = span.start().line;
                self.findings.push(Finding {
                    rule_id: "scaling/thread-sleep-async".to_string(),
                    rule_name: "Scaling — thread sleep".to_string(),
                    severity: Severity::Info,
                    file: self.file.path.clone(),
                    line,
                    column: span.start().column,
                    message: "`thread::sleep` in async context blocks the executor".to_string(),
                    suggestion: Some("`tokio::time::sleep` or structured backoff".to_string()),
                    context: self.file.context_around(line, 2),
                });
            }
        }
        visit::visit_expr(self, node);
    }
}

fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("test") || attr_is_cfg_test(a))
}

fn call_looks_like_std_fs_blocking(func: &Expr) -> bool {
    let Expr::Path(p) = func else {
        return false;
    };
    let segs: Vec<String> = p
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let joined = segs.join("::");
    joined.starts_with("std::fs::")
        || joined == "fs::read_to_string"
        || joined == "fs::read"
        || joined == "fs::write"
}

fn call_looks_like_thread_sleep(func: &Expr) -> bool {
    let Expr::Path(p) = func else {
        return false;
    };
    let last = p
        .path
        .segments
        .last()
        .map(|s| s.ident == "sleep")
        .unwrap_or(false);
    let has_thread = p.path.segments.iter().any(|s| s.ident == "thread");
    last && has_thread
}

pub(super) fn detect_rust_syn_blockings(
    file: &SourceFile,
    crate_allow_blocking_fs: bool,
) -> Vec<Finding> {
    if file.language != Language::Rust {
        return Vec::new();
    }
    let Ok(ast) = syn::parse_file(&file.content) else {
        return Vec::new();
    };
    let mut v = ScalingSynVisitor {
        file,
        findings: Vec::new(),
        in_async: false,
        in_test_module: false,
        depth_test_mod: 0,
        crate_allow_blocking_fs,
    };
    v.visit_file(&ast);
    v.findings
}
