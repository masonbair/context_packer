# ContextPacker Enhancement Specification

**Tool Location:** To be built at `/home/mason/.cargo/bin/context-packer`
**Purpose:** Smart context assembly for AI agents - the final integration layer that combines all tools
**Position in Ecosystem:** Integration layer using code-index, code-summarizer, and context-query

---

## Problem Statement

AI agents face a critical challenge: **balancing comprehensive context with token limits**.

### Current Issues Without ContextPacker

1. **Manual context selection is inefficient** - Developers guess which files to include
2. **Token budget violations** - Contexts exceed model limits, causing truncation
3. **Suboptimal relevance** - Important code excluded while irrelevant code included
4. **No model-specific optimization** - Same context format for Claude, GPT, Gemini
5. **Redundant re-sending** - Unchanged code re-sent across sessions, wasting tokens

### What ContextPacker Solves

ContextPacker intelligently:
- Selects the most relevant code for a given query/task
- Fits everything within a specified token budget
- Formats output optimally for the target model
- Caches contexts to avoid redundant transmission
- Prioritizes based on relevance, importance, and recency

---

## Core Features

### 1. Token Budget Management (CRITICAL)

**Strict budget enforcement:**
- Accept token budget as parameter (e.g., 8000 tokens)
- Always reserve tokens for architecture summary (200-500)
- Track token usage in real-time as content is added
- Stop adding content when budget is reached
- Report actual usage vs. budget

**Token counting:**
- Support multiple tokenizers (Claude, GPT-4, GPT-3.5)
- Accurate estimation before sending to model
- Account for formatting overhead (markdown syntax, code blocks)

### 2. Relevance Ranking

**Multi-factor scoring:**
- **Query similarity**: How well file matches the query/task
- **Dependency proximity**: Files directly used by relevant files
- **Hotness score**: From code-index metadata
- **Recency**: Recently modified files prioritized
- **Centrality**: Files with many dependencies (hub files)

**Scoring formula:**
```
relevance_score =
  (query_match * 3.0) +      // Highest weight
  (dep_proximity * 2.0) +     // Second priority
  (hotness * 1.5) +           // Complexity/change frequency
  (recency * 1.0) +           // Recent changes matter
  (centrality * 0.5)          // Hub files moderately important
```

### 3. Hierarchical Loading Strategy

**Load order (budget permitting):**
1. **Architecture summary** (200-500 tokens) - Always included
2. **Primary files** - Direct matches to query
3. **Direct dependencies** - Imports of primary files
4. **Callers** - Functions that use primary functions
5. **Related types** - Type definitions used by primary code
6. **Transitive dependencies** - Second-level dependencies if budget allows

### 4. Model-Specific Formatting

**Claude (Anthropic):**
- Use `<thinking>` tags for reasoning sections
- Prefer structured markdown with clear headers
- Include file paths with line numbers
- Code blocks with language hints
- Dependency graphs as markdown trees

**GPT (OpenAI):**
- System/user message separation
- JSON-structured metadata sections
- More compact formatting (GPT prefers density)
- Numbered lists for priorities

**Gemini (Google):**
- More verbose explanations
- Bullet points over numbered lists
- Inline code over large blocks where possible

### 5. Context Caching

**Cache unchanged content:**
- Hash file contents to detect changes
- Store previously packed contexts with hashes
- Reuse cached contexts when files unchanged
- Only re-pack modified files

**Cache invalidation:**
- Detect file changes via git commits or mtime
- Invalidate cache entries for modified files
- Invalidate dependent files (cascade)

### 6. Interactive vs. Batch Modes

**Interactive mode:**
- Prompt user for query/task description
- Ask for token budget
- Select target model
- Show real-time packing progress
- Display final statistics (tokens used, files included)

**Batch mode:**
- Accept all parameters via CLI flags
- Output to file or stdout
- Quiet mode for scripting
- JSON output option for programmatic use

---

## Architecture

### High-Level Workflow

```
Input: Query + Token Budget + Model
  │
  ├─> 1. Load architecture summary (200-500 tokens)
  │      └─> From .ai/context/ARCHITECTURE.md (code-summarizer output)
  │
  ├─> 2. Find relevant files
  │      └─> Use context-query to search for query terms
  │      └─> Use code-index to expand with dependencies
  │
  ├─> 3. Rank files by relevance
  │      └─> Calculate multi-factor scores
  │      └─> Sort descending by score
  │
  ├─> 4. Greedy packing algorithm
  │      └─> Add highest-ranked file
  │      └─> Check if budget exceeded
  │      └─> If yes: stop; if no: continue
  │      └─> Repeat until budget full
  │
  ├─> 5. Format for target model
  │      └─> Apply model-specific template
  │      └─> Add metadata (file paths, line numbers)
  │      └─> Generate dependency graphs
  │
  ├─> 6. Cache the result
  │      └─> Store with file content hashes
  │      └─> Enable reuse if files unchanged
  │
  ▼
Output: Formatted context (markdown/JSON)
```

