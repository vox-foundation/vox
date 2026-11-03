use crate::research::domain::codegen::{CodeSandboxResult, verify_rust_code_in_sandbox};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolyglotLanguage {
    Rust,
    TypeScript,
    Python,
    Sql,
    Vox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    GenericSqlite,
    Postgres,
    MySql,
}

pub async fn verify_in_polyglot_sandbox(
    code: &str,
    lang: PolyglotLanguage,
) -> anyhow::Result<CodeSandboxResult> {
    match lang {
        PolyglotLanguage::Rust => verify_rust_code_in_sandbox(code, &[]).await,
        PolyglotLanguage::Sql => verify_sql_in_sandbox(code, SqlDialect::GenericSqlite).await,
        PolyglotLanguage::Python => verify_python_in_sandbox(code).await,
        PolyglotLanguage::TypeScript => verify_typescript_in_sandbox(code).await,
        PolyglotLanguage::Vox => verify_vox_in_sandbox(code).await,
    }
}

pub async fn verify_sql_in_sandbox(
    sql: &str,
    dialect: SqlDialect,
) -> anyhow::Result<CodeSandboxResult> {
    match dialect {
        SqlDialect::GenericSqlite => {
            let pool = turso::Builder::new_local(":memory:").build().await?;
            let conn = pool.connect()?;
            match conn.execute_batch(sql).await {
                Ok(_) => Ok(CodeSandboxResult {
                    passed: true,
                    stdout: "SQLite query executed successfully".to_string(),
                    stderr: String::new(),
                }),
                Err(e) => Ok(CodeSandboxResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                }),
            }
        }
        SqlDialect::Postgres | SqlDialect::MySql => {
            let dialect_impl: Box<dyn sqlparser::dialect::Dialect> = match dialect {
                SqlDialect::Postgres => Box::new(sqlparser::dialect::PostgreSqlDialect {}),
                SqlDialect::MySql => Box::new(sqlparser::dialect::MySqlDialect {}),
                _ => unreachable!(),
            };
            match sqlparser::parser::Parser::parse_sql(&*dialect_impl, sql) {
                Ok(ast) => Ok(CodeSandboxResult {
                    passed: true,
                    stdout: format!(
                        "SQL syntax verified for {dialect:?} ({} statements)",
                        ast.len()
                    ),
                    stderr: String::new(),
                }),
                Err(e) => Ok(CodeSandboxResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: format!("SQL syntax error for {dialect:?}: {e}"),
                }),
            }
        }
    }
}

pub async fn verify_python_in_sandbox(code: &str) -> anyhow::Result<CodeSandboxResult> {
    let temp_file = tempfile::Builder::new().suffix(".py").tempfile()?;
    tokio::fs::write(temp_file.path(), code).await?;

    let python_cmd = if cfg!(windows) { "python" } else { "python3" };

    let syntax_check = tokio::process::Command::new(python_cmd)
        .kill_on_drop(true)
        .arg("-c")
        .arg("import ast, sys; ast.parse(open(sys.argv[1]).read())")
        .arg(temp_file.path())
        .output()
        .await;

    match syntax_check {
        Ok(out) => {
            let passed = out.status.success();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Ok(CodeSandboxResult {
                passed,
                stdout: String::new(),
                stderr,
            })
        }
        Err(e) => Ok(CodeSandboxResult {
            passed: false,
            stdout: String::new(),
            stderr: format!("Python executable failed to launch: {e}"),
        }),
    }
}

pub async fn verify_typescript_in_sandbox(code: &str) -> anyhow::Result<CodeSandboxResult> {
    let temp_dir_guard = tempfile::Builder::new()
        .prefix("vox-ts-sandbox-")
        .tempdir()?;
    let temp_dir = temp_dir_guard.path();
    let ts_file = temp_dir.join("probe.ts");
    let ambient_file = temp_dir.join("ambient.d.ts");

    tokio::fs::write(&ts_file, code).await?;
    tokio::fs::write(&ambient_file, "declare module '*';").await?;

    // Try tsc first
    let tsc_res = tokio::process::Command::new("tsc")
        .kill_on_drop(true)
        .arg("--noEmit")
        .arg("--skipLibCheck")
        .arg(&ambient_file)
        .arg(&ts_file)
        .output()
        .await;

    if let Ok(out) = tsc_res {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let all_errs = format!("{stdout}\n{stderr}");
        let real_errors: Vec<&str> = all_errs
            .lines()
            .filter(|l| l.contains("error TS") && !l.contains("TS2307") && !l.contains("TS7016"))
            .collect();

        return Ok(CodeSandboxResult {
            passed: real_errors.is_empty(),
            stdout,
            stderr: real_errors.join("\n"),
        });
    }

    // Fallback to bun
    let bun_res = tokio::process::Command::new("bun")
        .kill_on_drop(true)
        .args(["build", "--no-bundle"])
        .arg(&ts_file)
        .output()
        .await;

    if let Ok(out) = bun_res {
        return Ok(CodeSandboxResult {
            passed: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        });
    }

    anyhow::bail!("No TypeScript verifier (tsc or bun) available in environment")
}

pub async fn verify_vox_in_sandbox(code: &str) -> anyhow::Result<CodeSandboxResult> {
    let tokens = vox_compiler::lexer::lex(code);
    let parsed = vox_compiler::parser::parse(tokens.clone())
        .or_else(|_| vox_compiler::parser::parse_script(tokens));
    match parsed {
        Ok(_) => Ok(CodeSandboxResult {
            passed: true,
            stdout: "Vox source parsed successfully".to_string(),
            stderr: String::new(),
        }),
        Err(e) => Ok(CodeSandboxResult {
            passed: false,
            stdout: String::new(),
            stderr: format!("Vox syntax error: {e:?}"),
        }),
    }
}
