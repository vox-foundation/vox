/// Track 4 — Enter key dispatches the classified intent through `Session`,
/// instead of `app.rs` discarding it (`let _intent = input.submit();`).
///
/// Headless: drives `InputBox` + `Session` directly, the same two calls
/// `app.rs`'s Enter handler makes, without spinning up a real terminal.
use vox_term::ui::input::InputBox;
use vox_terminal_core::block::{BlockKind, BlockStatus};
use vox_terminal_core::session::Session;

#[test]
fn enter_on_a_shell_line_actually_runs_it() {
    let mut input = InputBox::new();
    for c in "!echo hello-from-vox-term".chars() {
        input.push_char(c);
    }

    let mut session = Session::new("main");
    let intent = input.submit();
    let id = session.submit(intent);

    let block = session.blocks().iter().find(|b| b.id == id).unwrap();
    assert_eq!(block.kind, BlockKind::Shell);
    assert_eq!(block.status, BlockStatus::Ok);
    assert!(
        block.plain_output().contains("hello-from-vox-term"),
        "unexpected output: {:?}",
        block.plain_output()
    );

    // The buffer is cleared on submit (existing InputBox behavior).
    assert!(input.buf.is_empty());
}

#[test]
fn enter_on_a_slash_command_dispatches_through_the_registry() {
    let mut input = InputBox::new();
    for c in "/help".chars() {
        input.push_char(c);
    }

    let mut session = Session::new("main");
    let intent = input.submit();
    let id = session.submit(intent);

    let block = session.blocks().iter().find(|b| b.id == id).unwrap();
    assert_eq!(block.kind, BlockKind::SlashCommand);
    assert_eq!(block.status, BlockStatus::Ok);
    assert!(block.plain_output().contains("/help"));
}
