use super::value::VoxValue;
use secrecy::ExposeSecret;

/// Dispatch a method call on a runtime value. Returns `None` if the method is
/// not known — callers should surface a user-visible `MethodNotFound` error.
pub fn call_builtin_method(
    obj: &VoxValue,
    method: &str,
    args: Vec<VoxValue>,
    caps: Option<&std::collections::HashSet<String>>,
) -> Option<VoxValue> {
    match obj {
        // ── List ──────────────────────────────────────────────────────
        VoxValue::List(v) => match method {
            "len" => Some(VoxValue::Int(v.len() as i64)),
            "is_empty" => Some(VoxValue::Bool(v.is_empty())),
            "push" => {
                let mut owned = v.clone();
                if let Some(val) = args.into_iter().next() {
                    owned.push(val);
                }
                Some(VoxValue::List(owned))
            }
            "pop" => {
                let mut owned = v.clone();
                let popped = owned.pop().unwrap_or(VoxValue::Null);
                Some(popped)
            }
            "get" => {
                let idx = args.into_iter().next()?;
                if let VoxValue::Int(i) = idx {
                    v.get(i as usize).cloned().or(Some(VoxValue::Null))
                } else {
                    Some(VoxValue::Null)
                }
            }
            "first" => Some(v.first().cloned().unwrap_or(VoxValue::Null)),
            "last" => Some(v.last().cloned().unwrap_or(VoxValue::Null)),
            "contains" => {
                let target = args.into_iter().next().unwrap_or(VoxValue::Null);
                Some(VoxValue::Bool(v.contains(&target)))
            }
            "join" => {
                let sep = match args.into_iter().next() {
                    Some(VoxValue::Str(s)) => s,
                    _ => String::new(),
                };
                let strings: Vec<String> = v.iter().map(|x| format!("{x:?}")).collect();
                Some(VoxValue::Str(strings.join(&sep)))
            }
            "reverse" => {
                let mut owned = v.clone();
                owned.reverse();
                Some(VoxValue::List(owned))
            }
            _ => None,
        },

        // ── Str ───────────────────────────────────────────────────────
        VoxValue::Str(s) => match method {
            "len" => Some(VoxValue::Int(s.len() as i64)),
            "is_empty" => Some(VoxValue::Bool(s.is_empty())),
            "to_upper" | "to_uppercase" => Some(VoxValue::Str(s.to_uppercase())),
            "to_lower" | "to_lowercase" => Some(VoxValue::Str(s.to_lowercase())),
            "trim" => Some(VoxValue::Str(s.trim().to_string())),
            "trim_start" => Some(VoxValue::Str(s.trim_start().to_string())),
            "trim_end" => Some(VoxValue::Str(s.trim_end().to_string())),
            "contains" => {
                let needle = match args.into_iter().next() {
                    Some(VoxValue::Str(n)) => n,
                    _ => return Some(VoxValue::Bool(false)),
                };
                Some(VoxValue::Bool(s.contains(&*needle)))
            }
            "starts_with" => {
                let prefix = match args.into_iter().next() {
                    Some(VoxValue::Str(p)) => p,
                    _ => return Some(VoxValue::Bool(false)),
                };
                Some(VoxValue::Bool(s.starts_with(&*prefix)))
            }
            "ends_with" => {
                let suffix = match args.into_iter().next() {
                    Some(VoxValue::Str(sf)) => sf,
                    _ => return Some(VoxValue::Bool(false)),
                };
                Some(VoxValue::Bool(s.ends_with(&*suffix)))
            }
            "split" => {
                let delim = match args.into_iter().next() {
                    Some(VoxValue::Str(d)) => d,
                    _ => " ".to_string(),
                };
                let parts: Vec<VoxValue> = s
                    .split(&*delim)
                    .map(|p| VoxValue::Str(p.to_string()))
                    .collect();
                Some(VoxValue::List(parts))
            }
            "replace" => {
                let mut it = args.into_iter();
                let from = match it.next() {
                    Some(VoxValue::Str(f)) => f,
                    _ => return Some(VoxValue::Str(s.clone())),
                };
                let to = match it.next() {
                    Some(VoxValue::Str(t)) => t,
                    _ => String::new(),
                };
                Some(VoxValue::Str(s.replace(&*from, &to)))
            }
            "repeat" => {
                let n = match args.into_iter().next() {
                    Some(VoxValue::Int(n)) => n as usize,
                    _ => 1,
                };
                Some(VoxValue::Str(s.repeat(n)))
            }
            "chars_count" => Some(VoxValue::Int(s.chars().count() as i64)),
            "to_str" | "to_string" => Some(VoxValue::Str(s.clone())),
            _ => None,
        },

        // ── Int ───────────────────────────────────────────────────────
        VoxValue::Int(n) => match method {
            "to_str" | "to_string" => Some(VoxValue::Str(n.to_string())),
            "abs" => Some(VoxValue::Int(n.unsigned_abs() as i64)),
            "min" => {
                let other = match args.into_iter().next() {
                    Some(VoxValue::Int(m)) => m,
                    _ => *n,
                };
                Some(VoxValue::Int(*n.min(&other)))
            }
            "max" => {
                let other = match args.into_iter().next() {
                    Some(VoxValue::Int(m)) => m,
                    _ => *n,
                };
                Some(VoxValue::Int(*n.max(&other)))
            }
            _ => None,
        },

        // ── Float ─────────────────────────────────────────────────────
        VoxValue::Float(f) => match method {
            "to_str" | "to_string" => Some(VoxValue::Str(f.to_string())),
            "abs" => Some(VoxValue::Float(f.abs())),
            "floor" => Some(VoxValue::Float(f.floor())),
            "ceil" => Some(VoxValue::Float(f.ceil())),
            "round" => Some(VoxValue::Float(f.round())),
            "sqrt" => Some(VoxValue::Float(f.sqrt())),
            _ => None,
        },

        // ── Bool ──────────────────────────────────────────────────────
        VoxValue::Bool(b) => match method {
            "to_str" | "to_string" => Some(VoxValue::Str(b.to_string())),
            _ => None,
        },

        // ── Object (including Namespaces) ───────────────────────────
        VoxValue::Object(fields) => {
            if method == "get" {
                let key = match args.into_iter().next() {
                    Some(VoxValue::Str(s)) => s,
                    _ => return Some(VoxValue::Null),
                };
                return Some(fields.iter().find(|(k, _)| k == &key).map(|(_, v)| v.clone()).unwrap_or(VoxValue::Null));
            }

            let ns = fields.iter().find(|(k, _)| k == "__namespace__").and_then(|(_, v)| {
                if let VoxValue::Str(s) = v { Some(s.as_str()) } else { None }
            });

            if let Some(ns_str) = ns {
                if let Some(c) = caps {
                    if (ns_str == "fs" || ns_str == "process" || ns_str == "env" || ns_str == "clavis") && !c.contains(ns_str) {
                        println!("Capability denied: script missing capability '{}'", ns_str);
                        return Some(VoxValue::Null);
                    }
                }
            }

            match ns {
                Some("fs") => match method {
                    "read_file" => {
                        let path = match args.into_iter().next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        match std::fs::read_to_string(path) {
                            Ok(s) => Some(VoxValue::Str(s)),
                            Err(_) => Some(VoxValue::Null),
                        }
                    }
                    "write_file" => {
                        let mut it = args.into_iter();
                        let path = match it.next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        let content = match it.next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        match std::fs::write(path, content) {
                            Ok(_) => Some(VoxValue::Bool(true)),
                            Err(_) => Some(VoxValue::Bool(false)),
                        }
                    }
                    "list_dir" => {
                        let path = match args.into_iter().next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => ".".to_string(),
                        };
                        if let Ok(entries) = std::fs::read_dir(path) {
                            let list: Vec<VoxValue> = entries
                                .filter_map(|e| e.ok())
                                .map(|e| VoxValue::Str(e.path().to_string_lossy().to_string()))
                                .collect();
                            Some(VoxValue::List(list))
                        } else {
                            Some(VoxValue::List(vec![]))
                        }
                    }
                    _ => None,
                },
                Some("env") => match method {
                    "get" => {
                        let name = match args.into_iter().next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        Some(std::env::var(name).map(VoxValue::Str).unwrap_or(VoxValue::Null))
                    }
                    _ => None,
                },
                Some("path") => match method {
                    "join" => {
                        let mut it = args.into_iter();
                        let a = match it.next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        let b = match it.next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        let joined = std::path::Path::new(&a).join(b);
                        Some(VoxValue::Str(joined.to_string_lossy().to_string()))
                    }
                    _ => None,
                },
                Some("clavis") => match method {
                    "resolve" => {
                        let name = match args.into_iter().next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        
                        let id = match std::str::FromStr::from_str(&name) {
                            Ok(id) => id,
                            Err(_) => return Some(VoxValue::Null),
                        };
                        
                        let resolved = vox_clavis::resolve_secret_with_context(id, "script");
                        if let Some(val) = resolved.value {
                            Some(VoxValue::Str(val.expose_secret().to_string()))
                        } else {
                            Some(VoxValue::Null)
                        }
                    }
                    _ => None,
                },
                Some("process") => match method {
                    "spawn" | "run" => {
                        let mut it = args.into_iter();
                        let cmd_name = match it.next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        let cmd_args = match it.next() {
                            Some(VoxValue::List(ls)) => ls.into_iter().filter_map(|v| {
                                if let VoxValue::Str(s) = v { Some(s) } else { None }
                            }).collect::<Vec<_>>(),
                            _ => vec![],
                        };
                        
                        let output = std::process::Command::new(cmd_name)
                            .args(cmd_args)
                            .output();
                            
                        match output {
                            Ok(out) => {
                                let mut res = Vec::new();
                                res.push(("stdout".to_string(), VoxValue::Str(String::from_utf8_lossy(&out.stdout).to_string())));
                                res.push(("stderr".to_string(), VoxValue::Str(String::from_utf8_lossy(&out.stderr).to_string())));
                                res.push(("code".to_string(), VoxValue::Int(out.status.code().unwrap_or(0) as i64)));
                                Some(VoxValue::Object(res))
                            }
                            Err(_) => Some(VoxValue::Null),
                        }
                    }
                    _ => None,
                },
                Some("json") => match method {
                    "parse" => {
                        let s = match args.into_iter().next() {
                            Some(VoxValue::Str(s)) => s,
                            _ => return Some(VoxValue::Null),
                        };
                        match serde_json::from_str::<serde_json::Value>(&s) {
                            Ok(v) => Some(json_to_vox(v)),
                            Err(_) => Some(VoxValue::Null),
                        }
                    }
                    "stringify" | "encode" => {
                        let v = match args.into_iter().next() {
                            Some(v) => v,
                            _ => return Some(VoxValue::Null),
                        };
                        let j = vox_to_json(v);
                        Some(VoxValue::Str(serde_json::to_string(&j).unwrap_or_default()))
                    }
                    _ => None,
                },
                _ => None,
            }
        }

        _ => None,
    }
}

