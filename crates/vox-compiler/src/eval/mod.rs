pub use vox_eval::*;

pub mod builtins;
pub mod env;
pub mod expr;
pub mod stmt;
pub mod value;

use crate::hir::nodes::HirModule;
use env::Scope;
use value::VoxValue;

#[derive(Debug)]
pub enum EvalError {
    UndefinedVariable(String),
    TypeError {
        expected: &'static str,
        found: String,
    },
    ArityMismatch {
        expected: usize,
        found: usize,
    },
    StepLimitExceeded,
    AssertionFailed(String),
    Panic(String),
}

pub struct Interpreter {
    pub scope: Scope,
    pub step_limit: usize,
    pub steps: usize,
    pub caps: Option<std::collections::HashSet<String>>,
}

impl Interpreter {
    pub fn new(step_limit: usize) -> Self {
        let mut scope = Scope::new();
        // Seed namespaces for glue-code builtins
        scope.set("null".to_string(), VoxValue::Null);
        scope.set(
            "fs".to_string(),
            VoxValue::Object(vec![(
                "__namespace__".to_string(),
                VoxValue::Str("fs".to_string()),
            )]),
        );
        scope.set(
            "process".to_string(),
            VoxValue::Object(vec![(
                "__namespace__".to_string(),
                VoxValue::Str("process".to_string()),
            )]),
        );
        scope.set(
            "env".to_string(),
            VoxValue::Object(vec![(
                "__namespace__".to_string(),
                VoxValue::Str("env".to_string()),
            )]),
        );
        scope.set(
            "path".to_string(),
            VoxValue::Object(vec![(
                "__namespace__".to_string(),
                VoxValue::Str("path".to_string()),
            )]),
        );
        scope.set(
            "clavis".to_string(),
            VoxValue::Object(vec![(
                "__namespace__".to_string(),
                VoxValue::Str("clavis".to_string()),
            )]),
        );
        scope.set(
            "json".to_string(),
            VoxValue::Object(vec![(
                "__namespace__".to_string(),
                VoxValue::Str("json".to_string()),
            )]),
        );

        // Standard library root
        let std_ns = VoxValue::Object(vec![
            (
                "fs".to_string(),
                VoxValue::Object(vec![(
                    "__namespace__".to_string(),
                    VoxValue::Str("fs".to_string()),
                )]),
            ),
            (
                "process".to_string(),
                VoxValue::Object(vec![(
                    "__namespace__".to_string(),
                    VoxValue::Str("process".to_string()),
                )]),
            ),
            (
                "env".to_string(),
                VoxValue::Object(vec![(
                    "__namespace__".to_string(),
                    VoxValue::Str("env".to_string()),
                )]),
            ),
            (
                "path".to_string(),
                VoxValue::Object(vec![(
                    "__namespace__".to_string(),
                    VoxValue::Str("path".to_string()),
                )]),
            ),
            (
                "json".to_string(),
                VoxValue::Object(vec![(
                    "__namespace__".to_string(),
                    VoxValue::Str("json".to_string()),
                )]),
            ),
        ]);
        scope.set("std".to_string(), std_ns);

        Self {
            scope,
            step_limit,
            steps: 0,
            caps: None,
        }
    }

    pub fn run_module(&mut self, module: &HirModule) -> Result<(), EvalError> {
        // Register ADT variant constructors so `Applied(x, y)` etc. work in tests.
        for ty in &module.types {
            for variant in &ty.variants {
                self.scope.set(variant.name.clone(), VoxValue::Constructor(variant.name.clone()));
            }
        }

        for f in &module.functions {
            let val = VoxValue::Fn {
                params: f.params.iter().map(|p| p.name.clone()).collect(),
                body: f.body.clone(),
                env: self.scope.clone(),
            };
            self.scope.set(f.name.clone(), val);
        }

        for f in &module.tests {
            let val = VoxValue::Fn {
                params: f.params.iter().map(|p| p.name.clone()).collect(),
                body: f.body.clone(),
                env: self.scope.clone(),
            };
            self.scope.set(f.name.clone(), val);
        }

        Ok(())
    }

    pub fn call(&mut self, name: &str, args: Vec<VoxValue>) -> Result<VoxValue, EvalError> {
        let val = self
            .scope
            .get(name)
            .cloned()
            .ok_or_else(|| EvalError::UndefinedVariable(name.to_string()))?;
        if let VoxValue::Fn {
            params,
            body,
            mut env,
        } = val
        {
            if params.len() != args.len() {
                return Err(EvalError::ArityMismatch {
                    expected: params.len(),
                    found: args.len(),
                });
            }
            env.push_frame();
            for (p, arg) in params.iter().zip(args) {
                env.set(p.clone(), arg);
            }

            // Temporary variable to hold the old scope context
            let old_scope = self.scope.clone();
            self.scope = env;

            let mut res = VoxValue::Null;
            for s in body {
                res = stmt::eval_stmt(self, &s)?;
                if let VoxValue::_Return(r) = res {
                    res = *r;
                    break;
                }
            }

            self.scope = old_scope;
            Ok(res)
        } else {
            Err(EvalError::TypeError {
                expected: "function",
                found: "other".into(),
            })
        }
    }

    pub fn track_step(&mut self) -> Result<(), EvalError> {
        self.steps += 1;
        if self.steps >= self.step_limit {
            Err(EvalError::StepLimitExceeded)
        } else {
            Ok(())
        }
    }
}
