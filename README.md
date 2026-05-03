# context-packer

Smart context assembly tool for AI agents - intelligently packs relevant code context within strict token budgets using multi-factor relevance ranking.

## Overview

**context-packer** is the capstone integration layer that combines code-index, code-summarizer, and context-query to create optimized context for AI agents. It solves the fundamental problem of balancing comprehensive code understanding with strict token limits.

### The Problem

AI agents need context to work effectively, but:
- Token limits are strict (8K-200K depending on model)
- Manually selecting files is inefficient and error-prone
- Including too much code wastes tokens on irrelevant information
- Excluding dependencies breaks understanding

### The Solution

context-packer uses intelligent relevance ranking and greedy packing algorithms to:
- **Never exceed token budgets** - Strict enforcement with real-time tracking
- **Maximize relevance** - Multi-factor scoring (query match, dependencies, hotness, recency)
- **Hierarchical loading** - Architecture summary → Primary files → Dependencies → Callers
- **Model-specific formatting** - Optimized output for Claude, GPT-4, Gemini
- **Smart caching** - Avoid re-sending unchanged code across sessions

## Position in Ecosystem

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

## Key Features

- **Token Budget Management** - Strict enforcement, never exceeds limits
- **Multi-Factor Relevance Scoring** - Query match + dependencies + hotness + recency + centrality
- **Hierarchical Loading** - Prioritizes architecture summary, primary files, then dependencies
- **Multi-Model Support** - Claude, GPT-4, GPT-3.5, Gemini with model-specific tokenizers
- **Smart Caching** - Content-hash based caching with automatic invalidation
- **Interactive Mode** - Guided workflow for building context
- **Dependency Tracking** - Automatically includes imported files and callers
- **Partial File Inclusion** - Extracts key functions when full file won't fit

## Installation

### Prerequisites

- Rust 1.75+ (`rustup` recommended)
- code-index installed and indexed
- context-query installed
- code-summarizer installed (optional but recommended)

### From Source

```bash
git clone https://github.com/yourusername/context-packer.git
cd context-packer
cargo build --release
sudo cp target/release/context-packer /usr/local/bin/
```

### Verify Installation

```bash
context-packer --version
```

## Quick Start

### Basic Usage

```bash
# Pack context for a task
context-packer pack --query "implement authentication"

# With specific budget and model
context-packer pack \
  --query "optimize search performance" \
  --budget 8000 \
  --model claude

# Focus on specific file with dependencies
context-packer pack \
  --file src/auth/login.ts \
  --include-dependencies \
  --include-callers
```

### Interactive Mode

```bash
context-packer interactive
```

The interactive mode will prompt you for:
1. What are you working on? (query)
2. Token budget? (default: 8000)
3. Target model? (default: claude)
4. Include dependencies? (y/n)
5. Include callers? (y/n)

## Usage Examples

### Pack Context for Code Review

```bash
context-packer pack \
  --query "review authentication changes" \
  --budget 10000 \
  --model claude \
  --output review-context.md
```

### Optimize for Specific Model

```bash
# GPT-4 with tight budget
context-packer pack \
  --query "add logging to API endpoints" \
  --budget 6000 \
  --model gpt4

# Gemini with large budget
context-packer pack \
  --query "refactor database layer" \
  --budget 50000 \
  --model gemini
```

### Include Dependencies and Callers

```bash
context-packer pack \
  --file src/core/engine.rs \
  --include-dependencies \
  --include-callers \
  --depth 2 \
  --budget 15000
```

### Dry Run (Preview)

```bash
context-packer pack \
  --query "add feature flags" \
  --dry-run
```

Output shows:
- Files ranked by relevance score
- Which files will be included (✓)
- Which files will be omitted (✗)
- Token usage breakdown

### JSON Output

```bash
context-packer pack \
  --query "implement caching" \
  --format json \
  --output context.json
```

## CLI Reference

### Global Options

```
--config <PATH>         Custom config file
--project-root <PATH>   Project root (default: current dir)
--verbose, -v           Verbose logging
--quiet, -q             Quiet mode (errors only)
--no-cache              Disable caching
--help, -h              Show help
--version, -V           Show version
```

