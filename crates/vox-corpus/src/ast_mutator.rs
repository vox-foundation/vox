use vox_compiler::hir::{HirExpr, HirPattern, HirStmt, HirModule};

/// Mutates an AST node with semantic layout variations for data augmentation.
pub fn mutate_module(module: &mut HirModule) {
    // Currently only mutating the bodies of functions
    for f in module.functions.iter_mut() {
        for stmt in f.body.iter_mut() {
            mutate_stmt(stmt);
        }
    }
}

pub fn mutate_expr(expr: &mut HirExpr) {
    match expr {
        HirExpr::Ident(name, _) => {
            // Variable renaming: swap camelCase to snake_case heuristically
            if name.chars().any(|c| c.is_uppercase()) && !name.contains('_') && name.chars().next().map_or(false, |c| c.is_lowercase()) {
                *name = to_snake_case(name);
            }
        }
        HirExpr::For(var_name, _iterable, boxed_body, _) => {
            // Simple string replace for variable
            if var_name.chars().any(|c| c.is_uppercase()) && !var_name.contains('_') && var_name.chars().next().map_or(false, |c| c.is_lowercase()) {
                *var_name = to_snake_case(var_name);
            }
            mutate_expr(boxed_body);
        }
        HirExpr::Binary(_, left, right, _) => {
            mutate_expr(left);
            mutate_expr(right);
        }
        HirExpr::Unary(_, inner, _) => {
            mutate_expr(inner);
        }
        HirExpr::Call(callee, args, _, _) => {
            mutate_expr(callee);
            for arg in args.iter_mut() {
                mutate_expr(&mut arg.value);
            }
        }
        HirExpr::MethodCall(obj, name, args, _) => {
            mutate_expr(obj);
            if name.chars().any(|c| c.is_uppercase()) && !name.contains('_') && name.chars().next().map_or(false, |c| c.is_lowercase()) {
                *name = to_snake_case(name);
            }
            for arg in args.iter_mut() {
                mutate_expr(&mut arg.value);
            }
        }
        HirExpr::DbTableOp { op: _, args, limit, .. } => {
            for arg in args.iter_mut() {
                mutate_expr(&mut arg.value);
            }
            if let Some(l) = limit {
                mutate_expr(l);
            }
            // We could inject a dummy wrapper conditionally here, but we are traversing
        }
        HirExpr::FieldAccess(obj, field, _) => {
            mutate_expr(obj);
            if field.chars().any(|c| c.is_uppercase()) && !field.contains('_') && field.chars().next().map_or(false, |c| c.is_lowercase()) {
                *field = to_snake_case(field);
            }
        }
        HirExpr::Match(target, arms, _) => {
            mutate_expr(target);
            for arm in arms.iter_mut() {
                mutate_expr(&mut arm.body);
            }
        }
        HirExpr::If(cond, then_body, else_body, _) => {
            mutate_expr(cond);
            for stmt in then_body.iter_mut() {
                mutate_stmt(stmt);
            }
            if let Some(ebody) = else_body {
                for stmt in ebody.iter_mut() {
                    mutate_stmt(stmt);
                }
            }
        }
        HirExpr::Lambda(params, _, body, _) => {
            for p in params.iter_mut() {
                if p.name.chars().any(|c| c.is_uppercase()) && !p.name.contains('_') && p.name.chars().next().map_or(false, |c| c.is_lowercase()) {
                    p.name = to_snake_case(&p.name);
                }
            }
            mutate_expr(body);
        }
        HirExpr::Pipe(left, right, _) => {
            mutate_expr(left);
            mutate_expr(right);
        }
        HirExpr::Spawn(actor, _) => {
            mutate_expr(actor);
        }
        HirExpr::With(opts, body, _) => {
            mutate_expr(opts);
            mutate_expr(body);
        }
        HirExpr::Block(stmts, _) => {
            for stmt in stmts.iter_mut() {
                mutate_stmt(stmt);
            }
        }
        HirExpr::Try(t) => {
            mutate_expr(&mut t.target);
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for item in items.iter_mut() {
                mutate_expr(item);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, val) in fields.iter_mut() {
                mutate_expr(val);
            }
        }
        _ => {}
    }
}

pub fn mutate_stmt(stmt: &mut HirStmt) {
    match stmt {
        HirStmt::Let { pattern, value, .. } => {
            mutate_pattern(pattern);
            mutate_expr(value);
        }
        HirStmt::Assign { target, value, .. } => {
            mutate_expr(target);
            mutate_expr(value);
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                mutate_expr(v);
            }
        }
        HirStmt::Expr { expr, .. } => {
            mutate_expr(expr);
        }
        HirStmt::While { condition, body, .. } => {
            mutate_expr(condition);
            for stmt in body.iter_mut() {
                mutate_stmt(stmt);
            }
        }
        HirStmt::Loop { body, .. } => {
            for stmt in body.iter_mut() {
                mutate_stmt(stmt);
            }
        }
        _ => {}
    }
}

pub fn mutate_pattern(pattern: &mut HirPattern) {
    match pattern {
        HirPattern::Ident(name, _) => {
            if name.chars().any(|c| c.is_uppercase()) && !name.contains('_') && name.chars().next().map_or(false, |c| c.is_lowercase()) {
                *name = to_snake_case(name);
            }
        }
        HirPattern::Tuple(patterns, _) => {
            for p in patterns.iter_mut() {
                mutate_pattern(p);
            }
        }
        HirPattern::Constructor(_, patterns, _) => {
            for p in patterns.iter_mut() {
                mutate_pattern(p);
            }
        }
        _ => {}
    }
}

fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            for lc in c.to_lowercase() {
                snake.push(lc);
            }
        } else {
            snake.push(c);
        }
    }
    snake
}
