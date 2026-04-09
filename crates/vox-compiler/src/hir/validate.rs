//! HIR structural validation — invariants that should hold after lowering.
//!
//! Emits [`HirValidationError`] values; the CLI pipeline maps these to
//! [`crate::typeck::diagnostics::Diagnostic`] with category [`crate::typeck::diagnostics::DiagnosticCategory::HirInvariant`].

use crate::ast::span::Span;
use crate::hir::*;

/// A HIR validation diagnostic (span + message).
#[derive(Debug)]
pub struct HirValidationError {
    pub message: String,
    pub span: Span,
    pub correction_hint: Option<String>,
}

/// Validate structural invariants of a [`HirModule`].
/// Returns a list of validation errors (empty = no structural violations reported here).
#[must_use]
pub fn validate_module(module: &HirModule) -> Vec<HirValidationError> {
    let mut errors = Vec::new();

    for f in &module.functions {
        validate_fn(f, "function", &mut errors);
    }
    for f in &module.tests {
        validate_fn(f, "test", &mut errors);
    }
    for s in &module.server_fns {
        validate_name_and_params(&s.name, &s.params, s.span, "server fn", &mut errors);
        if s.route_path.is_empty() {
            errors.push(HirValidationError {
                message: "server fn route_path is empty".into(),
                span: s.span,
                correction_hint: None,
            });
        }
    }
    for s in &module.query_fns {
        validate_name_and_params(&s.name, &s.params, s.span, "@query fn", &mut errors);
        if s.route_path.is_empty() {
            errors.push(HirValidationError {
                message: "@query fn route_path is empty".into(),
                span: s.span,
                correction_hint: None,
            });
        }
    }
    for s in &module.mutation_fns {
        validate_name_and_params(&s.name, &s.params, s.span, "@mutation fn", &mut errors);
        if s.route_path.is_empty() {
            errors.push(HirValidationError {
                message: "@mutation fn route_path is empty".into(),
                span: s.span,
                correction_hint: None,
            });
        }
    }
    for m in &module.mcp_tools {
        validate_fn(&m.func, "mcp tool", &mut errors);
    }
    let mut seen_resource_uris = std::collections::HashSet::<&str>::new();
    for m in &module.mcp_resources {
        validate_fn(&m.func, "mcp resource", &mut errors);
        if m.uri.trim().is_empty() {
            errors.push(HirValidationError {
                message: "mcp resource URI must not be empty".into(),
                span: m.func.span,
                correction_hint: None,
            });
        }
        if !seen_resource_uris.insert(m.uri.as_str()) {
            errors.push(HirValidationError {
                message: format!("duplicate @mcp.resource URI: {}", m.uri),
                span: m.func.span,
                correction_hint: None,
            });
        }
        if !m.func.params.is_empty() {
            errors.push(HirValidationError {
                message: "mcp resource function must take no parameters (MCP resources/read supplies only `uri`)".into(),
                span: m.func.span,
                correction_hint: None,
            });
        }
    }

    for w in &module.workflows {
        if w.name.is_empty() {
            errors.push(HirValidationError {
                message: "workflow name is empty".into(),
                span: w.span,
                correction_hint: None,
            });
        }
        for p in &w.params {
            if p.name.is_empty() {
                errors.push(HirValidationError {
                    message: format!("Empty parameter name in workflow '{}'", w.name),
                    span: p.span,
                    correction_hint: None,
                });
            }
        }
    }
    for a in &module.activities {
        if a.name.is_empty() {
            errors.push(HirValidationError {
                message: "activity name is empty".into(),
                span: a.span,
                correction_hint: None,
            });
        }
        for p in &a.params {
            if p.name.is_empty() {
                errors.push(HirValidationError {
                    message: format!("Empty parameter name in activity '{}'", a.name),
                    span: p.span,
                    correction_hint: None,
                });
            }
        }
    }

    for actor in &module.actors {
        if actor.name.is_empty() {
            errors.push(HirValidationError {
                message: "Actor name is empty".into(),
                span: actor.span,
                correction_hint: None,
            });
        }
        for h in &actor.handlers {
            if h.event_name.is_empty() {
                errors.push(HirValidationError {
                    message: format!("Empty handler event name in actor '{}'", actor.name),
                    span: h.span,
                    correction_hint: None,
                });
            }
            for p in &h.params {
                if p.name.is_empty() {
                    errors.push(HirValidationError {
                        message: format!(
                            "Empty parameter name in actor '{}' handler '{}'",
                            actor.name, h.event_name
                        ),
                        span: p.span,
                        correction_hint: None,
                    });
                }
            }
        }
    }

    for r in &module.routes {
        if r.path.trim().is_empty() {
            errors.push(HirValidationError {
                message: "HTTP route path is empty".into(),
                span: r.span,
                correction_hint: None,
            });
        }
    }

    for table in &module.tables {
        if table.name.is_empty() {
            errors.push(HirValidationError {
                message: "Table name is empty".into(),
                span: table.span,
                correction_hint: None,
            });
        }
        for field in &table.fields {
            if field.name.is_empty() {
                errors.push(HirValidationError {
                    message: format!("Empty field name in table '{}'", table.name),
                    span: field.span,
                    correction_hint: None,
                });
            }
        }
    }

    for t in &module.types {
        if t.name.is_empty() {
            errors.push(HirValidationError {
                message: "Type name is empty".into(),
                span: t.span,
                correction_hint: None,
            });
        }
        for v in &t.variants {
            if v.name.is_empty() {
                errors.push(HirValidationError {
                    message: format!("Empty variant name in type '{}'", t.name),
                    span: v.span,
                    correction_hint: None,
                });
            }
            for (fname, _) in &v.fields {
                if fname.is_empty() {
                    errors.push(HirValidationError {
                        message: format!(
                            "Empty field name in variant '{}' of type '{}'",
                            v.name, t.name
                        ),
                        span: v.span,
                        correction_hint: None,
                    });
                }
            }
        }
    }

    for idx in &module.indexes {
        if idx.table_name.is_empty() {
            errors.push(HirValidationError {
                message: "index table_name is empty".into(),
                span: idx.span,
                correction_hint: None,
            });
        }
        if idx.index_name.is_empty() {
            errors.push(HirValidationError {
                message: format!("index name is empty (table '{}')", idx.table_name),
                span: idx.span,
                correction_hint: None,
            });
        }
    }

    for c in &module.collections {
        if c.name.is_empty() {
            errors.push(HirValidationError {
                message: "collection name is empty".into(),
                span: c.span,
                correction_hint: None,
            });
        }
        for field in &c.fields {
            if field.name.is_empty() {
                errors.push(HirValidationError {
                    message: format!("Empty field name in collection '{}'", c.name),
                    span: field.span,
                    correction_hint: None,
                });
            }
        }
    }

    for v in &module.vector_indexes {
        if v.table_name.is_empty() {
            errors.push(HirValidationError {
                message: "vector index table_name is empty".into(),
                span: v.span,
                correction_hint: None,
            });
        }
        if v.index_name.is_empty() {
            errors.push(HirValidationError {
                message: format!("vector index name is empty (table '{}')", v.table_name),
                span: v.span,
                correction_hint: None,
            });
        }
        if v.column.is_empty() {
            errors.push(HirValidationError {
                message: format!("vector index column is empty ('{}')", v.index_name),
                span: v.span,
                correction_hint: None,
            });
        }
    }

    for s in &module.search_indexes {
        if s.table_name.is_empty() {
            errors.push(HirValidationError {
                message: "search index table_name is empty".into(),
                span: s.span,
                correction_hint: None,
            });
        }
        if s.index_name.is_empty() {
            errors.push(HirValidationError {
                message: format!("search index name is empty (table '{}')", s.table_name),
                span: s.span,
                correction_hint: None,
            });
        }
        if s.search_field.is_empty() {
            errors.push(HirValidationError {
                message: format!("search index field is empty ('{}')", s.index_name),
                span: s.span,
                correction_hint: None,
            });
        }
    }

    for rc in &module.reactive_components {
        validate_name_and_params(
            &rc.name,
            &rc.params,
            rc.span,
            "reactive component",
            &mut errors,
        );
    }

    for ri in &module.rust_imports {
        if ri.crate_name.trim().is_empty() {
            errors.push(HirValidationError {
                message: "rust import crate name is empty".into(),
                span: ri.span,
                correction_hint: None,
            });
        }
        if ri.alias.trim().is_empty() {
            errors.push(HirValidationError {
                message: format!("rust import alias is empty for crate '{}'", ri.crate_name),
                span: ri.span,
                correction_hint: None,
            });
        }
        if ri.path.is_some() && ri.git.is_some() {
            errors.push(HirValidationError {
                message: format!(
                    "rust import '{}' has both path and git source configured",
                    ri.crate_name
                ),
                span: ri.span,
                correction_hint: None,
            });
        }
    }

    errors
}

fn validate_fn(f: &HirFn, kind: &str, errors: &mut Vec<HirValidationError>) {
    validate_name_and_params(&f.name, &f.params, f.span, kind, errors);
}

fn validate_name_and_params(
    name: &str,
    params: &[HirParam],
    span: Span,
    kind: &str,
    errors: &mut Vec<HirValidationError>,
) {
    if name.is_empty() {
        errors.push(HirValidationError {
            message: format!("{kind} name is empty"),
            span,
            correction_hint: None,
        });
    }
    for p in params {
        if p.name.is_empty() {
            errors.push(HirValidationError {
                message: format!("Empty parameter name in {kind} '{name}'"),
                span: p.span,
                correction_hint: None,
            });
        }
    }
}
