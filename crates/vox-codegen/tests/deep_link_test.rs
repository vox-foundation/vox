use vox_codegen::codegen_ts::emitter::generate;
use vox_compiler::{hir::lower_module, lexer::cursor::lex, parser::parse};

fn emit(src: &str) -> String {
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    generate(&hir)
        .expect("emit")
        .files
        .iter()
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn deep_link_emits_app_url_open_listener() {
    let src = r#"
@endpoint(kind: query) fn handle_link(url: str) to str { return "/" }
@deep_link {
    scheme: "voxmental"
    on_link: handle_link
}
"#;
    let ts = emit(src);
    assert!(ts.contains("appUrlOpen"), "got:\n{ts}");
    assert!(ts.contains("handle_link("), "got:\n{ts}");
    assert!(
        ts.contains("useNavigate"),
        "must import useNavigate, got:\n{ts}"
    );
    assert!(
        ts.contains("useEffect"),
        "must import useEffect, got:\n{ts}"
    );
}

#[test]
fn back_button_and_deep_link_deduplicates_app_import() {
    let src = r#"
@endpoint(kind: query) fn handle_back() to bool { return true }
@endpoint(kind: query) fn handle_link(url: str) to str { return "/" }
@back_button { on_press: handle_back }
@deep_link { scheme: "vox" on_link: handle_link }
"#;
    let ts = emit(src);
    // App import must appear exactly once.
    let count = ts.matches("from '@capacitor/app'").count();
    assert_eq!(
        count, 1,
        "App import should appear once, got {count} times in:\n{ts}"
    );
}
