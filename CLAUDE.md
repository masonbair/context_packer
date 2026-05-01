# context-packer - Smart Context Assembly for AI Agents

## Project Overview

The final integration layer that assembles relevant code context within token budgets, optimized for specific AI models. Combines code-index, code-summarizer, and context-query to create the perfect context for any AI task.

**Primary Languages:** Rust
**Project Type:** CLI tool - Integration layer
**Created:** 2026-04-30
**Target Platform:** Arch Linux (portable to other Linux distros)

---

## Purpose & Problem Statement

### Core Problem Solved
AI agents need to balance comprehensive context with strict token limits. Manually selecting files is inefficient and often includes irrelevant code while excluding important dependencies. ContextPacker uses intelligent relevance ranking and greedy packing algorithms to maximize useful information per token.

### Key Value Propositions
- **Token budget management**: Strict enforcement, never exceeds limits
- **Relevance prioritization**: Multi-factor scoring (query match, dependencies, hotness)
- **Model-specific formatting**: Optimized output for Claude, GPT, Gemini
- **Hierarchical loading**: Architecture summary → Primary files → Dependencies
- **Caching**: Avoids re-sending unchanged code across sessions
- **Integration layer**: Combines all AI agent tools into one workflow

---

## Position in Ecosystem

ContextPacker is the **capstone tool** that brings everything together:

```
┌──────────────┐
│  code-index  │ ← Provides: Symbol lookup, dependencies, hotness scores
└──────┬───────┘
       │
       ├──> ┌────────────────┐
       │    │code-summarizer │ ← Provides: Architecture summaries
       │    └────────┬───────┘
       │             │
       ├─────────────┼──> ┌───────────────┐
       │             │    │context-query  │ ← Provides: Relevant file search
       │             │    └───────┬───────┘
       │             │            │
       ▼             ▼            ▼
   ┌─────────────────────────────────┐
   │      context-packer             │ ← Combines all tools
   │  (Smart context assembly)       │
   └─────────────────┬───────────────┘
                     │
                     ▼
              AI Agent receives
              optimized context
```

---

## Required Features

### 1. Token Budget Management (CRITICAL)

**Strict enforcement:**
- Accept token budget parameter (default: 8000)
- Always reserve 200-500 tokens for architecture summary
- Track usage in real-time as content added
- Stop when budget reached or <100 tokens remain
- Report actual usage vs. budget

**Multi-model token counting:**
- **Claude**: Use `tokenizers` crate with Anthropic tokenizer
- **GPT-4/3.5**: Use `tiktoken-rs` with OpenAI BPE
- **Gemini**: Use Google's tokenizer API
- Account for markdown formatting overhead

### 2. Relevance Ranking System

**Multi-factor scoring formula:**
```
score = (query_match * 3.0) +
        (dep_proximity * 2.0) +
        (hotness * 1.5) +
        (recency * 1.0) +
        (centrality * 0.5)
```

**Factors:**
- **query_match** (0.0-1.0): How well file content matches query
- **dep_proximity** (0.0-1.0): Direct dependency=1.0, transitive=0.5
- **hotness** (0.0-1.0): From code-index (complexity + change frequency)
- **recency** (0.0-1.0): Exponential decay based on last modified
- **centrality** (0.0-1.0): Files with many connections (hubs)

### 3. Hierarchical Loading Strategy

**Priority order (within budget):**
1. **Architecture summary** (200-500 tokens) - ALWAYS included
2. **Primary files** - Top ranked by query match
3. **Direct dependencies** - Files imported by primary files
4. **Callers** - Functions/files that use primary code
5. **Type definitions** - Structs/types used by primary code
6. **Transitive deps** - Second-level dependencies (if budget allows)

### 4. Model-Specific Formatting

**Claude (Anthropic):**
```markdown
# Context: {query}

**Token Budget:** {budget} / {used} used ({percent}%)

---

## Architecture Overview
{architecture_summary}

---

## Relevant Code

### src/file.rs (Priority: HIGH, 450 tokens)
\`\`\`rust
{content}
\`\`\`

**Dependencies:**
- src/dep1.rs (included below)
- src/dep2.rs (omitted: low relevance)

**Called by:**
- src/caller.rs:23
```

