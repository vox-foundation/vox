//! `vox secrets` — re-exported from `vox_cli_research::secrets`.

pub use vox_cli_research::secrets::*;

#[cfg(test)]
mod oauth_login_cli_tests {
    use clap::Parser;

    #[test]
    fn oauth_flag_parses_on_login_subcommand() {
        let root = crate::VoxCliRoot::try_parse_from([
            "vox",
            "secrets",
            "login",
            "--oauth",
            "--provider",
            "openrouter",
        ])
        .expect("parses");
        match root.cmd {
            crate::Cli::Secrets {
                cmd:
                    vox_cli_research::secrets::SecretsCmd::Login {
                        oauth, provider, ..
                    },
            } => {
                assert!(oauth);
                assert_eq!(provider.as_deref(), Some("openrouter"));
            }
            _ => panic!("expected Cli::Secrets{{ cmd: SecretsCmd::Login }}"),
        }
    }

    #[test]
    fn managed_set_requires_stdin_and_canonical_name() {
        let root = crate::VoxCliRoot::try_parse_from([
            "vox",
            "secrets",
            "set",
            "OPENROUTER_API_KEY",
            "--stdin",
        ])
        .expect("managed set syntax parses");
        match root.cmd {
            crate::Cli::Secrets {
                cmd:
                    vox_cli_research::secrets::SecretsCmd::Set {
                        canonical_env,
                        stdin,
                    },
            } => {
                assert_eq!(canonical_env, "OPENROUTER_API_KEY");
                assert!(stdin);
            }
            _ => panic!("expected managed secrets set command"),
        }

        assert!(
            crate::VoxCliRoot::try_parse_from([
                "vox",
                "secrets",
                "set",
                "OPENROUTER_API_KEY",
                "token-on-command-line",
            ])
            .is_err(),
            "positional token input must be rejected by the command shape"
        );
    }
}
