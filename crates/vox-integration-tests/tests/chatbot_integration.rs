#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_module;

fn check(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(src);
    let module = parse(tokens).expect("Source should parse without errors");
    typecheck_module(&module, "")
}

fn errors(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    check(src)
        .into_iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect()
}

#[test]
fn chatbot_full_stack_integration() {
    // Subset of examples/chatbot.vox shaped for current type inference — v0.3 brace syntax.
    let src = r#"
type ChatResult =
    | Success(text: str)
    | Error(message: str)

component Chat() {
    let (messages, set_messages) = use_state([{role: "bot", text: ""}])
    let (input, set_input) = use_state("")
    let send = fn(_e) set_messages(messages.append({role: "user", text: input}))
    view: (
        <div class="chat-container">
            <h1>"Vox Chatbot"</h1>
            <div class="messages">
                for msg in messages {
                    <div class="message">
                        {msg.text}
                    </div>
                }
            </div>
            <div class="input-area">
                <input class="chat-input" value={input}/>
                <button class="send-btn" on_click={send}>"Send"</button>
            </div>
        </div>
    )
}

http post "/api/chat" to ChatResult {
    let _body = request.json()
    Success("Hello")
}

actor Claude {
    on send(msg: str) to ChatResult {
        Success("Hello from Vox! You said: " + msg)
    }
}
"#;

    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "Chatbot integration test failed with errors: {:?}",
        errs
    );
}
