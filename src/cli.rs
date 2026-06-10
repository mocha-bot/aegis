use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "aegis", version, about = "Agnostic RBAC permission scanner")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan source code for RBAC permission usage
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

    /// Show permissions used in code but missing from RBAC catalog
    Diff {
        /// RBAC API base URL
        #[arg(long)]
        api: String,

        /// Path to .aegis.yaml config file
        #[arg(short, long)]
        config: Option<String>,

        /// Root directory to scan
        #[arg(short, long)]
        root: Option<String>,
    },

    /// CI gate — exit 1 if any permission used in code is missing from catalog
    Lint {
        /// RBAC API base URL
        #[arg(long)]
        api: String,

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