### Component Breakdown

**1. Token Counter Module:**
- Wraps tiktoken (for GPT) and tokenizers (for Claude)
- Provides unified interface for counting tokens
- Caches token counts for repeated strings
- Accounts for markdown formatting overhead

**2. Query Processor:**
- Parses user query/task description
- Extracts keywords for search
- Determines search strategy (text, structural, hybrid)
- Interfaces with context-query tool

**3. Relevance Ranker:**
- Fetches file metadata from code-index
- Calculates relevance scores
- Sorts files by priority
- Handles tie-breaking (prefer smaller files)

**4. Budget Packer:**
- Implements greedy knapsack algorithm
- Tracks remaining budget in real-time
- Decides when to include full files vs. snippets
- Handles partial file inclusion (key functions only)

**5. Formatter:**
- Model-specific templates (Tera or similar)
- Generates markdown or JSON output
- Adds navigation aids (file:line references)
- Creates dependency visualizations

**6. Cache Manager:**
- Stores packed contexts with metadata
- Checks cache validity (file hashes)
- Implements LRU eviction (max 100MB cache)
- Clears stale entries (>7 days old)

---

## CLI Interface Specification

### Command Structure

```bash
context-packer <SUBCOMMAND> [OPTIONS]
```

### Subcommands

#### 1. Pack Context (Primary Command)

```bash
# Basic usage with query
context-packer pack --query "implement user authentication"

# Specify token budget
context-packer pack --query "fix login bug" --budget 8000

# Specify target model
context-packer pack --query "add caching" --budget 5000 --model claude

# Output to file
context-packer pack --query "refactor database layer" --output context.md

# Focus on specific file
context-packer pack --file src/auth/login.ts --budget 5000

# Include specific context types
context-packer pack --query "optimize search" \
  --include-callers \
  --include-dependencies \
  --include-types
```

**Options:**
- `--query, -q <TEXT>` - Task/query description (required unless --file)
- `--file, -f <PATH>` - Focus on specific file
- `--budget, -b <TOKENS>` - Token budget (default: 8000)
- `--model, -m <MODEL>` - Target model: claude|gpt4|gpt35|gemini (default: claude)
- `--output, -o <PATH>` - Output file (default: stdout)
- `--format <FORMAT>` - Output format: markdown|json (default: markdown)
- `--include-callers` - Include functions that call the relevant code
- `--include-dependencies` - Include imported/used files
- `--include-types` - Include type definitions
- `--depth <N>` - Dependency depth (default: 1, max: 3)

#### 2. Interactive Mode

```bash
# Launch interactive prompt
context-packer interactive

# Prompts:
# - What are you working on? (query)
# - Token budget? (default: 8000)
# - Target model? (default: claude)
# - Include dependencies? (y/n)
# - Include callers? (y/n)
#
# Then displays:
# - Files being included (with scores)
# - Current token usage
# - Final output preview
```

#### 3. Cache Management

```bash
# Show cache statistics
context-packer cache stats
# Output:
# Cache location: ~/.cache/ai-tools/context-packer/
# Total entries: 23
# Total size: 12.4 MB
# Oldest entry: 3 days ago
# Newest entry: 2 hours ago

# Clear cache
context-packer cache clear [--older-than <DAYS>]

# Invalidate cache for specific files
context-packer cache invalidate src/auth/*.ts
```

#### 4. Dry Run / Analysis

```bash
# Show what would be included without generating output
context-packer pack --query "add logging" --dry-run

# Output:
# Query: "add logging"
# Budget: 8000 tokens
# Model: claude
#
# Files to include (ranked):
#   1. src/utils/logger.ts (score: 0.95, 450 tokens)
#   2. src/config/logging.ts (score: 0.87, 320 tokens)
#   3. src/main.ts (score: 0.72, 580 tokens)
#   ...
#
# Total: 7 files, 7,842 tokens (98% of budget)
# Omitted: 12 files (low relevance or budget exceeded)
```

#### 5. Template Management

```bash
# List available templates
context-packer templates list

# Create custom template
context-packer templates create --name my-template --model claude

# Use custom template
context-packer pack --query "task" --template my-template
```

### Global Options

```bash
--config <PATH>      # Custom config file
--project-root <PATH> # Override project root (default: current dir)
--verbose, -v        # Verbose logging
--quiet, -q          # Suppress non-essential output
--no-cache           # Disable cache (always regenerate)
--help, -h           # Show help
--version, -V        # Show version
```

