//! `vox kb` — knowledge-base operations (eval now; reindex/status land in Phase 2).

pub mod eval;
pub(crate) mod gate;

/// Knowledge-base operations (eval now; reindex/status land in Phase 2).
#[derive(clap::Subcommand, Debug)]
pub enum KbCmd {
    /// Retrieval eval over the golden set, pooling queries, heading listing and corpus snapshot.
    Eval(eval::KbEvalArgs),
}

/// Dispatch a `vox kb` subcommand.
pub fn run(cmd: KbCmd) -> anyhow::Result<()> {
    match cmd {
        KbCmd::Eval(args) => eval::run(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Harness {
        #[command(subcommand)]
        cmd: KbCmd,
    }

    fn parse(args: &[&str]) -> Result<Harness, clap::Error> {
        Harness::try_parse_from(std::iter::once("kb").chain(args.iter().copied()))
    }

    #[test]
    fn plain_eval_defaults_to_dev() {
        let KbCmd::Eval(a) = parse(&["eval"]).unwrap().cmd;
        assert_eq!(a.split, eval::SplitArg::Dev);
        assert!(!a.json && a.query.is_none() && !a.snapshot_corpus);
    }

    #[test]
    fn top_requires_query() {
        assert!(parse(&["eval", "--top", "5"]).is_err());
        assert!(parse(&["eval", "--query", "x", "--top", "5"]).is_ok());
    }

    #[test]
    fn ratchet_modes_reject_explicit_split() {
        for mode in ["--gate", "--tighten"] {
            assert!(parse(&["eval", mode, "--split", "test"]).is_err(), "{mode}");
            assert!(parse(&["eval", mode, "--split", "dev"]).is_err(), "{mode}");
            assert!(parse(&["eval", mode]).is_ok(), "{mode}");
        }
        let wb = ["--write-baseline", "--epoch-note", "n"];
        assert!(parse(&[&["eval"], &wb[..], &["--split", "dev"]].concat()).is_err());
        // Ratchet modes never mix with each other or the special modes.
        assert!(parse(&["eval", "--gate", "--tighten"]).is_err());
        assert!(parse(&["eval", "--gate", "--query", "x"]).is_err());
        assert!(parse(&["eval", "--tighten", "--snapshot-corpus"]).is_err());
        assert!(parse(&[&["eval", "--gate"], &wb[..]].concat()).is_err());
    }

    #[test]
    fn write_baseline_requires_epoch_note() {
        assert!(parse(&["eval", "--write-baseline"]).is_err());
        assert!(parse(&["eval", "--epoch-note", "n"]).is_err());
        let KbCmd::Eval(a) = parse(&["eval", "--write-baseline", "--epoch-note", "n"])
            .unwrap()
            .cmd;
        assert!(a.write_baseline && a.epoch_note.as_deref() == Some("n"));
    }

    #[test]
    fn base_requires_gate() {
        assert!(parse(&["eval", "--base", "origin/main"]).is_err());
        let KbCmd::Eval(a) = parse(&["eval", "--gate", "--base", "x"]).unwrap().cmd;
        assert_eq!(a.base, "x");
        let KbCmd::Eval(a) = parse(&["eval", "--gate"]).unwrap().cmd;
        assert_eq!(a.base, "origin/main");
    }

    #[test]
    fn special_modes_are_mutually_exclusive() {
        assert!(parse(&["eval", "--query", "x", "--headings", "docs/src/a.md"]).is_err());
        assert!(parse(&["eval", "--query", "x", "--snapshot-corpus"]).is_err());
        assert!(parse(&["eval", "--headings", "d", "--snapshot-corpus"]).is_err());
    }
}