fn vox_to_json(v: VoxValue) -> serde_json::Value {
    match v {
        VoxValue::Int(n) => serde_json::Value::Number(n.into()),
        VoxValue::Float(f) => serde_json::json!(f),
        VoxValue::Str(s) => serde_json::Value::String(s),
        VoxValue::Bool(b) => serde_json::Value::Bool(b),
        VoxValue::Null => serde_json::Value::Null,
        VoxValue::List(ls) => {
            serde_json::Value::Array(ls.into_iter().map(vox_to_json).collect())
        }
        VoxValue::Object(fields) => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                if k == "__namespace__" { continue; }
                map.insert(k, vox_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        VoxValue::Tuple(ls) => {
            serde_json::Value::Array(ls.into_iter().map(vox_to_json).collect())
        }
        _ => serde_json::Value::Null,
    }
}

fn json_to_vox(v: serde_json::Value) -> VoxValue {
    match v {
        serde_json::Value::Null => VoxValue::Null,
        serde_json::Value::Bool(b) => VoxValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                VoxValue::Int(i)
            } else {
                VoxValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => VoxValue::Str(s),
        serde_json::Value::Array(arr) => {
            VoxValue::List(arr.into_iter().map(json_to_vox).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut fields = Vec::new();
            for (k, v) in obj {
                fields.push((k, json_to_vox(v)));
            }
            VoxValue::Object(fields)
        }
    }
}

/// Attempt to call a global built-in function (not a method).
/// Returns `None` if `name` is not a known global.
pub fn call_global_builtin(name: &str, args: Vec<VoxValue>) -> Option<VoxValue> {
    match name {
        "print" => {
            let msg = args
                .iter()
                .map(|v| vox_value_display(v))
                .collect::<Vec<_>>()
                .join(" ");
            println!("{msg}");
            Some(VoxValue::Null)
        }
        "assert" => {
            let cond = args.first();
            let ok = matches!(cond, Some(VoxValue::Bool(true)));
            if !ok {
                let msg = args
                    .get(1)
                    .map(|v| vox_value_display(v))
                    .unwrap_or_else(|| "Assertion failed".to_string());
                eprintln!("assertion failed: {msg}");
                // Surface as Null — callers can check via EvalError::AssertionFailed
                return None; // signals caller to raise AssertionFailed
            }
            Some(VoxValue::Null)
        }
        "len" => {
            let v = args.into_iter().next()?;
            match v {
                VoxValue::List(ls) => Some(VoxValue::Int(ls.len() as i64)),
                VoxValue::Str(s) => Some(VoxValue::Int(s.len() as i64)),
                VoxValue::Object(o) => Some(VoxValue::Int(o.len() as i64)),
                _ => Some(VoxValue::Null),
            }
        }
        "str" => {
            let v = args.into_iter().next().unwrap_or(VoxValue::Null);
            Some(VoxValue::Str(vox_value_display(&v)))
        }
        "int" => {
            let v = args.into_iter().next().unwrap_or(VoxValue::Null);
            match v {
                VoxValue::Int(n) => Some(VoxValue::Int(n)),
                VoxValue::Float(f) => Some(VoxValue::Int(f as i64)),
                VoxValue::Str(s) => Some(VoxValue::Int(s.trim().parse::<i64>().unwrap_or(0))),
                VoxValue::Bool(b) => Some(VoxValue::Int(if b { 1 } else { 0 })),
                _ => Some(VoxValue::Int(0)),
            }
        }
        "float" => {
            let v = args.into_iter().next().unwrap_or(VoxValue::Null);
            match v {
                VoxValue::Float(f) => Some(VoxValue::Float(f)),
                VoxValue::Int(n) => Some(VoxValue::Float(n as f64)),
                VoxValue::Str(s) => Some(VoxValue::Float(s.trim().parse::<f64>().unwrap_or(0.0))),
                _ => Some(VoxValue::Float(0.0)),
            }
        }
        "bool" => {
            let v = args.into_iter().next().unwrap_or(VoxValue::Null);
            let b = match v {
                VoxValue::Bool(b) => b,
                VoxValue::Int(n) => n != 0,
                VoxValue::Float(f) => f != 0.0,
                VoxValue::Str(s) => !s.is_empty(),
                VoxValue::Null => false,
                VoxValue::List(l) => !l.is_empty(),
                _ => true,
            };
            Some(VoxValue::Bool(b))
        }
        "range" => {
            let mut it = args.into_iter();
            let (start, end) = match (it.next(), it.next()) {
                (Some(VoxValue::Int(e)), None) => (0, e),
                (Some(VoxValue::Int(s)), Some(VoxValue::Int(e))) => (s, e),
                _ => return Some(VoxValue::List(vec![])),
            };
            let list: Vec<VoxValue> = (start..end).map(VoxValue::Int).collect();
            Some(VoxValue::List(list))
        }
        "type_of" => {
            let v = args.into_iter().next().unwrap_or(VoxValue::Null);
            let t = match v {
                VoxValue::Int(_) => "int",
                VoxValue::Float(_) => "float",
                VoxValue::Str(_) => "str",
                VoxValue::Bool(_) => "bool",
                VoxValue::List(_) => "List",
                VoxValue::Object(_) => "Object",
                VoxValue::Tuple(_) => "Tuple",
                VoxValue::Null => "null",
                VoxValue::Fn { .. } => "fn",
                _ => "unknown",
            };
            Some(VoxValue::Str(t.to_string()))
        }
        _ => None,
    }
}

fn vox_value_display(v: &VoxValue) -> String {
    match v {
        VoxValue::Int(n) => n.to_string(),
        VoxValue::Float(f) => f.to_string(),
        VoxValue::Str(s) => s.clone(),
        VoxValue::Bool(b) => b.to_string(),
        VoxValue::Null => "null".to_string(),
        VoxValue::List(ls) => {
            let items: Vec<String> = ls.iter().map(vox_value_display).collect();
            format!("[{}]", items.join(", "))
        }
        VoxValue::Object(o) => {
            let fields: Vec<String> = o
                .iter()
                .map(|(k, v)| format!("{k}: {}", vox_value_display(v)))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
        VoxValue::Tuple(t) => {
            let items: Vec<String> = t.iter().map(vox_value_display).collect();
            format!("({})", items.join(", "))
        }
        _ => format!("{v:?}"),
    }
}