---

## Rust Implementation Guide

### Recommended Crate Dependencies

```toml
[dependencies]
# Core functionality
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.11"

# Token counting
tiktoken-rs = "0.5"              # For GPT models
tokenizers = "0.15"              # For Claude/Gemini models

# Template rendering
tera = "1.19"                    # Jinja2-like templating

# File operations
walkdir = "2.4"
glob = "0.3"
sha2 = "0.10"                    # For content hashing

# Configuration
toml = "0.8"
directories = "5.0"              # For config/cache dirs

# Tool integration
rusqlite = { version = "0.31", features = ["bundled"] }  # For code-index queries

# Process execution (for calling other tools)
tokio = { version = "1.36", features = ["process", "io-util"] }

# Cache management
bincode = "1.3"                  # Binary serialization for cache

[dev-dependencies]
tempfile = "3.10"
assert_cmd = "2.0"
predicates = "3.1"
```

### Module Structure

```
src/
├── main.rs                 # CLI entry point
├── cli.rs                  # Command-line argument parsing
├── config.rs               # Configuration management
├── error.rs                # Custom error types
│
├── tokens/
│   ├── mod.rs              # Token counting interface
│   ├── claude.rs           # Claude token counter
│   ├── gpt.rs              # GPT token counter (tiktoken)
│   └── cache.rs            # Token count caching
│
├── query/
│   ├── mod.rs              # Query processor
│   ├── parser.rs           # Parse user query
│   ├── executor.rs         # Execute via context-query tool
│   └── expander.rs         # Expand with dependencies
│
├── rank/
│   ├── mod.rs              # Relevance ranker
│   ├── scorer.rs           # Multi-factor scoring
│   ├── metadata.rs         # Fetch from code-index
│   └── sort.rs             # Sorting and prioritization
│
├── pack/
│   ├── mod.rs              # Context packer
│   ├── budget.rs           # Token budget management
│   ├── greedy.rs           # Greedy packing algorithm
│   ├── selector.rs         # File/snippet selection
│   └── partial.rs          # Partial file inclusion logic
│
├── format/
│   ├── mod.rs              # Formatter interface
│   ├── claude.rs           # Claude-specific formatting
│   ├── gpt.rs              # GPT formatting
│   ├── gemini.rs           # Gemini formatting
│   ├── json.rs             # JSON output
│   └── templates/          # Tera templates
│       ├── claude.md.tera
│       ├── gpt.md.tera
│       └── gemini.md.tera
│
├── cache/
│   ├── mod.rs              # Cache manager
│   ├── storage.rs          # File-based cache storage
│   ├── hash.rs             # Content hashing
│   └── invalidation.rs     # Cache invalidation logic
│
├── tools/
│   ├── mod.rs              # Tool integration
│   ├── code_index.rs       # code-index client
│   ├── context_query.rs    # context-query client
│   └── code_summarizer.rs  # code-summarizer client
│
└── utils.rs                # Utility functions

templates/                  # Built-in templates
├── claude.md.tera
├── gpt.md.tera
└── gemini.md.tera

tests/
├── integration_test.rs
├── packing_test.rs
├── ranking_test.rs
└── token_test.rs
```

### Key Implementation Details

#### 1. Token Counter (`src/tokens/mod.rs`)

```rust
use anyhow::Result;

pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> Result<usize>;
    fn count_with_overhead(&self, text: &str, format: FormatType) -> Result<usize>;
}

pub struct ClaudeTokenCounter {
    tokenizer: tokenizers::Tokenizer,
}

impl ClaudeTokenCounter {
    pub fn new() -> Result<Self> {
        let tokenizer = tokenizers::Tokenizer::from_pretrained("anthropic/claude", None)?;
        Ok(Self { tokenizer })
    }
}

impl TokenCounter for ClaudeTokenCounter {
    fn count(&self, text: &str) -> Result<usize> {
        let encoding = self.tokenizer.encode(text, false)?;
        Ok(encoding.get_ids().len())
    }

    fn count_with_overhead(&self, text: &str, format: FormatType) -> Result<usize> {
        let base_count = self.count(text)?;
        let overhead = match format {
            FormatType::CodeBlock => 10,  // ``` markers + language hint
            FormatType::Section => 5,     // # headers
            FormatType::Plain => 0,
        };
        Ok(base_count + overhead)
    }
}

pub struct GptTokenCounter {
    bpe: tiktoken_rs::CoreBPE,
}

impl GptTokenCounter {
    pub fn new(model: &str) -> Result<Self> {
        let bpe = tiktoken_rs::get_bpe_from_model(model)?;
        Ok(Self { bpe })
    }
}

