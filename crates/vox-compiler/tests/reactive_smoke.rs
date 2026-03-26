#[test]
fn test_reactive_codegen_smoke() {
    let source = r#"
component Counter(initial: int) {
    state count: int = initial
    derived double = count * 2
    
    mount: {
        log("mounted")
    }

    view: (
        <div class="p-4">
            <h1>"Count: {count}"</h1>
            <p>"Double: {double}"</p>
            <button on:click={count = count + 1}>"Increment"</button>
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    for t in &tokens {
        println!("Token: {:?} at {:?}", t.token, t.span);
    }
    let module = vox_compiler::parser::parse(tokens).expect("Parsing failed");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("Codegen failed");

    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "Counter.tsx")
        .map(|(_, c)| c)
        .expect("Counter.tsx not found");
    println!("Generated TSX:\n{}", ts);

    assert!(ts.contains("function Counter"));
    assert!(ts.contains("useMemo(() => count * 2, [count])"));
    assert!(ts.contains("useEffect(() => {"));
    assert!(ts.contains("onClick={() => {"));
    assert!(ts.contains("set_count(count + 1);"));
}

#[test]
fn test_island_jsx_emits_data_vox_island_mount() {
    let source = r#"
@island DataChart { title: str }

component Panel() {
    state label: str = "Hello"
    view: (
        <div class="wrap">
            <DataChart title={label} />
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("Parsing failed");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("Codegen failed");

    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "Panel.tsx")
        .map(|(_, c)| c)
        .expect("Panel.tsx not found");

    assert!(
        ts.contains("data-vox-island=\"DataChart\""),
        "expected island mount attr, got:\n{ts}"
    );
    assert!(ts.contains("data-prop-title="));

    let meta = output
        .files
        .iter()
        .find(|(f, _)| f == "vox-islands-meta.ts")
        .map(|(_, c)| c)
        .expect("vox-islands-meta.ts");
    assert!(meta.contains("DataChart"));
}