**GPT (OpenAI):**
- More compact formatting
- JSON metadata sections
- Numbered priority lists
- Shorter explanatory text

**Gemini (Google):**
- More verbose explanations
- Bullet points over numbered lists
- Inline code snippets where possible

### 5. Context Caching

**Cache strategy:**
- Hash file contents (SHA-256)
- Store packed contexts with file hashes
- Validate hashes on cache load
- Invalidate if any file changed
- LRU eviction (max 100MB cache)
- Auto-clean entries >7 days old

### 6. Interactive Mode

```bash
context-packer interactive

# Prompts user for:
# 1. What are you working on? (query)
# 2. Token budget? (default: 8000)
# 3. Target model? (default: claude)
# 4. Include dependencies? (y/n)
# 5. Include callers? (y/n)
#
# Shows progress:
# - Files being scored
# - Files selected (with scores)
# - Current token usage
# - Final output preview
```

---

## Architecture

### Workflow Diagram

```
Input: Query + Budget + Model
  │
  ├─> Load architecture summary (reserve 200-500 tokens)
  │
  ├─> Find relevant files
  │   ├─> Use context-query for text search
  │   └─> Use code-index for dependencies
  │
  ├─> Score files (multi-factor)
  │   ├─> Query match score
  │   ├─> Dependency proximity
  │   ├─> Hotness from code-index
  │   ├─> Recency from file mtime
  │   └─> Centrality from dep graph
  │
  ├─> Sort by score (descending)
  │
  ├─> Greedy packing
  │   ├─> Add highest scored file
  │   ├─> Count tokens
  │   ├─> If exceeds budget: try partial or skip
  │   ├─> Continue until budget full
  │   └─> Track included & omitted files
  │
  ├─> Format for model
  │   ├─> Apply model-specific template
  │   ├─> Add metadata (paths, line numbers)
  │   └─> Generate dependency graphs
  │
  ├─> Cache result
  │   └─> Store with file content hashes
  │
  ▼
Output: Optimized context (markdown/JSON)
```

### Component Modules

**1. Token Counter** (`src/tokens/`)
- Abstraction over multiple tokenizers
- Claude: tokenizers crate
- GPT: tiktoken-rs
- Caching for performance

**2. Query Processor** (`src/query/`)
- Parses user query
- Extracts search terms
- Calls context-query tool
- Expands with dependencies via code-index

**3. Relevance Ranker** (`src/rank/`)
- Implements scoring algorithm
- Fetches metadata from code-index
- Calculates all factors
- Sorts files by priority

**4. Budget Packer** (`src/pack/`)
- Manages token budget
- Greedy knapsack algorithm
- Partial file inclusion logic
- Tracks included/omitted files

**5. Formatter** (`src/format/`)
- Model-specific templates (Tera)
- Markdown generation
- JSON output
- Dependency graph visualization

**6. Cache Manager** (`src/cache/`)
- File content hashing (SHA-256)
- Cache storage (bincode serialization)
- Validation on load
- LRU eviction policy

**7. Tool Integration** (`src/tools/`)
- code-index client (SQLite queries)
- context-query client (subprocess execution)
- code-summarizer integration (file reading)

---

## CLI Interface

### Command Structure

```bash
context-packer <SUBCOMMAND> [OPTIONS]
```

### Primary Commands

#### 1. Pack Context

```bash
# Basic usage
context-packer pack --query "implement authentication"

# With budget and model
context-packer pack \
  --query "optimize search performance" \
  --budget 8000 \
  --model claude

# Focus on specific file
context-packer pack \
  --file src/auth/login.ts \
  --include-dependencies \
  --include-callers

# Output to file
context-packer pack \
  --query "refactor database layer" \
  --output context.md \
  --format markdown
```

