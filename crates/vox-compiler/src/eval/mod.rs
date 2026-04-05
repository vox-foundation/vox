pub use vox_eval::*;

pub mod value;
pub mod env;
pub mod expr;
pub mod stmt;
pub mod builtins;

use crate::hir::nodes::HirModule;
use value::VoxValue;
use env::Scope;

#[derive(Debug)]
pub enum EvalError {
    UndefinedVariable(String),
    TypeError { expected: &'static str, found: String },
    ArityMismatch { expected: usize, found: usize },
    StepLimitExceeded,
    AssertionFailed(String),
    Panic(String),
}

pub struct Interpreter {
    pub scope: Scope,
    pub step_limit: usize,
    pub steps: usize,
}

impl Interpreter {
    pub fn new(step_limit: usize) -> Self {
        Self {
            scope: Scope::new(),
            step_limit,
            steps: 0,
        }
    }

    pub fn run_module(&mut self, module: &HirModule) -> Result<(), EvalError> {
        for f in &module.functions {
            let val = VoxValue::Fn {
                params: f.params.iter().map(|p| p.name.clone()).collect(),
                body: f.body.clone(),
                env: self.scope.clone()
            };
            self.scope.set(f.name.clone(), val);
        }
        
        for f in &module.tests {
            let val = VoxValue::Fn {
                params: f.params.iter().map(|p| p.name.clone()).collect(),
                body: f.body.clone(),
                env: self.scope.clone()
            };
            self.scope.set(f.name.clone(), val);
        }

        Ok(())
    }

    pub fn call(&mut self, name: &str, args: Vec<VoxValue>) -> Result<VoxValue, EvalError> {
        let val = self.scope.get(name).cloned().ok_or_else(|| EvalError::UndefinedVariable(name.to_string()))?;
        if let VoxValue::Fn { params, body, mut env } = val {
            if params.len() != args.len() {
                return Err(EvalError::ArityMismatch { expected: params.len(), found: args.len() });
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
            Err(EvalError::TypeError { expected: "function", found: "other".into() })
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
