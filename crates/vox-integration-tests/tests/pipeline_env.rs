//! Shared env guard + chatbot fixture for pipeline integration tests (`tests/pipeline.rs`).
#![allow(unsafe_code)] // `std::env::set_var` / `remove_var` are `unsafe` (Rust 2024); mutation is serialized by [`ENV_MUTEX`].

use std::ffi::OsString;
use std::sync::Mutex;

/// Serializes all tests that read or write `VOX_EMIT_EXPRESS_SERVER`.
/// Without this, parallel test runners observe the env-var mid-mutation,
/// causing `assert!(!server.ts)` tests to see a stale `=1` value.
pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Sets `VOX_EMIT_EXPRESS_SERVER=1` for the duration of `f`, then restores the prior value.
/// Holds [`ENV_MUTEX`] for the entire call so parallel tests see a stable env.
pub fn with_express_server_enabled<R>(f: impl FnOnce() -> R) -> R {
    let _env_guard = ENV_MUTEX.lock().expect("ENV_MUTEX poisoned");
    const KEY: &str = "VOX_EMIT_EXPRESS_SERVER";
    struct Guard {
        prev: Option<OsString>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(KEY, v) },
                None => unsafe { std::env::remove_var(KEY) },
            }
        }
    }
    let prev = std::env::var_os(KEY);
    unsafe {
        unsafe { std::env::set_var(KEY, "1") };
    }
    let _guard = Guard { prev };
    f()
}

/// Call `generate()` while holding [`ENV_MUTEX`], ensuring the env-var is NOT set.
/// Prevents `codegen_server_has_express_route_with_await` from racing past "without express" tests.
#[macro_export]
macro_rules! generate_without_express {
    ($module:expr) => {{
        let _env_guard = $crate::pipeline_env::ENV_MUTEX
            .lock()
            .expect("ENV_MUTEX poisoned");
        let hir = vox_compiler::hir::lower_module($module);
        vox_compiler::codegen_ts::generate(&hir).expect("Should generate without errors")
    }};
}

pub const CHATBOT_SRC: &str = r#"import react.use_state

type ChatResult =
    | Ok(text: str)
    | Error(message: str)

@component fn Chat() to Element {
    let (messages, set_messages) = use_state([{role: "bot", text: ""}])
    let (input, set_input) = use_state("")
    let send = fn(msg) set_messages(messages.append({role: "user", text: msg}))
    <div class="chat-container">
        <h1>"Vox Chatbot"</h1>
        <button on_click={fn(_e) send(input)}>"Send"</button>
    </div>
}

actor Claude {
    on send(msg: str) to ChatResult {
        Ok("ok")
    }
}

http post "/api/chat" to ChatResult {
    let body = request.json()
    let prompt = body.message
    let response = spawn(Claude).send(prompt)
    ret response
}
"#;