**Options:**
- `--query, -q <TEXT>` - Task/query description
- `--file, -f <PATH>` - Focus on specific file
- `--budget, -b <N>` - Token budget (default: 8000)
- `--model, -m <MODEL>` - claude|gpt4|gpt35|gemini (default: claude)
- `--output, -o <PATH>` - Output file (default: stdout)
- `--format <FMT>` - markdown|json (default: markdown)
- `--include-dependencies` - Include imported files
- `--include-callers` - Include calling code
- `--include-types` - Include type definitions
- `--depth <N>` - Dependency depth (1-3, default: 1)

#### 2. Interactive Mode

```bash
context-packer interactive
```

#### 3. Dry Run Analysis

```bash
# Preview what would be included
context-packer pack \
  --query "add logging" \
  --dry-run

# Output:
# Query: "add logging"
# Budget: 8000 tokens
# Model: claude
#
# Files ranked by relevance:
#   1. src/utils/logger.ts      (score: 0.95, 450 tokens) ✓
#   2. src/config/logging.ts    (score: 0.87, 320 tokens) ✓
#   3. src/main.ts              (score: 0.72, 580 tokens) ✓
#   4. src/services/api.ts      (score: 0.65, 720 tokens) ✓
#   ...
#   15. tests/logger.test.ts    (score: 0.23, 890 tokens) ✗ (budget limit)
#
# Total: 7 files included, 7,842 tokens (98% of budget)
# Omitted: 8 files (low relevance or budget exceeded)
```

#### 4. Cache Management

```bash
# Cache statistics
context-packer cache stats

# Clear cache
context-packer cache clear

# Clear old entries
context-packer cache clear --older-than 7

# Invalidate specific patterns
context-packer cache invalidate "src/auth/*.ts"
```

### Global Options

```bash
--config <PATH>         # Custom config file
--project-root <PATH>   # Project root (default: current dir)
--verbose, -v           # Verbose logging
--quiet, -q             # Quiet mode (errors only)
--no-cache              # Disable caching
--help, -h              # Show help
--version, -V           # Show version
```

---

## Rust Implementation Guide

### Dependencies (Cargo.toml)

```toml
[dependencies]
# CLI & Core
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.11"

# Token counting
tiktoken-rs = "0.5"              # GPT tokenizer
tokenizers = "0.15"              # Claude/Gemini tokenizer

# Templating
tera = "1.19"

# File operations
walkdir = "2.4"
glob = "0.3"
sha2 = "0.10"                    # Content hashing

# Configuration
toml = "0.8"
directories = "5.0"

# Tool integration
rusqlite = { version = "0.31", features = ["bundled"] }
tokio = { version = "1.36", features = ["process", "io-util"] }

# Cache
bincode = "1.3"
chrono = "0.4"

[dev-dependencies]
tempfile = "3.10"
assert_cmd = "2.0"
predicates = "3.1"
```

### Module Structure

