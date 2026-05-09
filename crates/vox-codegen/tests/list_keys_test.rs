use vox_codegen::codegen_ts::emitter::generate;
use vox_compiler::{lexer::cursor::lex, parser::parse, hir::lower_module};

fn try_emit(src: &str) -> Result<String, String> {
    let m = parse(lex(src)).map_err(|e| format!("{e:?}"))?;
    let hir = lower_module(&m);
    let out = generate(&hir)?;
    Ok(out.files.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>().join("\n"))
}

#[test]
fn list_render_with_explicit_key_emits_key_prop() {
    let src = r#"
component List(items: list[str]) {
    view: column(raw_class="list") {
        for it in items key=it {
            column(raw_class="item") { it }
        }
    }
}
"#;
    let ts = try_emit(src).expect("emit");
    assert!(ts.contains("key={it}") || ts.contains("key="), "must emit key prop\n---\n{}", ts);
}

#[test]
fn list_render_without_key_fails() {
    let src = r#"
component List(items: list[str]) {
    view: column(raw_class="list") {
        for it in items {
            column(raw_class="item") { it }
        }
    }
}
"#;
    let err = try_emit(src).expect_err("must fail without key");
    assert!(err.contains("validate.list_key.required") || err.contains("key"), "expected key error, got: {err}");
}

#[test]
fn list_with_field_key() {
    let src = r#"
component L(words: list[str]) {
    view: column(raw_class="l") {
        for w in words key=w {
            column(raw_class="word") { w }
        }
    }
}
"#;
    let ts = try_emit(src).expect("emit");
    assert!(ts.contains("key={w}") || ts.contains("key="), "must emit key: {ts}");
}
