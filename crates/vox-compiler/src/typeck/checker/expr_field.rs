use crate::ast::span::Span;
use crate::hir::HirExpr;
use crate::typeck::diagnostics::Diagnostic;
use crate::typeck::env::BindingKind;
use crate::typeck::ty::Ty;

use super::Checker;

impl<'a> Checker<'a> {
    pub(super) fn check_expr_field_access(
        &mut self,
        object: &HirExpr,
        field: &str,
        span: Span,
    ) -> Ty {
        let raw_obj = self.check_expr(object);
        let obj_ty = self.uf.resolve(&raw_obj);
        match &obj_ty {
            Ty::Named(n) if n == "JsonBody" => match field {
                "message" => Ty::Str,
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Field '{field}' not found on JsonBody"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "KeyboardEvent" => match field {
                "key" => Ty::Str,
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Field '{field}' not found on KeyboardEvent"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "StdNamespace" => match field {
                "fs" => Ty::Named("StdFsNs".into()),
                "path" => Ty::Named("StdPathNs".into()),
                "env" => Ty::Named("StdEnvNs".into()),
                "process" => Ty::Named("StdProcessNs".into()),
                "json" => Ty::Named("StdJsonNs".into()),
                "args" => Ty::List(Box::new(Ty::Str)),
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown std submodule or field '{field}'"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "StdFsNs" => match field {
                "read" | "remove" | "mkdir" => {
                    Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str))))
                }
                "read_bytes" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str)))),
                "write" => Ty::Fn(
                    vec![Ty::Str, Ty::Str],
                    Box::new(Ty::Result(Box::new(Ty::Unit))),
                ),
                "exists" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Bool)),
                "list_dir" => Ty::Fn(
                    vec![Ty::Str],
                    Box::new(Ty::Result(Box::new(Ty::List(Box::new(Ty::Str))))),
                ),
                "glob" => Ty::Fn(
                    vec![Ty::Str],
                    Box::new(Ty::Result(Box::new(Ty::List(Box::new(Ty::Str))))),
                ),
                "remove_dir_all" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Unit)))),
                "copy" => Ty::Fn(
                    vec![Ty::Str, Ty::Str],
                    Box::new(Ty::Result(Box::new(Ty::Unit))),
                ),
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown std.fs method '{field}'"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "StdPathNs" => match field {
                "join" => Ty::Fn(vec![Ty::Str, Ty::Str], Box::new(Ty::Str)),
                "join_many" => Ty::Fn(vec![Ty::List(Box::new(Ty::Str))], Box::new(Ty::Str)),
                "basename" | "dirname" | "extension" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Str)),
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown std.path method '{field}'"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "StdEnvNs" => match field {
                "get" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Option(Box::new(Ty::Str)))),
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown std.env method '{field}'"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "StdProcessNs" => match field {
                "run" => Ty::Fn(
                    vec![Ty::Str, Ty::List(Box::new(Ty::Str))],
                    Box::new(Ty::Result(Box::new(Ty::Int))),
                ),
                "run_ex" => Ty::Fn(
                    vec![
                        Ty::Str,
                        Ty::List(Box::new(Ty::Str)),
                        Ty::Str,
                        Ty::List(Box::new(Ty::Str)),
                    ],
                    Box::new(Ty::Result(Box::new(Ty::Int))),
                ),
                "run_capture" => Ty::Fn(
                    vec![Ty::Str, Ty::List(Box::new(Ty::Str))],
                    Box::new(Ty::Result(Box::new(Ty::Record(vec![
                        ("exit".into(), Ty::Int),
                        ("stdout".into(), Ty::Str),
                        ("stderr".into(), Ty::Str),
                    ])))),
                ),
                "run_capture_ex" => Ty::Fn(
                    vec![
                        Ty::Str,
                        Ty::List(Box::new(Ty::Str)),
                        Ty::Str,
                        Ty::List(Box::new(Ty::Str)),
                    ],
                    Box::new(Ty::Result(Box::new(Ty::Record(vec![
                        ("exit".into(), Ty::Int),
                        ("stdout".into(), Ty::Str),
                        ("stderr".into(), Ty::Str),
                    ])))),
                ),
                "exit" => Ty::Fn(vec![Ty::Int], Box::new(Ty::Never)),
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown std.process method '{field}'"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Named(n) if n == "StdJsonNs" => match field {
                "read_str" => Ty::Fn(
                    vec![Ty::Str, Ty::Str],
                    Box::new(Ty::Result(Box::new(Ty::Str))),
                ),
                "read_f64" => Ty::Fn(
                    vec![Ty::Str, Ty::Str],
                    Box::new(Ty::Result(Box::new(Ty::Float))),
                ),
                "quote" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Str)),
                _ => {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown std.json method '{field}'"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            },
            Ty::Record(fields) | Ty::Table(_, fields) | Ty::Collection(_, fields) => fields
                .iter()
                .find(|(n, _)| n == field)
                .map(|(_, t)| t.clone())
                .unwrap_or_else(|| {
                    self.diags.push(Diagnostic::error(
                        format!("Field '{field}' not found on {obj_ty:?}"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }),
            Ty::Database => {
                if let Some(binding) = self.env.lookup(field) {
                    if binding.kind == BindingKind::Table {
                        binding.ty.clone()
                    } else {
                        self.diags.push(Diagnostic::error(
                            format!("Unknown table '{field}' in database"),
                            span,
                            self.source,
                        ));
                        Ty::Error
                    }
                } else {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown table '{field}' in database"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            }
            Ty::Error => Ty::Error,
            _ => {
                self.diags.push(Diagnostic::error(
                    format!("Cannot access field '{field}' on {obj_ty:?}"),
                    span,
                    self.source,
                ));
                Ty::Error
            }
        }
    }
}
