use crate::error::Result;
use crate::pack::TokenBudget;
use std::path::PathBuf;

/// A file that was included in the packed context
#[derive(Debug, Clone)]
pub struct IncludedFile {
    pub path: PathBuf,
    pub content: String,
    pub tokens: usize,
    pub score: f64,
}

/// A file that was scored but not included
#[derive(Debug, Clone)]
pub struct OmittedFile {
    pub path: PathBuf,
    pub tokens: usize,
    pub score: f64,
    pub reason: String,
}

/// The result of packing context
#[derive(Debug)]
pub struct PackedContext {
    pub architecture_summary: Option<String>,
    pub included_files: Vec<IncludedFile>,
    pub omitted_files: Vec<OmittedFile>,
    pub tokens_used: usize,
    pub tokens_budget: usize,
}

/// File with relevance score for packing
#[derive(Debug, Clone)]
pub struct ScoredFile {
    pub path: PathBuf,
    pub score: f64,
}

/// Context packer - greedy algorithm to pack files within budget
pub struct ContextPacker {
    budget: TokenBudget,
    min_remaining: usize,
}

impl ContextPacker {
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            budget,
            min_remaining: 100, // Stop when less than this many tokens remain
        }
    }

    /// Pack files in order of score (highest first) until budget is exhausted
    pub fn pack(&mut self, scored_files: Vec<ScoredFile>) -> Result<PackedContext> {
        let mut included = Vec::new();
        let mut omitted = Vec::new();

        for file in scored_files {
            // Stop if too little budget remains
            if self.budget.remaining() < self.min_remaining {
                omitted.push(OmittedFile {
                    path: file.path,
                    tokens: 0,
                    score: file.score,
                    reason: "Budget exhausted".to_string(),
                });
                continue;
            }

            // Read file content
            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(e) => {
                    omitted.push(OmittedFile {
                        path: file.path,
                        tokens: 0,
                        score: file.score,
                        reason: format!("Read error: {}", e),
                    });
                    continue;
                }
            };

            // Try to fit file in budget
            if self.budget.can_fit(&content)? {
                let tokens = self.budget.add(&content)?;
                included.push(IncludedFile {
                    path: file.path,
                    content,
                    tokens,
                    score: file.score,
                });
            } else {
                omitted.push(OmittedFile {
                    path: file.path,
                    tokens: 0, // Could count tokens if needed
                    score: file.score,
                    reason: "Exceeds remaining budget".to_string(),
                });
            }
        }

        Ok(PackedContext {
            architecture_summary: None,
            included_files: included,
            omitted_files: omitted,
            tokens_used: self.budget.used(),
            tokens_budget: self.budget.total(),
        })
    }

    /// Add architecture summary (uses reserved budget)
    pub fn add_architecture_summary(&mut self, summary: &str) -> Result<usize> {
        self.budget.add_reserved(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::TiktokenCounter;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn make_packer(budget: usize) -> ContextPacker {
        let counter = Arc::new(TiktokenCounter::new().unwrap());
        let budget = TokenBudget::new(budget, counter);
        ContextPacker::new(budget)
    }

    #[test]
    fn test_packer_creation() {
        let packer = make_packer(8000);
        assert_eq!(packer.budget.total(), 8000);
    }

    #[test]
    fn test_pack_empty_list() {
        let mut packer = make_packer(8000);
        let result = packer.pack(vec![]).unwrap();

        assert!(result.included_files.is_empty());
        assert!(result.omitted_files.is_empty());
        assert_eq!(result.tokens_used, 0);
    }

    #[test]
    fn test_pack_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let mut packer = make_packer(8000);
        let files = vec![ScoredFile {
            path: file_path.clone(),
            score: 1.0,
        }];

        let result = packer.pack(files).unwrap();

        assert_eq!(result.included_files.len(), 1);
        assert_eq!(result.included_files[0].path, file_path);
        assert!(result.tokens_used > 0);
    }

    #[test]
    fn test_pack_respects_budget() {
        let dir = tempdir().unwrap();

        // Create a large file
        let large_file = dir.path().join("large.rs");
        let large_content = "fn function() { println!(\"x\"); }\n".repeat(500);
        std::fs::write(&large_file, &large_content).unwrap();

        // Very small budget
        let mut packer = make_packer(100);
        let files = vec![ScoredFile {
            path: large_file.clone(),
            score: 1.0,
        }];

        let result = packer.pack(files).unwrap();

        // File should be omitted due to budget
        assert!(result.included_files.is_empty());
        assert_eq!(result.omitted_files.len(), 1);
    }

    #[test]
    fn test_pack_orders_by_score() {
        let dir = tempdir().unwrap();

        let file1 = dir.path().join("low.rs");
        let file2 = dir.path().join("high.rs");
        std::fs::write(&file1, "// low priority").unwrap();
        std::fs::write(&file2, "// high priority").unwrap();

        let mut packer = make_packer(8000);
        // Note: files should be passed already sorted by score
        let files = vec![
            ScoredFile { path: file2.clone(), score: 0.9 },
            ScoredFile { path: file1.clone(), score: 0.1 },
        ];

        let result = packer.pack(files).unwrap();

        assert_eq!(result.included_files.len(), 2);
        // First file should be high priority
        assert_eq!(result.included_files[0].path, file2);
    }

    #[test]
    fn test_pack_handles_missing_file() {
        let mut packer = make_packer(8000);
        let files = vec![ScoredFile {
            path: PathBuf::from("/nonexistent/file.rs"),
            score: 1.0,
        }];

        let result = packer.pack(files).unwrap();

        assert!(result.included_files.is_empty());
        assert_eq!(result.omitted_files.len(), 1);
        assert!(result.omitted_files[0].reason.contains("Read error"));
    }
}