```
src/
├── main.rs                 # CLI entry, command routing
├── cli.rs                  # Argument parsing (clap)
├── config.rs               # Configuration management
├── error.rs                # Custom error types
│
├── tokens/
│   ├── mod.rs              # TokenCounter trait
│   ├── claude.rs           # Claude tokenizer
│   ├── gpt.rs              # GPT tokenizer (tiktoken)
│   ├── gemini.rs           # Gemini tokenizer
│   └── cache.rs            # Token count caching
│
├── query/
│   ├── mod.rs              # Query processor
│   ├── parser.rs           # Parse query string
│   ├── executor.rs         # Execute via context-query
│   └── expander.rs         # Expand with dependencies
│
├── rank/
│   ├── mod.rs              # Relevance ranker
│   ├── scorer.rs           # Multi-factor scoring
│   ├── metadata.rs         # Fetch from code-index
│   └── factors.rs          # Individual scoring factors
│
├── pack/
│   ├── mod.rs              # Context packer
│   ├── budget.rs           # Token budget tracker
│   ├── greedy.rs           # Greedy packing algorithm
│   ├── selector.rs         # File selection logic
│   └── partial.rs          # Partial file inclusion
│
├── format/
│   ├── mod.rs              # Formatter trait
│   ├── claude.rs           # Claude formatter
│   ├── gpt.rs              # GPT formatter
│   ├── gemini.rs           # Gemini formatter
│   ├── json.rs             # JSON output
│   └── utils.rs            # Formatting helpers
│
├── cache/
│   ├── mod.rs              # Cache manager
│   ├── storage.rs          # File-based storage
│   ├── hash.rs             # Content hashing (SHA-256)
│   ├── validation.rs       # Cache validation
│   └── eviction.rs         # LRU eviction
│
├── tools/
│   ├── mod.rs              # Tool integration
│   ├── code_index.rs       # code-index client
│   ├── context_query.rs    # context-query client
│   └── code_summarizer.rs  # code-summarizer client
│
└── utils.rs                # Shared utilities

templates/
├── claude.md.tera          # Claude template
├── gpt.md.tera             # GPT template
└── gemini.md.tera          # Gemini template

tests/
├── integration_test.rs     # End-to-end tests
├── packing_test.rs         # Packing algorithm tests
├── ranking_test.rs         # Scoring tests
├── token_test.rs           # Token counting tests
└── cache_test.rs           # Cache tests
```

### Core Implementation Examples

#### Token Counter Interface

```rust
// src/tokens/mod.rs
pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> Result<usize>;
    fn count_with_overhead(&self, text: &str, format_type: FormatType) -> Result<usize>;
}

pub enum FormatType {
    CodeBlock,  // ``` markers
    Section,    # headers
    Plain,
}

pub fn get_counter(model: &str) -> Result<Box<dyn TokenCounter>> {
    match model {
        "claude" => Ok(Box::new(ClaudeTokenCounter::new()?)),
        "gpt4" | "gpt35" => Ok(Box::new(GptTokenCounter::new(model)?)),
        "gemini" => Ok(Box::new(GeminiTokenCounter::new()?)),
        _ => Err(anyhow::anyhow!("Unsupported model: {}", model)),
    }
}
```

#### Budget Tracker

```rust
// src/pack/budget.rs
pub struct TokenBudget {
    total: usize,
    used: usize,
    reserved: usize,  // For architecture summary
    counter: Box<dyn TokenCounter>,
}

impl TokenBudget {
    pub fn new(total: usize, counter: Box<dyn TokenCounter>) -> Self {
        Self { total, used: 0, reserved: 500, counter }
    }

    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.used + self.reserved)
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

    pub fn usage_percent(&self) -> f64 {
        (self.used as f64 / self.total as f64) * 100.0
    }
}
```

#### Relevance Scorer

```rust
// src/rank/scorer.rs
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
    pub fn score_files(
        &self,
        files: &[PathBuf],
        query: &str,
        primary_files: &[PathBuf],
    ) -> Result<Vec<FileScore>> {
        let mut scores = Vec::new();

        for file in files {
            let query_match = self.calc_query_match(file, query)?;
            let dep_proximity = self.calc_dep_proximity(file, primary_files)?;
            let hotness = self.get_hotness(file)?;
            let recency = self.calc_recency(file)?;
            let centrality = self.calc_centrality(file)?;

            // Weighted formula
            let total = (query_match * 3.0) +
                       (dep_proximity * 2.0) +
                       (hotness * 1.5) +
                       (recency * 1.0) +
                       (centrality * 0.5);

            scores.push(FileScore {
                path: file.clone(),
                score: total,
                query_match,
                dep_proximity,
                hotness,
                recency,
                centrality,
            });
        }

        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(scores)
    }
}
```

#### Greedy Packer

```rust
// src/pack/greedy.rs
pub struct ContextPacker {
    budget: TokenBudget,
}

