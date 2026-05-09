use vox_codegen::codegen_ts::builtin_registry::{BuiltinLowering, BuiltinRegistry};

#[test]
fn registry_has_str_length_as_property() {
    let r = BuiltinRegistry::standard();
    let lo = r.lookup_method("str", "length", 0).expect("str.length");
    assert!(matches!(lo, BuiltinLowering::Property("length")));
}

#[test]
fn registry_has_time_now_ms_inlined() {
    let r = BuiltinRegistry::standard();
    let lo = r
        .lookup_function("std.time.now_ms", 0)
        .expect("std.time.now_ms");
    assert!(matches!(lo, BuiltinLowering::Inline("Date.now()")));
}

#[test]
fn registry_has_speech_namespace_alias() {
    let r = BuiltinRegistry::standard();
    let alias = r.lookup_namespace("Speech").expect("Speech namespace");
    assert_eq!(alias, "Speech"); // not 'mobile'; namespace keeps its name
}

#[test]
fn registry_unknown_method_returns_none() {
    let r = BuiltinRegistry::standard();
    assert!(r.lookup_method("str", "nonexistent", 0).is_none());
}
