use crate::builtin_registry::{
    builtin_entry_param_tys, builtin_entry_result_ty, builtin_registry_entries,
};
use crate::typeck::env::{AdtDef, Binding, BindingKind, TypeEnv, VariantDef};
use crate::typeck::ty::Ty;

/// Pre-registered type signatures for the Vox standard library.
///
/// This populates the root scope of a `TypeEnv` with:
/// - Built-in types (Option, Result as ADTs with proper constructors)
/// - Standard library functions (print, str, int, float, len)
/// - React/frontend bindings (`use_state`, `use_effect`, `use_memo`, `use_ref`, `use_callback`)
/// - HTTP/network module bindings
/// - String, list, and record methods
pub struct BuiltinTypes {
    /// Method signatures: type_key → { method_name → return_type }
    methods: std::collections::HashMap<String, std::collections::HashMap<String, Ty>>,
}

impl BuiltinTypes {
    /// Populate the given TypeEnv with all built-in definitions.
    pub fn register_all(env: &mut TypeEnv) -> Self {
        let mut methods: std::collections::HashMap<String, std::collections::HashMap<String, Ty>> =
            std::collections::HashMap::new();

        // ── Built-in ADTs ─────────────────────────────────────

        // Option[T] = | Some(value: T) | None
        env.register_type(AdtDef {
            name: "Option".into(),
            variants: vec![
                VariantDef {
                    name: "Some".into(),
                    fields: vec![("value".into(), Ty::GenericParam(0))],
                },
                VariantDef {
                    name: "None".into(),
                    fields: vec![],
                },
            ],
        });

        // Some(value: T) → Option[T]
        env.define(
            "Some".into(),
            Binding {
                ty: Ty::Fn(
                    vec![Ty::GenericParam(0)],
                    Box::new(Ty::Option(Box::new(Ty::GenericParam(0)))),
                ),
                mutable: false,
                kind: BindingKind::Constructor,
                is_deprecated: false,
            },
        );
        // None → Option[T]
        env.define(
            "None".into(),
            Binding {
                ty: Ty::Option(Box::new(Ty::GenericParam(0))),
                mutable: false,
                kind: BindingKind::Constructor,
                is_deprecated: false,
            },
        );

        // Result[T] = | Ok(value: T) | Error(message: str)
        env.register_type(AdtDef {
            name: "Result".into(),
            variants: vec![
                VariantDef {
                    name: "Ok".into(),
                    fields: vec![("value".into(), Ty::GenericParam(0))],
                },
                VariantDef {
                    name: "Error".into(),
                    fields: vec![("message".into(), Ty::Str)],
                },
            ],
        });

        // Ok(value: T) → Result[T]
        env.define(
            "Ok".into(),
            Binding {
                ty: Ty::Fn(
                    vec![Ty::GenericParam(0)],
                    Box::new(Ty::Result(Box::new(Ty::GenericParam(0)))),
                ),
                mutable: false,
                kind: BindingKind::Constructor,
                is_deprecated: false,
            },
        );
        // Error(message: str) → Result[T]
        env.define(
            "Error".into(),
            Binding {
                ty: Ty::Fn(
                    vec![Ty::Str],
                    // Error returns Result[T]
                    Box::new(Ty::Result(Box::new(Ty::GenericParam(0)))),
                ),
                mutable: false,
                kind: BindingKind::Constructor,
                is_deprecated: false,
            },
        );

        // bool as an ADT
        env.define(
            "true".into(),
            Binding {
                ty: Ty::Bool,
                mutable: false,
                kind: BindingKind::Constructor,
                is_deprecated: false,
            },
        );
        env.define(
            "false".into(),
            Binding {
                ty: Ty::Bool,
                mutable: false,
                kind: BindingKind::Constructor,
                is_deprecated: false,
            },
        );

        // ── Standard library functions ────────────────────────

        // print(value: str) → Unit
        env.define(
            "print".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::Str], Box::new(Ty::Unit)),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // std — namespace for `std.fs.*`, `std.path.*`, `std.env.*`, `std.process.*`,
        // `std.json.*`, `std.http.*`, `std.crypto.*`, `std.time.*`, `std.log.*`,
        // and direct hash/time helpers.
        env.define(
            "std".into(),
            Binding {
                ty: Ty::Named("StdNamespace".into()),
                mutable: false,
                kind: BindingKind::Import,
                is_deprecated: false,
            },
        );

