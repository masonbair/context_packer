mod cache;
mod cli;
mod error;
mod format;
mod pack;
mod rank;
mod tokens;

use anyhow::Result;
use clap::Parser;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use cache::{CacheEntry, CacheManager};
use cli::{CacheCommands, Cli, Commands};
use format::get_formatter;
use pack::{ContextPacker, ScoredFile, TokenBudget};
use rank::RelevanceScorer;
use tokens::TiktokenCounter;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack {
            query,
            budget,
            model,
            output,
            include_dependencies: _,
            dry_run,
            file,
        } => {
            let project_root = cli.project_root.unwrap_or_else(|| PathBuf::from("."));
            run_pack(
                &project_root,
                &query,
                budget,
                &model,
                output,
                dry_run,
                file,
                cli.verbose,
                false, // use_cache
            )?;
        }
        Commands::Interactive => {
            run_interactive(cli.project_root, cli.verbose)?;
        }
        Commands::Cache { action } => match action {
            CacheCommands::Stats => {
                let cache = CacheManager::new()?;
                let stats = cache.stats()?;
                println!("{}", stats);
            }
            CacheCommands::Clear { older_than } => {
                let cache = CacheManager::new()?;
                let cleared = if let Some(days) = older_than {
                    cache.clear_older_than(days)?
                } else {
                    cache.clear()?
                };
                println!("Cleared {} cache entries", cleared);
            }
        },
    }

    Ok(())
}

fn run_pack(
    project_root: &PathBuf,
    query: &str,
    budget: usize,
    model: &str,
    output: Option<PathBuf>,
    dry_run: bool,
    files: Vec<PathBuf>,
    verbose: bool,
    use_cache: bool,
) -> Result<()> {
    // Check cache first
    if use_cache && !dry_run {
        if let Ok(cache) = CacheManager::new() {
            if let Ok(Some(entry)) = cache.get(query, model, budget) {
                if verbose {
                    eprintln!("Cache hit! Using cached context ({} tokens)", entry.tokens_used);
                }
                if let Some(output_path) = output {
                    let mut file = std::fs::File::create(&output_path)?;
                    file.write_all(entry.packed_context.as_bytes())?;
                } else {
                    print!("{}", entry.packed_context);
                }
                return Ok(());
            }
        }
    }

    // Create token counter
    let counter = Arc::new(TiktokenCounter::for_model(model)?);

    // Create budget and packer
    let token_budget = TokenBudget::new(budget, counter);
    let mut packer = ContextPacker::new(token_budget);

    // Find and score files
    let scored_files = if files.is_empty() {
        if verbose {
            eprintln!("Scoring files in {}...", project_root.display());
        }
        let scorer = RelevanceScorer::new(project_root.clone());
        scorer.score_files(query)?
    } else {
        // Use provided files with default scores
        files
            .into_iter()
            .map(|path| ScoredFile { path, score: 1.0 })
            .collect()
    };

    if dry_run {
        println!("Query: \"{}\"", query);
        println!("Budget: {} tokens", budget);
        println!("Model: {}", model);
        println!("\nFiles ranked by relevance:");
        for (i, file) in scored_files.iter().enumerate() {
            println!(
                "  {}. {} (score: {:.2})",
                i + 1,
                file.path.display(),
                file.score
            );
        }
        return Ok(());
    }

    // Pack the context
    let context = packer.pack(scored_files.clone())?;

    // Format output
    let formatter = get_formatter(model);
    let formatted = formatter.format(&context, query);

    // Store in cache
    if use_cache {
        if let Ok(cache) = CacheManager::new() {
            let file_paths: Vec<PathBuf> =
                context.included_files.iter().map(|f| f.path.clone()).collect();
            if let Ok(file_hashes) = cache.hash_files(&file_paths) {
                let entry = CacheEntry {
                    query: query.to_string(),
                    model: model.to_string(),
                    budget,
                    file_hashes,
                    packed_context: formatted.clone(),
                    tokens_used: context.tokens_used,
                    created_at: chrono::Utc::now().timestamp(),
                };
                let _ = cache.store(&entry);
            }
        }
    }

    // Write output
    if let Some(output_path) = output {
        let mut file = std::fs::File::create(&output_path)?;
        file.write_all(formatted.as_bytes())?;
        if verbose {
            println!(
                "Wrote {} tokens to {}",
                context.tokens_used,
                output_path.display()
            );
        }
    } else {
        print!("{}", formatted);
    }

    Ok(())
}

fn run_interactive(project_root: Option<PathBuf>, verbose: bool) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let project_root = project_root.unwrap_or_else(|| PathBuf::from("."));

    println!("=== Context Packer Interactive Mode ===\n");

    // Query
    print!("What are you working on? ");
    stdout.flush()?;
    let mut query = String::new();
    stdin.lock().read_line(&mut query)?;
    let query = query.trim().to_string();

    if query.is_empty() {
        println!("No query provided. Exiting.");
        return Ok(());
    }

    // Budget
    print!("Token budget? [8000]: ");
    stdout.flush()?;
    let mut budget_str = String::new();
    stdin.lock().read_line(&mut budget_str)?;
    let budget: usize = budget_str.trim().parse().unwrap_or(8000);

    // Model
    print!("Target model? (claude/gpt4/gpt35/gemini) [claude]: ");
    stdout.flush()?;
    let mut model = String::new();
    stdin.lock().read_line(&mut model)?;
    let model = if model.trim().is_empty() {
        "claude".to_string()
    } else {
        model.trim().to_string()
    };

    println!("\n--- Scoring files... ---\n");

    // Score files
    let counter = Arc::new(TiktokenCounter::for_model(&model)?);
    let scorer = RelevanceScorer::new(project_root.clone());
    let scored_files = scorer.score_files(&query)?;

    // Show preview
    println!("Files ranked by relevance:");
    for (i, file) in scored_files.iter().take(10).enumerate() {
        println!("  {}. {} (score: {:.2})", i + 1, file.path.display(), file.score);
    }
    if scored_files.len() > 10 {
        println!("  ... and {} more files", scored_files.len() - 10);
    }

    // Confirm
    print!("\nProceed with packing? (y/n) [y]: ");
    stdout.flush()?;
    let mut confirm = String::new();
    stdin.lock().read_line(&mut confirm)?;
    if confirm.trim().to_lowercase() == "n" {
        println!("Cancelled.");
        return Ok(());
    }

    println!("\n--- Packing context... ---\n");

    // Pack
    let token_budget = TokenBudget::new(budget, counter);
    let mut packer = ContextPacker::new(token_budget);
    let context = packer.pack(scored_files)?;

    // Show result
    println!("Included {} files, {} tokens used ({:.1}% of budget)",
        context.included_files.len(),
        context.tokens_used,
        (context.tokens_used as f64 / budget as f64) * 100.0
    );

    if !context.omitted_files.is_empty() {
        println!("Omitted {} files due to budget constraints", context.omitted_files.len());
    }

    // Output option
    print!("\nOutput to file? (enter path or leave empty for stdout): ");
    stdout.flush()?;
    let mut output_path = String::new();
    stdin.lock().read_line(&mut output_path)?;
    let output_path = output_path.trim();

    let formatter = get_formatter(&model);
    let formatted = formatter.format(&context, &query);

    if output_path.is_empty() {
        println!("\n--- Output ---\n");
        print!("{}", formatted);
    } else {
        std::fs::write(output_path, &formatted)?;
        println!("Wrote to {}", output_path);
    }

    Ok(())
}