### Pack Command

```
context-packer pack [OPTIONS]

OPTIONS:
  --query, -q <TEXT>        Task/query description
  --file, -f <PATH>         Focus on specific file
  --budget, -b <N>          Token budget (default: 8000)
  --model, -m <MODEL>       claude|gpt4|gpt35|gemini (default: claude)
  --output, -o <PATH>       Output file (default: stdout)
  --format <FMT>            markdown|json (default: markdown)
  --include-dependencies    Include imported files
  --include-callers         Include calling code
  --include-types           Include type definitions
  --depth <N>               Dependency depth (1-3, default: 1)
  --dry-run                 Preview without generating output
```

### Cache Management

```bash
# View cache statistics
context-packer cache stats

# Clear all cache
context-packer cache clear

# Clear old entries (>7 days)
context-packer cache clear --older-than 7

# Invalidate specific patterns
context-packer cache invalidate "src/auth/*.ts"
```

## Configuration

Default config location: `~/.config/ai-tools/config.toml`

```toml
[context-packer]
default_budget = 8000
default_model = "claude"
cache_dir = "~/.cache/ai-tools/context-packer"
cache_max_size_mb = 100
cache_max_age_days = 7

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
```

## How It Works

### Relevance Ranking Formula

```
score = (query_match × 3.0) +
        (dep_proximity × 2.0) +
        (hotness × 1.5) +
        (recency × 1.0) +
        (centrality × 0.5)
```

**Factors:**
- **query_match** - How well file content matches the query
- **dep_proximity** - Direct dependency=1.0, transitive=0.5
- **hotness** - From code-index (complexity + change frequency)
- **recency** - Exponential decay based on last modified
- **centrality** - Files with many connections (hubs)

### Packing Algorithm

1. **Reserve** 200-500 tokens for architecture summary
2. **Find** relevant files using context-query
3. **Score** files using multi-factor formula
4. **Sort** by score (descending)
5. **Pack** greedily until budget full:
   - Add highest scored file
   - Count tokens
   - If exceeds budget: try partial or skip
   - Continue until <100 tokens remain
6. **Format** for target model
7. **Cache** result with file content hashes

## Development

**Languages:** Rust
**Project Type:** CLI tool

### Build from Source

```bash
git clone https://github.com/yourusername/context-packer.git
cd context-packer
cargo build
cargo test
cargo run -- pack --query "test"
```

### Run Tests

```bash
# All tests
cargo test

# Integration tests only
cargo test --test integration_test

# With logging
RUST_LOG=debug cargo test

# Performance benchmarks
cargo bench
```

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── cli.rs               # Argument parsing
├── tokens/              # Token counting (Claude, GPT, Gemini)
├── query/               # Query processing & expansion
├── rank/                # Relevance scoring
├── pack/                # Greedy packing algorithm
├── format/              # Model-specific formatters
├── cache/               # Cache management
└── tools/               # Tool integration (code-index, etc.)
```

### AI Agent Support

This project is configured for AI agent workflows:
- `CLAUDE.md` - Detailed AI agent instructions
- `.ai/TOOLS.md` - Available custom tooling
- `.ai/ARCHITECTURE.md` - System architecture
- `.ai/CONVENTIONS.md` - Coding conventions

## Performance

- Pack 500-file project: < 3 seconds
- Cache hit: < 50ms
- Token counting per file: < 10ms
- Relevance scoring (100 files): < 500ms

## Troubleshooting

### "code-index database not found"

Ensure code-index is installed and you've run `code-index index` in your project.

### "Token count exceeded budget"

Try:
- Increase budget: `--budget 15000`
- Enable partial files in config
- Use more specific query to reduce matches

### Cache not working

Clear and rebuild:
```bash
context-packer cache clear
```

### Slow performance

- Check if code-index is up to date
- Clear old cache entries
- Reduce `--depth` for dependencies

## License

MIT License - see LICENSE file for details
