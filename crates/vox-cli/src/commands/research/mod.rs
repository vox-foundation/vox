use clap::Subcommand;

pub mod eval;
pub mod infra;

#[derive(Subcommand)]
pub enum ResearchCmd {
    /// Start the SearXNG sidecar (requires Docker).
    Up,
    /// Stop the SearXNG sidecar.
    Down,
    /// Check the health of research backends (SearXNG, DDG, Tavily).
    Status,
    /// Run the research evaluation harness against golden queries.
    Eval {
        /// Optional path to a golden query JSONL file.
        #[arg(long)]
        queries: Option<std::path::PathBuf>,
        /// Output path for the evaluation report.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        /// Number of parallel queries to run.
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
    },
}

pub async fn run(cmd: ResearchCmd) -> anyhow::Result<()> {
    match cmd {
        ResearchCmd::Up => infra::up().await,
        ResearchCmd::Down => infra::down().await,
        ResearchCmd::Status => infra::status().await,
        ResearchCmd::Eval {
            queries,
            output,
            concurrency,
        } => eval::run_eval(queries, output, concurrency).await,
    }
}