impl TokenCounter for GptTokenCounter {
    fn count(&self, text: &str) -> Result<usize> {
        Ok(self.bpe.encode_with_special_tokens(text).len())
    }
}

pub fn get_counter(model: &str) -> Result<Box<dyn TokenCounter>> {
    match model {
        "claude" => Ok(Box::new(ClaudeTokenCounter::new()?)),
        "gpt4" | "gpt35" => Ok(Box::new(GptTokenCounter::new(model)?)),
        _ => Err(anyhow::anyhow!("Unsupported model: {}", model)),
    }
}
```

#### 2. Budget Manager (`src/pack/budget.rs`)

```rust
use crate::tokens::TokenCounter;

pub struct TokenBudget {
    total: usize,
    used: usize,
    reserved: usize,  // For architecture summary
    counter: Box<dyn TokenCounter>,
}

impl TokenBudget {
    pub fn new(total: usize, counter: Box<dyn TokenCounter>) -> Self {
        Self {
            total,
            used: 0,
            reserved: 500,  // Reserve for architecture
            counter,
        }
    }

    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.used)
    }

    pub fn can_fit(&self, content: &str) -> Result<bool> {
        let tokens = self.counter.count(content)?;
        Ok(self.used + tokens + self.reserved <= self.total)
    }

    pub fn add(&mut self, content: &str) -> Result<usize> {
        let tokens = self.counter.count(content)?;
        if self.used + tokens + self.reserved > self.total {
            return Err(anyhow::anyhow!("Budget exceeded"));
        }
        self.used += tokens;
        Ok(tokens)
    }

    pub fn add_reserved(&mut self, content: &str) -> Result<usize> {
        let tokens = self.counter.count(content)?;
        self.reserved = self.reserved.saturating_sub(tokens);
        self.used += tokens;
        Ok(tokens)
    }

    pub fn usage_percent(&self) -> f64 {
        (self.used as f64 / self.total as f64) * 100.0
    }
}
```

#### 3. Relevance Scorer (`src/rank/scorer.rs`)

```rust
use crate::tools::CodeIndexClient;

#[derive(Debug, Clone)]
pub struct FileScore {
    pub path: PathBuf,
    pub score: f64,
    pub query_match: f64,
    pub dep_proximity: f64,
    pub hotness: f64,
    pub recency: f64,
    pub centrality: f64,
}

pub struct RelevanceScorer {
    index_client: CodeIndexClient,
}

impl RelevanceScorer {
    pub fn new(index_client: CodeIndexClient) -> Self {
        Self { index_client }
    }

    pub fn score_files(
        &self,
        files: &[PathBuf],
        query: &str,
        primary_files: &[PathBuf],
    ) -> Result<Vec<FileScore>> {
        let mut scores = Vec::new();

        for file in files {
            let query_match = self.calculate_query_match(file, query)?;
            let dep_proximity = self.calculate_dep_proximity(file, primary_files)?;
            let hotness = self.get_hotness(file)?;
            let recency = self.calculate_recency(file)?;
            let centrality = self.calculate_centrality(file)?;

            // Weighted scoring formula
            let total_score =
                (query_match * 3.0) +
                (dep_proximity * 2.0) +
                (hotness * 1.5) +
                (recency * 1.0) +
                (centrality * 0.5);

            scores.push(FileScore {
                path: file.clone(),
                score: total_score,
                query_match,
                dep_proximity,
                hotness,
                recency,
                centrality,
            });
        }

        // Sort descending by score
        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(scores)
    }

    fn calculate_query_match(&self, file: &Path, query: &str) -> Result<f64> {
        // Use context-query to check how well file matches query
        // Return score 0.0-1.0
        Ok(0.5) // Placeholder
    }

    fn calculate_dep_proximity(&self, file: &Path, primary: &[PathBuf]) -> Result<f64> {
        // Check if file is imported by or imports primary files
        // Direct dependency: 1.0
        // Transitive: 0.5
        // No relation: 0.0
        let deps = self.index_client.get_dependencies(file)?;
        for dep in deps {
            if primary.contains(&dep) {
                return Ok(1.0);
            }
        }
        Ok(0.0)
    }

    fn get_hotness(&self, file: &Path) -> Result<f64> {
        // Query code-index for hotness score
        let metadata = self.index_client.get_file_metadata(file)?;
        Ok(metadata.hotness_score / 100.0)  // Normalize to 0-1
    }

    fn calculate_recency(&self, file: &Path) -> Result<f64> {
        // Files modified recently get higher scores
        let metadata = std::fs::metadata(file)?;
        let modified = metadata.modified()?;
        let age = SystemTime::now().duration_since(modified)?.as_secs();
        let days = age / 86400;

        // Exponential decay: fresh files = 1.0, old files approach 0.0
        Ok((-days as f64 / 30.0).exp())
    }

