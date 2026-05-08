#[inline]
fn env_var_explicitly_disabled(res: Result<String, std::env::VarError>) -> bool {
    match res {
        Ok(v) => {
            v == "0"
                || v.eq_ignore_ascii_case("false")
                || v.eq_ignore_ascii_case("no")
                || v.eq_ignore_ascii_case("off")
        }
        Err(_) => false,
    }
}

/// Web IR lower + validate runs at the end of `vox_compiler_emit::codegen_ts::generate` unless disabled.
///
/// **Default:** validation is **on** (unset). Set `VOX_WEBIR_VALIDATE=0`, `false`, `no`, or `off` to skip.
#[must_use]
pub fn web_ir_validate_gate_enabled() -> bool {
    !env_var_explicitly_disabled(std::env::var("VOX_WEBIR_VALIDATE"))
}

/// Path C reactive `view:` may emit Web IR preview TSX when validate is clean and parity matches.
///
/// **Default:** **on** (unset). Set `VOX_WEBIR_EMIT_REACTIVE_VIEWS=0`, `false`, `no`, or `off` for legacy `emit_hir_expr` views only.
#[must_use]
pub fn web_ir_emit_reactive_views_enabled() -> bool {
    !env_var_explicitly_disabled(std::env::var("VOX_WEBIR_EMIT_REACTIVE_VIEWS"))
}