        // str(value: any) → str
        env.define(
            "str".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::Str)),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // int(value: any) → int
        env.define(
            "int".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::Int)),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // float(value: any) → float
        env.define(
            "float".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::Float)),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // len(collection: any) → int
        env.define(
            "len".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::Int)),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // ── React/frontend bindings ───────────────────────────

        // use_state: fn(T) -> (T, fn(T) -> Unit)
        // use_state(initial: T) → (T, fn(T) → Unit)
        env.define(
            "use_state".into(),
            Binding {
                ty: Ty::Fn(
                    vec![Ty::GenericParam(0)],
                    Box::new(Ty::Tuple(vec![
                        Ty::GenericParam(0),
                        Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::Unit)),
                    ])),
                ),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // use_effect(fn() → Unit) → Unit
        env.define(
            "use_effect".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::Fn(vec![], Box::new(Ty::Unit))], Box::new(Ty::Unit)),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        env.define(
            "use_memo".into(),
            Binding {
                ty: Ty::Fn(
                    vec![Ty::Fn(vec![], Box::new(Ty::GenericParam(0)))],
                    Box::new(Ty::GenericParam(0)),
                ),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        env.define(
            "use_ref".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::GenericParam(0))),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        env.define(
            "use_callback".into(),
            Binding {
                ty: Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::GenericParam(0))),
                mutable: false,
                kind: BindingKind::Function,
                is_deprecated: false,
            },
        );

        // DOM / synthetic types (field access is special-cased in `Checker`)
        env.define_type("KeyboardEvent".into(), Ty::Named("KeyboardEvent".into()));

        // ── HTTP/network module ───────────────────────────────

        // HTTP module binding
        env.define(
            "HTTP".into(),
            Binding {
                ty: Ty::Named("HTTPModule".into()),
                mutable: false,
                kind: BindingKind::Import,
                is_deprecated: false,
            },
        );

        // request binding removed from global scope
        // It is now injected into HTTP route scopes in check.rs

        // Claude LLM actor (built-in)
        env.define(
            "Claude".into(),
            Binding {
                ty: Ty::Named("ClaudeActor".into()),
                mutable: false,
                kind: BindingKind::Actor,
                is_deprecated: false,
            },
        );

        // Speech-to-text module (Oratio / Candle Whisper — codegen links `vox-oratio`)
        env.define(
            "Speech".into(),
            Binding {
                ty: Ty::Named("SpeechModule".into()),
                mutable: false,
                kind: BindingKind::Import,
                is_deprecated: false,
            },
        );

        // OpenClaw gateway module (WS-first runtime adapter).
        env.define(
            "OpenClaw".into(),
            Binding {
                ty: Ty::Named("OpenClawModule".into()),
                mutable: false,
                kind: BindingKind::Import,
                is_deprecated: false,
            },
        );

        // Chromium/CDP browser module (native runtime only).
        env.define(
            "Browser".into(),
            Binding {
                ty: Ty::Named("BrowserModule".into()),
                mutable: false,
                kind: BindingKind::Import,
                is_deprecated: false,
            },
        );

        // Mobile native bridge (std.mobile).
        env.define(
            "mobile".into(),
            Binding {
                ty: Ty::Named("StdMobileNs".into()),
                mutable: false,
                kind: BindingKind::Import,
                is_deprecated: false,
            },
        );

        // ── Method registrations ──────────────────────────────

        // List methods
        let mut list_methods = std::collections::HashMap::new();
        list_methods.insert(
            "append".into(),
            Ty::Fn(
                vec![Ty::GenericParam(0)],
                Box::new(Ty::List(Box::new(Ty::GenericParam(0)))),
            ),
        );
        list_methods.insert("length".into(), Ty::Fn(vec![], Box::new(Ty::Int)));
        list_methods.insert(
            "map".into(),
            Ty::Fn(
                vec![Ty::Fn(
                    vec![Ty::GenericParam(0)],
                    Box::new(Ty::GenericParam(1)),
                )],
                Box::new(Ty::List(Box::new(Ty::GenericParam(1)))),
            ),
        );
        list_methods.insert(
            "filter".into(),
            Ty::Fn(
                vec![Ty::Fn(vec![Ty::GenericParam(0)], Box::new(Ty::Bool))],
                Box::new(Ty::List(Box::new(Ty::GenericParam(0)))),
            ),
        );
        methods.insert("List".into(), list_methods);

        // String methods
        let mut str_methods = std::collections::HashMap::new();
        str_methods.insert("length".into(), Ty::Fn(vec![], Box::new(Ty::Int)));
        str_methods.insert("contains".into(), Ty::Fn(vec![Ty::Str], Box::new(Ty::Bool)));
        str_methods.insert(
            "split".into(),
            Ty::Fn(vec![Ty::Str], Box::new(Ty::List(Box::new(Ty::Str)))),
        );
        str_methods.insert("trim".into(), Ty::Fn(vec![], Box::new(Ty::Str)));
        str_methods.insert("to_upper".into(), Ty::Fn(vec![], Box::new(Ty::Str)));
        str_methods.insert("to_lower".into(), Ty::Fn(vec![], Box::new(Ty::Str)));
        methods.insert("Str".into(), str_methods);

        // HTTP module methods
        let mut http_methods = std::collections::HashMap::new();
        http_methods.insert(
            "post".into(),
            Ty::Fn(
                vec![Ty::Str],
                Box::new(Ty::Result(Box::new(Ty::Named("Response".into())))),
            ),
        );
        http_methods.insert(
            "get".into(),
            Ty::Fn(
                vec![Ty::Str],
                Box::new(Ty::Result(Box::new(Ty::Named("Response".into())))),
            ),
        );
        http_methods.insert(
            "put".into(),
            Ty::Fn(
                vec![Ty::Str],
                Box::new(Ty::Result(Box::new(Ty::Named("Response".into())))),
            ),
        );
        http_methods.insert(
            "delete".into(),
            Ty::Fn(
                vec![Ty::Str],
                Box::new(Ty::Result(Box::new(Ty::Named("Response".into())))),
            ),
        );
        methods.insert("HTTPModule".into(), http_methods);

        // Speech module: transcribe(path: str) → Result[str] (refined display text)
        let mut speech_methods = std::collections::HashMap::new();
        speech_methods.insert(
            "transcribe".into(),
            Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str)))),
        );
        methods.insert("SpeechModule".into(), speech_methods);

        // OpenClaw module methods come from shared builtin registry entries.
        let mut openclaw_methods = std::collections::HashMap::new();
        for entry in builtin_registry_entries()
            .iter()
            .copied()
            .filter(|e| e.namespace == "OpenClaw")
        {
            let Some(params) = builtin_entry_param_tys(entry) else {
                continue;
            };
            openclaw_methods.insert(
                entry.name.to_string(),
                Ty::Fn(params, Box::new(builtin_entry_result_ty(entry))),
            );
        }
        methods.insert("OpenClawModule".into(), openclaw_methods);

        let mut browser_methods = std::collections::HashMap::new();
        for entry in builtin_registry_entries()
            .iter()
            .copied()
            .filter(|e| e.namespace == "Browser")
        {
            let Some(params) = builtin_entry_param_tys(entry) else {
                continue;
            };
            browser_methods.insert(
                entry.name.to_string(),
                Ty::Fn(params, Box::new(builtin_entry_result_ty(entry))),
            );
        }
        methods.insert("BrowserModule".into(), browser_methods);

        let mut mobile_methods = std::collections::HashMap::new();
        for entry in builtin_registry_entries()
            .iter()
            .copied()
            .filter(|e| e.namespace == "std.mobile")
        {
            let Some(params) = builtin_entry_param_tys(entry) else {
                continue;
            };
            mobile_methods.insert(
                entry.name.to_string(),
                Ty::Fn(params, Box::new(builtin_entry_result_ty(entry))),
            );
        }
        methods.insert("StdMobileNs".into(), mobile_methods);

        // Request methods
        let mut req_methods = std::collections::HashMap::new();
        req_methods.insert(
            "json".into(),
            Ty::Fn(vec![], Box::new(Ty::Named("JsonBody".into()))),
        );
        req_methods.insert("text".into(), Ty::Fn(vec![], Box::new(Ty::Str)));
        methods.insert("Request".into(), req_methods);

        // Response methods
        let mut resp_methods = std::collections::HashMap::new();
        resp_methods.insert("text".into(), Ty::Fn(vec![], Box::new(Ty::Str)));
        resp_methods.insert(
            "json".into(),
            Ty::Fn(vec![], Box::new(Ty::Named("JsonBody".into()))),
        );
        resp_methods.insert("status".into(), Ty::Fn(vec![], Box::new(Ty::Int)));
        methods.insert("Response".into(), resp_methods);

        Self { methods }
    }

    /// Look up a method on a given type.
    pub fn lookup_method(&self, obj_ty: &Ty, method: &str) -> Option<Ty> {
        if let Ty::Table(_, fields) = obj_ty {
            return match method {
                "insert" => {
                    // insert(item: Record) -> Result[i64]
                    let item_ty = Ty::Record(fields.clone());
                    Some(Ty::Fn(
                        vec![item_ty],
                        Box::new(Ty::Result(Box::new(Ty::Int))),
                    ))
                }
                "get" => {
                    // get(id: int) -> Result[Option[Record]]
                    let record_ty = Ty::Record(fields.clone());
                    Some(Ty::Fn(
                        vec![Ty::Int],
                        Box::new(Ty::Result(Box::new(Ty::Option(Box::new(record_ty))))),
                    ))
                }
                "delete" => {
                    // delete(id: int) -> Result[Unit]
                    Some(Ty::Fn(
                        vec![Ty::Int],
                        Box::new(Ty::Result(Box::new(Ty::Unit))),
                    ))
                }
                "query" => {
                    // query(sql: str) -> Result[List[Record]] (Simplified params)
                    let record_ty = Ty::Record(fields.clone());
                    Some(Ty::Fn(
                        vec![Ty::Str],
                        Box::new(Ty::Result(Box::new(Ty::List(Box::new(record_ty))))),
                    ))
                }
                "all" => {
                    let record_ty = Ty::Record(fields.clone());
                    Some(Ty::Fn(
                        vec![],
                        Box::new(Ty::Result(Box::new(Ty::List(Box::new(record_ty))))),
                    ))
                }
                "count" => Some(Ty::Fn(vec![], Box::new(Ty::Result(Box::new(Ty::Int))))),
                "find" => {
                    let record_ty = Ty::Record(fields.clone());
                    Some(Ty::Fn(
                        vec![Ty::Int],
                        Box::new(Ty::Result(Box::new(Ty::Option(Box::new(record_ty))))),
                    ))
                }
                _ => None,
            };
        }

        let type_key = match obj_ty {
            Ty::Named(n) => n.as_str(),
            Ty::List(_) => "List",
            Ty::Str => "Str",
            Ty::Result(_) => "Result",
            Ty::Option(_) => "Option",
            _ => return None,
        };
        self.methods.get(type_key)?.get(method).cloned()
    }

    /// Look up a variable in builtins (legacy interface, used by old infer code).
    pub fn lookup_var(&self, name: &str) -> Option<Ty> {
        // This is now handled by TypeEnv, so this method is a no-op.
        // Kept for backward compatibility during migration.
        let _ = std::hint::black_box(name.as_ptr() as usize);
        None
    }
}

impl Default for BuiltinTypes {
    fn default() -> Self {
        let mut env = TypeEnv::new();
        Self::register_all(&mut env)
    }
}