    fn calculate_centrality(&self, file: &Path) -> Result<f64> {
        // Files with many dependencies are "hub" files
        let deps = self.index_client.get_dependencies(file)?;
        let callers = self.index_client.get_callers(file)?;
        let total = deps.len() + callers.len();

        // Normalize: 0 deps = 0.0, 20+ deps = 1.0
        Ok((total as f64 / 20.0).min(1.0))
    }
}
```

#### 4. Greedy Packer (`src/pack/greedy.rs`)

```rust
use crate::pack::budget::TokenBudget;
use crate::rank::FileScore;

pub struct ContextPacker {
    budget: TokenBudget,
}

impl ContextPacker {
    pub fn new(budget: TokenBudget) -> Self {
        Self { budget }
    }

    pub fn pack(&mut self, scored_files: Vec<FileScore>) -> Result<PackedContext> {
        let mut included = Vec::new();
        let mut omitted = Vec::new();

        // Reserve budget for architecture summary
        let arch_summary = self.load_architecture_summary()?;
        self.budget.add_reserved(&arch_summary)?;

        // Greedy selection: add files by score until budget full
        for file_score in scored_files {
            let content = std::fs::read_to_string(&file_score.path)?;

            if self.budget.can_fit(&content)? {
                let tokens = self.budget.add(&content)?;
                included.push(IncludedFile {
                    path: file_score.path.clone(),
                    content,
                    score: file_score.score,
                    tokens,
                });
            } else {
                // Try partial inclusion (key functions only)
                if let Some(partial) = self.try_partial_inclusion(&file_score, &content)? {
                    let tokens = self.budget.add(&partial.content)?;
                    included.push(partial);
                } else {
                    omitted.push(file_score);
                }
            }

            if self.budget.remaining() < 100 {
                // Less than 100 tokens left, stop
                break;
            }
        }

        Ok(PackedContext {
            architecture_summary: arch_summary,
            included_files: included,
            omitted_files: omitted,
            tokens_used: self.budget.used,
            tokens_budget: self.budget.total,
        })
    }

    fn load_architecture_summary(&self) -> Result<String> {
        // Load from .ai/context/ARCHITECTURE.md (code-summarizer output)
        let arch_path = Path::new(".ai/context/ARCHITECTURE.md");
        if arch_path.exists() {
            Ok(std::fs::read_to_string(arch_path)?)
        } else {
            Ok("No architecture summary available.".to_string())
        }
    }

    fn try_partial_inclusion(
        &self,
        file_score: &FileScore,
        full_content: &str,
    ) -> Result<Option<IncludedFile>> {
        // Extract only the most relevant functions/classes
        // Use code-index to identify key symbols in this file
        // Return partial content if it fits in budget
        // This is a simplified placeholder
        Ok(None)
    }
}

#[derive(Debug)]
pub struct PackedContext {
    pub architecture_summary: String,
    pub included_files: Vec<IncludedFile>,
    pub omitted_files: Vec<FileScore>,
    pub tokens_used: usize,
    pub tokens_budget: usize,
}

#[derive(Debug)]
pub struct IncludedFile {
    pub path: PathBuf,
    pub content: String,
    pub score: f64,
    pub tokens: usize,
}
```

#### 5. Claude Formatter (`src/format/claude.rs`)

```rust
use tera::{Tera, Context};
use crate::pack::PackedContext;

pub struct ClaudeFormatter {
    tera: Tera,
}

impl ClaudeFormatter {
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "claude",
            include_str!("../templates/claude.md.tera")
        )?;
        Ok(Self { tera })
    }

    pub fn format(&self, packed: &PackedContext, query: &str) -> Result<String> {
        let mut context = Context::new();
        context.insert("query", query);
        context.insert("architecture", &packed.architecture_summary);
        context.insert("files", &packed.included_files);
        context.insert("omitted", &packed.omitted_files);
        context.insert("tokens_used", &packed.tokens_used);
        context.insert("tokens_budget", &packed.tokens_budget);
        context.insert("usage_percent", &((packed.tokens_used as f64 / packed.tokens_budget as f64) * 100.0));

        let rendered = self.tera.render("claude", &context)?;
        Ok(rendered)
    }
}
```

#### 6. Cache Manager (`src/cache/storage.rs`)

```rust
use sha2::{Sha256, Digest};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub query: String,
    pub model: String,
    pub budget: usize,
    pub file_hashes: HashMap<PathBuf, String>,  // path -> content hash
    pub packed_context: String,
    pub created_at: i64,
}

pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find cache directory"))?
            .join("ai-tools")
            .join("context-packer");

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir })
    }

    pub fn get(&self, query: &str, model: &str, budget: usize) -> Result<Option<String>> {
        let cache_key = self.make_cache_key(query, model, budget);
        let cache_path = self.cache_dir.join(&cache_key);

        if !cache_path.exists() {
            return Ok(None);
        }

        let entry: CacheEntry = bincode::deserialize(&std::fs::read(&cache_path)?)?;

        // Validate that files haven't changed
        if self.validate_hashes(&entry.file_hashes)? {
            Ok(Some(entry.packed_context))
        } else {
            // Cache invalid, delete it
            std::fs::remove_file(&cache_path)?;
            Ok(None)
        }
    }

    pub fn store(&self, entry: CacheEntry) -> Result<()> {
        let cache_key = self.make_cache_key(&entry.query, &entry.model, entry.budget);
        let cache_path = self.cache_dir.join(&cache_key);

        let serialized = bincode::serialize(&entry)?;
        std::fs::write(&cache_path, serialized)?;

        Ok(())
    }

    fn make_cache_key(&self, query: &str, model: &str, budget: usize) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query);
        hasher.update(model);
        hasher.update(budget.to_string());
        format!("{:x}.cache", hasher.finalize())
    }

    fn validate_hashes(&self, hashes: &HashMap<PathBuf, String>) -> Result<bool> {
        for (path, expected_hash) in hashes {
            if !path.exists() {
                return Ok(false);
            }

            let content = std::fs::read_to_string(path)?;
            let actual_hash = self.hash_content(&content);

            if actual_hash != *expected_hash {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn hash_content(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    pub fn clear(&self, older_than_days: Option<u64>) -> Result<usize> {
        let mut deleted = 0;
        let now = chrono::Utc::now().timestamp();

        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(days) = older_than_days {
                let metadata = std::fs::metadata(&path)?;
                let modified = metadata.modified()?;
                let age_secs = SystemTime::now().duration_since(modified)?.as_secs();
                let age_days = age_secs / 86400;

                if age_days > days {
                    std::fs::remove_file(&path)?;
                    deleted += 1;
                }
            } else {
                std::fs::remove_file(&path)?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}
```

---

## Templates

### Claude Template (`templates/claude.md.tera`)

```markdown
# Context: {{ query }}

**Token Budget:** {{ tokens_budget }} / {{ tokens_used }} used ({{ usage_percent | round }}%)
**Model:** Claude
**Generated:** {{ now() | date(format="%Y-%m-%d %H:%M") }}

---

## Architecture Overview

{{ architecture }}

---

## Relevant Code

{% for file in files %}
### {{ file.path }} (Priority: {{ file.score | round(precision=2) }}, {{ file.tokens }} tokens)

```{{ file.path | file_extension }}
{{ file.content }}
```

---

{% endfor %}

{% if omitted | length > 0 %}
## Omitted Files (Low Relevance or Budget Exceeded)

{% for file in omitted %}
- `{{ file.path }}` (score: {{ file.score | round(precision=2) }})
{% endfor %}

*Run with higher budget (--budget) to include more files.*
{% endif %}

---

**Context Generation Complete**
- Files included: {{ files | length }}
- Files omitted: {{ omitted | length }}
- Token efficiency: {{ usage_percent | round }}%
```

---

## Tool Integration

### Integration with code-index

```rust
pub struct CodeIndexClient {
    db_path: PathBuf,
}

impl CodeIndexClient {
    pub fn new(project_root: &Path) -> Result<Self> {
        let db_path = project_root
            .join(".ai")
            .join("code-index.db")
            .or_else(|| dirs::cache_dir()?.join("ai-tools/code-index.db"));

        Ok(Self { db_path })
    }

    pub fn get_dependencies(&self, file: &Path) -> Result<Vec<PathBuf>> {
        // Query SQLite database
        // SELECT target_file FROM dependencies WHERE source_file = ?
        Ok(vec![])
    }

    pub fn get_callers(&self, file: &Path) -> Result<Vec<PathBuf>> {
        // Query for files that call functions in this file
        Ok(vec![])
    }

    pub fn get_file_metadata(&self, file: &Path) -> Result<FileMetadata> {
        // SELECT * FROM files WHERE path = ?
        Ok(FileMetadata::default())
    }
}
```

### Integration with context-query

```rust
pub struct ContextQueryClient;

impl ContextQueryClient {
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Execute: context-query --text "query" --format json
        let output = std::process::Command::new("context-query")
            .args(&["--text", query, "--format", "json"])
            .output()?;

        let results: Vec<SearchResult> = serde_json::from_slice(&output.stdout)?;
        Ok(results)
    }
}
```

### Integration with code-summarizer

```rust
pub fn load_architecture_summary(project_root: &Path) -> Result<String> {
    let arch_path = project_root.join(".ai/context/ARCHITECTURE.md");

    if !arch_path.exists() {
        // Try to generate it
        std::process::Command::new("code-summarizer")
            .args(&["generate", "--tier", "minimal"])
            .current_dir(project_root)
            .output()?;
    }

    Ok(std::fs::read_to_string(arch_path)?)
}
```

---

## Configuration

Default config: `~/.config/ai-tools/config.toml`

```toml
[context-packer]
default_budget = 8000
default_model = "claude"
cache_dir = "~/.cache/ai-tools/context-packer"
cache_max_size_mb = 100
cache_max_age_days = 7

[context-packer.models]
claude = { tokenizer = "anthropic/claude", max_tokens = 200000 }
gpt4 = { tokenizer = "gpt-4", max_tokens = 128000 }
gpt35 = { tokenizer = "gpt-3.5-turbo", max_tokens = 16000 }
gemini = { tokenizer = "google/gemini", max_tokens = 1000000 }

[context-packer.ranking]
query_match_weight = 3.0
dep_proximity_weight = 2.0
hotness_weight = 1.5
recency_weight = 1.0
centrality_weight = 0.5

[context-packer.packing]
reserve_for_architecture = 500  # Always reserve tokens
min_remaining_to_continue = 100 # Stop if less than this
enable_partial_files = true     # Try to include key functions only
max_dependency_depth = 3

[context-packer.cache]
enabled = true
validate_on_load = true         # Check file hashes
max_entries = 100
```

---

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let counter = ClaudeTokenCounter::new().unwrap();
        let text = "Hello, world!";
        let count = counter.count(text).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_budget_management() {
        let counter = Box::new(ClaudeTokenCounter::new().unwrap());
        let mut budget = TokenBudget::new(1000, counter);

        let content = "x".repeat(500);  // ~500 tokens
        assert!(budget.can_fit(&content).unwrap());

        budget.add(&content).unwrap();
        assert!(budget.remaining() < 1000);
    }

    #[test]
    fn test_relevance_scoring() {
        let scorer = RelevanceScorer::new(mock_index_client());
        let files = vec![PathBuf::from("src/main.rs")];
        let scores = scorer.score_files(&files, "authentication", &[]).unwrap();

        assert_eq!(scores.len(), 1);
        assert!(scores[0].score >= 0.0);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = CacheManager::new().unwrap();

        let mut hashes = HashMap::new();
        hashes.insert(PathBuf::from("test.txt"), "abc123".to_string());

        // Should fail if file doesn't exist
        assert!(!cache.validate_hashes(&hashes).unwrap());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_packing_workflow() {
    // 1. Create temp directory with test files
    let temp_dir = tempfile::tempdir().unwrap();
    create_test_project(&temp_dir);

    // 2. Run context-packer
    let output = std::process::Command::new("context-packer")
        .args(&[
            "pack",
            "--query", "test query",
            "--budget", "5000",
            "--project-root", temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    // 3. Verify output structure
    let result = String::from_utf8(output.stdout).unwrap();
    assert!(result.contains("# Context:"));
    assert!(result.contains("## Architecture Overview"));
    assert!(result.contains("## Relevant Code"));

    // 4. Verify token budget respected
    // Parse "X / Y used" from output
    // Assert X <= Y
}

#[test]
fn test_caching() {
    // 1. Run once
    let output1 = run_packer("test query");

    // 2. Run again with same query (should use cache)
    let start = Instant::now();
    let output2 = run_packer("test query");
    let duration = start.elapsed();

    // Should be much faster (< 100ms)
    assert!(duration.as_millis() < 100);
    assert_eq!(output1, output2);

    // 3. Modify a file
    modify_file("src/main.rs");

    // 4. Run again (should regenerate)
    let output3 = run_packer("test query");
    // Output might differ due to changes
}
```

### Performance Benchmarks

- Pack context for 500-file project: < 3 seconds
- Cache hit: < 50ms
- Token counting: < 10ms per file
- Relevance scoring: < 500ms for 100 files

---

## Best Practices & Industry Standards

### Error Handling
- Never use `.unwrap()` in production code
- Use `anyhow::Result` for error propagation
- Use `thiserror` for custom error types
- Provide helpful error messages with context
- Log errors with `log::error!` before returning

### Code Quality
- **DRY**: Extract common patterns into functions
- **SOLID**: Single responsibility, dependency injection
- **Testability**: Mock external dependencies (code-index, file system)
- **Documentation**: Add `///` docs for public APIs
- **Type Safety**: Leverage Rust's type system for correctness

### Performance Optimization
- **Cache aggressively**: Token counts, file contents, scores
- **Lazy loading**: Only read files when needed
- **Parallel processing**: Use rayon for scoring multiple files
- **Efficient data structures**: HashMap for O(1) lookups
- **Profile before optimizing**: Use `cargo flamegraph`

### Security Considerations
- **Path validation**: Prevent directory traversal attacks
- **File size limits**: Reject files > 10MB
- **Token limit enforcement**: Never exceed model limits
- **Cache permissions**: 0600 for cache files
- **Input sanitization**: Validate query strings

### Logging Best Practices

```rust
// Error - Critical failures
log::error!("Failed to load architecture summary: {}", e);

// Warn - Recoverable issues
log::warn!("code-index not available, using basic scoring");

// Info - High-level operations
log::info!("Packing context for query: '{}'", query);
log::info!("Packed {} files, {} tokens used", files.len(), tokens);

// Debug - Detailed flow
log::debug!("Scored file {} with score {}", path.display(), score);

// Trace - Very verbose
log::trace!("Token count for {}: {}", file, count);
```

### Code Style

```rust
// Good: Clear, descriptive names
fn calculate_relevance_score(file: &Path, query: &str) -> Result<f64> { }

// Bad: Abbreviations, unclear
fn calc_rel(f: &Path, q: &str) -> Result<f64> { }

// Good: Small, focused functions
fn load_file_content(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

fn count_tokens(content: &str, counter: &dyn TokenCounter) -> Result<usize> {
    counter.count(content)
}

// Bad: Large, multi-purpose functions
fn process_everything(path: &Path) -> Result<(String, usize, f64)> {
    // 100 lines of mixed concerns
}
```

---

## Success Criteria

After implementation, context-packer should:

- [ ] Accept query and budget, generate optimized context
- [ ] Respect token budget (never exceed)
- [ ] Include architecture summary automatically
- [ ] Rank files by relevance accurately
- [ ] Support Claude, GPT, Gemini models
- [ ] Cache results for unchanged files
- [ ] Integrate with code-index, context-query, code-summarizer
- [ ] Provide interactive mode
- [ ] Generate markdown and JSON output
- [ ] Handle edge cases (no files, budget too small, etc.)
- [ ] All tests pass with >80% coverage
- [ ] Performance benchmarks met
- [ ] Documentation complete
- [ ] Ready for use by AI agents

---

## Development Phases

### Phase 1: Core Functionality (Week 1)
1. Token counting (Claude, GPT)
2. Budget management
3. Basic file selection (no ranking)
4. Simple markdown output

### Phase 2: Ranking & Intelligence (Week 2)
1. Relevance scoring algorithm
2. Integration with code-index
3. Integration with context-query
4. Hierarchical loading strategy

### Phase 3: Formatting & Caching (Week 3)
1. Model-specific formatters
2. Template system (Tera)
3. Cache manager
4. Cache invalidation

### Phase 4: Polish & Testing (Week 4)
1. Interactive mode
2. Dry-run analysis
3. Comprehensive tests
4. Documentation
5. CLI refinement

---

## Integration Example: AI Agent Workflow

```bash
# Agent receives task: "Optimize the search functionality"

# 1. Agent calls context-packer
context-packer pack \
  --query "optimize search functionality performance" \
  --budget 8000 \
  --model claude \
  --include-dependencies \
  --output /tmp/context.md

# 2. Context packer workflow:
#    - Loads architecture summary (200 tokens)
#    - Searches for "search" via context-query
#    - Finds: src/search/mod.rs, src/search/text.rs, etc.
#    - Scores files by relevance
#    - Includes dependencies: src/index/client.rs
#    - Packs within 8000 token budget
#    - Formats for Claude

# 3. Agent reads /tmp/context.md (7,842 tokens)
#    - Understands search architecture
#    - Sees current implementation
#    - Identifies bottlenecks
#    - Has full context to make optimization decisions

# 4. Agent proceeds with optimizations, context-efficient workflow
```

---

## Next Steps After Implementation

1. **Test with real projects** - Verify token savings and relevance accuracy
2. **Optimize ranking algorithm** - Tune weights based on empirical results
3. **Add more models** - Support additional AI models (Mistral, Llama, etc.)
4. **Build feedback loop** - Allow users to rate context quality
5. **Integration with IDEs** - VS Code extension, Neovim plugin
6. **Cloud service** - Optional hosted version for teams

---

**This specification provides everything needed to build context-packer.** Focus on core token management and packing first, then add ranking and caching. This is the capstone tool that brings the entire ecosystem together.
