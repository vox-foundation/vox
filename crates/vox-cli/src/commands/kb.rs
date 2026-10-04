//! `vox kb` — knowledge-base operations (eval now; reindex/status land in Phase 2).

pub mod eval;

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
    fn special_modes_are_mutually_exclusive() {
        assert!(parse(&["eval", "--query", "x", "--headings", "docs/src/a.md"]).is_err());
        assert!(parse(&["eval", "--query", "x", "--snapshot-corpus"]).is_err());
        assert!(parse(&["eval", "--headings", "d", "--snapshot-corpus"]).is_err());
    }
}