impl ContextPacker {
    pub fn pack(&mut self, scored_files: Vec<FileScore>) -> Result<PackedContext> {
        let mut included = Vec::new();
        let mut omitted = Vec::new();

        // Load architecture summary first
        let arch = self.load_architecture_summary()?;
        self.budget.add_reserved(&arch)?;

        // Greedy packing: add by score until budget full
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
                if let Some(partial) = self.try_partial(&file_score, &content)? {
                    let tokens = self.budget.add(&partial.content)?;
                    included.push(partial);
                } else {
                    omitted.push(file_score);
                }
            }

            // Stop if <100 tokens remain
            if self.budget.remaining() < 100 {
                break;
            }
        }

        Ok(PackedContext {
            architecture_summary: arch,
            included_files: included,
            omitted_files: omitted,
            tokens_used: self.budget.used,
            tokens_budget: self.budget.total,
        })
    }
}
```

#### Cache Manager

```rust
// src/cache/storage.rs
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize)]
pub struct CacheEntry {
    pub query: String,
    pub model: String,
    pub budget: usize,
    pub file_hashes: HashMap<PathBuf, String>,
    pub packed_context: String,
    pub created_at: i64,
}

pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    pub fn get(&self, query: &str, model: &str, budget: usize) -> Result<Option<String>> {
        let key = self.make_key(query, model, budget);
        let path = self.cache_dir.join(&key);

        if !path.exists() {
            return Ok(None);
        }

        let entry: CacheEntry = bincode::deserialize(&std::fs::read(&path)?)?;

        // Validate file hashes
        if self.validate_hashes(&entry.file_hashes)? {
            Ok(Some(entry.packed_context))
        } else {
            std::fs::remove_file(&path)?;
            Ok(None)
        }
    }

    pub fn store(&self, entry: CacheEntry) -> Result<()> {
        let key = self.make_key(&entry.query, &entry.model, entry.budget);
        let path = self.cache_dir.join(&key);
        let data = bincode::serialize(&entry)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    fn make_key(&self, query: &str, model: &str, budget: usize) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query);
        hasher.update(model);
        hasher.update(budget.to_string());
        format!("{:x}.cache", hasher.finalize())
    }

    fn validate_hashes(&self, hashes: &HashMap<PathBuf, String>) -> Result<bool> {
        for (path, expected) in hashes {
            if !path.exists() {
                return Ok(false);
            }
            let content = std::fs::read_to_string(path)?;
            let actual = self.hash_content(&content);
            if actual != *expected {
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
}
```

---

## Configuration

Default: `~/.config/ai-tools/config.toml`

```toml
[context-packer]
default_budget = 8000
default_model = "claude"
cache_dir = "~/.cache/ai-tools/context-packer"
cache_max_size_mb = 100
cache_max_age_days = 7

[context-packer.models]
[context-packer.models.claude]
tokenizer = "anthropic/claude"
max_tokens = 200000

[context-packer.models.gpt4]
tokenizer = "gpt-4"
max_tokens = 128000

[context-packer.models.gpt35]
tokenizer = "gpt-3.5-turbo"
max_tokens = 16000

[context-packer.models.gemini]
tokenizer = "google/gemini"
max_tokens = 1000000

[context-packer.ranking]
query_match_weight = 3.0
dep_proximity_weight = 2.0
hotness_weight = 1.5
recency_weight = 1.0
centrality_weight = 0.5

[context-packer.packing]
reserve_for_architecture = 500
min_remaining_to_continue = 100
enable_partial_files = true
max_dependency_depth = 3

[context-packer.cache]
enabled = true
validate_on_load = true
max_entries = 100
```

---

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_token_counting() {
    let counter = ClaudeTokenCounter::new().unwrap();
    assert!(counter.count("Hello").unwrap() > 0);
}

#[test]
fn test_budget_enforcement() {
    let counter = Box::new(ClaudeTokenCounter::new().unwrap());
    let mut budget = TokenBudget::new(1000, counter);

    let big_content = "x".repeat(10000);
    assert!(!budget.can_fit(&big_content).unwrap());
}

#[test]
fn test_relevance_scoring() {
    let scorer = RelevanceScorer::new(mock_client());
    let scores = scorer.score_files(&[path], "test", &[]).unwrap();
    assert!(scores[0].score >= 0.0);
}

#[test]
fn test_cache_validation() {
    let cache = CacheManager::new().unwrap();
    let mut hashes = HashMap::new();
    hashes.insert(PathBuf::from("nonexistent.txt"), "hash".to_string());
    assert!(!cache.validate_hashes(&hashes).unwrap());
}
```

### Integration Tests

```rust
#[test]
fn test_full_workflow() {
    let temp = tempfile::tempdir().unwrap();
    create_test_project(&temp);

    let output = Command::new("context-packer")
        .args(&["pack", "--query", "test", "--budget", "5000"])
        .current_dir(&temp)
        .output()
        .unwrap();

    assert!(output.status.success());
    let result = String::from_utf8(output.stdout).unwrap();
    assert!(result.contains("# Context:"));
    assert!(result.contains("## Architecture Overview"));
}
```

### Performance Benchmarks

- Pack 500-file project: < 3 seconds
- Cache hit: < 50ms
- Token counting per file: < 10ms
- Relevance scoring (100 files): < 500ms

---

## Best Practices & Industry Standards

### Error Handling
- Never `.unwrap()` in production
- Use `anyhow::Result` for propagation
- Use `thiserror` for custom errors
- Log errors before returning
- Provide helpful context in errors

### Code Quality
- **DRY**: Extract common patterns
- **SOLID**: Single responsibility per module
- **Testability**: Mock external dependencies
- **Documentation**: `///` for public APIs
- **Type safety**: Leverage Rust's type system

### Performance
- Cache token counts
- Lazy load file contents
- Use rayon for parallel scoring
- Profile before optimizing
- Efficient data structures (HashMap)

### Security
- Validate file paths (no directory traversal)
- Limit file sizes (< 10MB)
- Enforce token limits
- Cache file permissions (0600)
- Sanitize user input

### Logging

```rust
log::error!("Failed to load architecture: {}", e);
log::warn!("code-index unavailable, using basic scoring");
log::info!("Packed {} files, {} tokens used", n, tokens);
log::debug!("Scored {} with {:.2}", path, score);
log::trace!("Token count for {}: {}", file, count);
```

---

## Success Criteria

- [ ] Respects token budgets (never exceeds)
- [ ] Accurate relevance ranking
- [ ] Integrates with code-index, context-query, code-summarizer
- [ ] Supports Claude, GPT, Gemini
- [ ] Caching works correctly
- [ ] Interactive mode functional
- [ ] All tests pass (>80% coverage)
- [ ] Performance benchmarks met
- [ ] Documentation complete

---

## Development Phases

**Phase 1: Core (Week 1)**
- Token counting (Claude, GPT)
- Budget management
- Basic file selection
- Simple markdown output

**Phase 2: Ranking (Week 2)**
- Relevance scoring
- code-index integration
- context-query integration
- Hierarchical loading

**Phase 3: Polish (Week 3)**
- Model-specific formatters
- Template system
- Cache manager
- Interactive mode

**Phase 4: Testing (Week 4)**
- Comprehensive tests
- Documentation
- Performance tuning
- CLI refinement

---

## AI Agent Workflow Example

```bash
# Task: "Optimize search functionality"

# Agent calls:
context-packer pack \
  --query "optimize search performance" \
  --budget 8000 \
  --model claude \
  --include-dependencies \
  --output /tmp/context.md

# Workflow:
# 1. Load architecture (200 tokens)
# 2. Search for "search" (context-query)
# 3. Find: src/search/*.rs
# 4. Score by relevance
# 5. Include dependencies
# 6. Pack within 8000 tokens
# 7. Format for Claude

# Result: /tmp/context.md (7,842 tokens)
# Agent has perfect context to optimize
```

---

**This is the capstone tool.** Build token management and packing first, then add ranking and caching. Focus on correctness over performance initially.

For detailed implementation guidance, see `context-packer-ENHANCED.md`.
