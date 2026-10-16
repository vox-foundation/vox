use vox_terminal_core::commands::{CommandRegistry, CommandResult};

#[test]
fn nu_command_help_and_dispatch() {
    let reg = CommandRegistry::default();

    // /nu with empty args or "help" surfaces the POSIX-to-Nushell quick reference
    let empty_res = reg.dispatch("/nu");
    match empty_res {
        CommandResult::Output(s) => {
            assert!(s.contains("Usage: /nu"));
            assert!(s.contains("POSIX-to-Nushell Quick Reference"));
            assert!(s.contains("&&"));
            assert!(s.contains("$VAR"));
        }
        _ => panic!("expected Output from /nu"),
    }

    let help_res = reg.dispatch("/nu help");
    match help_res {
        CommandResult::Output(s) => {
            assert!(s.contains("POSIX-to-Nushell Quick Reference"));
        }
        _ => panic!("expected Output from /nu help"),
    }

    // /nu <pipeline> executes or falls back cleanly
    let run_res = reg.dispatch("/nu echo test-val");
    match run_res {
        CommandResult::Output(s) => {
            assert!(s.contains("test-val"));
        }
        _ => panic!("expected Output from /nu echo test-val"),
    }
}

#[test]
fn help_includes_nu_slash_command() {
    let reg = CommandRegistry::default();
    let help_res = reg.dispatch("/help");
    match help_res {
        CommandResult::Output(s) => {
            assert!(s.contains("/nu"));
        }
        _ => panic!("expected Output from /help"),
    }
}
