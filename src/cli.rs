use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "aegis",
    version,
    about = "Agnostic authorization pattern scanner"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan source code for authorization permission usage
    Scan {
        /// Path to .aegis.yaml config file
        #[arg(short, long)]
        config: Option<String>,

        /// Root directory to scan
        #[arg(short, long)]
        root: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value_t = Format::Table)]
        format: Format,

        /// Ignore a specific rule by ID (repeatable)
        #[arg(long = "ignore-rule")]
        ignore_rules: Vec<String>,
    },

    /// Show permissions used in code but missing from authorization catalog
    Diff {
        /// Authorization API base URL (mutually exclusive with --baseline)
        #[arg(long)]
        api: Option<String>,

        /// Path to a local baseline catalog file (mutually exclusive with --api)
        #[arg(long)]
        baseline: Option<String>,

        /// Path to .aegis.yaml config file
        #[arg(short, long)]
        config: Option<String>,

        /// Root directory to scan
        #[arg(short, long)]
        root: Option<String>,
    },

    /// CI gate — exit 1 if any permission used in code is missing from catalog
    Lint {
        /// Authorization API base URL (mutually exclusive with --baseline)
        #[arg(long)]
        api: Option<String>,

        /// Path to a local baseline catalog file (mutually exclusive with --api)
        #[arg(long)]
        baseline: Option<String>,

        /// Path to .aegis.yaml config file
        #[arg(short, long)]
        config: Option<String>,

        /// Root directory to scan
        #[arg(short, long)]
        root: Option<String>,

        /// Ignore a specific rule by ID (repeatable)
        #[arg(long = "ignore-rule")]
        ignore_rules: Vec<String>,
    },
}

#[derive(Clone, ValueEnum)]
pub enum Format {
    Table,
    Csv,
    Json,
    CatalogJson,
}
