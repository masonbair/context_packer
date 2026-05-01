use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "context-packer")]
#[command(about = "Smart context assembly for AI agents - packs relevant code within token budgets")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Project root directory (default: current directory)
    #[arg(long, global = true)]
    pub project_root: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pack context for a query
    Pack {
        /// Task or query description
        #[arg(short, long)]
        query: String,

        /// Token budget (default: 8000)
        #[arg(short, long, default_value = "8000")]
        budget: usize,

        /// Target model: claude, gpt4, gpt35, gemini
        #[arg(short, long, default_value = "claude")]
        model: String,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include file dependencies
        #[arg(long)]
        include_dependencies: bool,

        /// Dry run - show what would be included without outputting
        #[arg(long)]
        dry_run: bool,

        /// Focus on specific file(s)
        #[arg(short, long)]
        file: Vec<PathBuf>,
    },

    /// Interactive mode - guided context packing
    Interactive,

    /// Manage cache
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Show cache statistics
    Stats,

    /// Clear cache
    Clear {
        /// Clear entries older than N days
        #[arg(long)]
        older_than: Option<u32>,
    },
}
