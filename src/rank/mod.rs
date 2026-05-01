use crate::error::Result;
use crate::pack::ScoredFile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Relevance scorer with optional code-index integration
pub struct RelevanceScorer {
    use_code_index: bool,
    project_root: PathBuf,
}

impl RelevanceScorer {
    pub fn new(project_root: PathBuf) -> Self {
        // Check if code-index is available
        let use_code_index = Command::new("code-index")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self {
            use_code_index,
            project_root,
        }
    }

    /// Score files relevant to a query
    pub fn score_files(&self, query: &str) -> Result<Vec<ScoredFile>> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut files = Vec::new();

        // Get hotness scores from code-index if available
        let hotness_map = if self.use_code_index {
            self.get_hotness_scores().unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Walk the project and score files
        for entry in WalkDir::new(&self.project_root)
            .into_iter()
            .filter_entry(|e| !is_ignored(e.path()))
        {
            let entry = entry?;
            if entry.file_type().is_file() && is_code_file(entry.path()) {
                let path = entry.path().to_path_buf();

                // Calculate multi-factor score
                let query_match = self.calc_query_match(&path, &query_words);
                let recency = self.calc_recency(&path);
                let hotness = hotness_map
                    .get(&path)
                    .copied()
                    .unwrap_or(0.0);

                // Weighted score formula
                let score = (query_match * 3.0) + (hotness * 1.5) + (recency * 1.0);

                // Only include files with some query relevance, or very high hotness
                if query_match > 0.0 || hotness > 0.5 {
                    files.push(ScoredFile {
                        path,
                        score, // Keep raw score for proper ranking
                    });
                }
            }
        }

        // Sort by score descending
        files.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(files)
    }

    /// Calculate query match score (0.0-1.0)
    fn calc_query_match(&self, path: &Path, query_words: &[&str]) -> f64 {
        let path_str = path.to_string_lossy().to_lowercase();
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let mut score = 0.0;

        // Filename matches are weighted higher
        for word in query_words {
            if filename.contains(word) {
                score += 0.3;
            }
            if path_str.contains(word) {
                score += 0.1;
            }
        }

        // Check content matches
        if let Ok(content) = std::fs::read_to_string(path) {
            let content_lower = content.to_lowercase();
            for word in query_words {
                if content_lower.contains(word) {
                    score += 0.1;
                }
            }
        }

        // Boost entry points
        if path_str.ends_with("main.rs") || path_str.ends_with("lib.rs") || path_str.ends_with("mod.rs") {
            score += 0.1;
        }

        (score / 3.0_f64).min(1.0_f64) // Normalize to 0-1
    }

    /// Calculate recency score based on file modification time (0.0-1.0)
    fn calc_recency(&self, path: &Path) -> f64 {
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let now = std::time::SystemTime::now();
                if let Ok(age) = now.duration_since(modified) {
                    // Exponential decay: newer files score higher
                    let days = age.as_secs() as f64 / 86400.0;
                    return (-days / 30.0).exp(); // Half-life of ~30 days
                }
            }
        }
        0.5 // Default for files where we can't determine age
    }

    /// Get hotness scores from code-index
    fn get_hotness_scores(&self) -> Result<HashMap<PathBuf, f64>> {
        let output = Command::new("code-index")
            .args(["query", "hot-files", "--json"])
            .current_dir(&self.project_root)
            .output();

        let mut scores = HashMap::new();

        if let Ok(output) = output {
            if output.status.success() {
                if let Ok(json) = String::from_utf8(output.stdout) {
                    // Parse JSON output - simplified parsing
                    // Expected format: [{"path": "src/file.rs", "hotness": 0.8}, ...]
                    if let Ok(parsed) = serde_json::from_str::<Vec<HotFile>>(&json) {
                        for entry in parsed {
                            let path = self.project_root.join(&entry.path);
                            scores.insert(path, entry.hotness);
                        }
                    }
                }
            }
        }

        Ok(scores)
    }
}

#[derive(serde::Deserialize)]
struct HotFile {
    path: String,
    hotness: f64,
}

fn is_ignored(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/.git")
        || path_str.contains("/target")
        || path_str.contains("/node_modules")
        || path_str.contains("/.ai")
        || path_str.contains("/__pycache__")
}

fn is_code_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(
            ext,
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp"
                | "rb" | "ex" | "exs" | "sh" | "toml" | "yaml" | "yml" | "json" | "md"
        ),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scorer_creation() {
        let dir = tempdir().unwrap();
        let scorer = RelevanceScorer::new(dir.path().to_path_buf());
        // Should not crash
        assert!(scorer.project_root == dir.path());
    }

    #[test]
    fn test_score_files_empty_dir() {
        let dir = tempdir().unwrap();
        let scorer = RelevanceScorer::new(dir.path().to_path_buf());
        let files = scorer.score_files("test").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_score_files_matches_query() {
        let dir = tempdir().unwrap();

        // Create test files
        std::fs::write(dir.path().join("auth.rs"), "fn authenticate() {}").unwrap();
        std::fs::write(dir.path().join("other.rs"), "fn other() {}").unwrap();

        let scorer = RelevanceScorer::new(dir.path().to_path_buf());
        let files = scorer.score_files("auth").unwrap();

        assert!(!files.is_empty());
        // auth.rs should be ranked first
        assert!(files[0].path.to_string_lossy().contains("auth"));
    }

    #[test]
    fn test_recency_score() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.rs");
        std::fs::write(&file, "fn test() {}").unwrap();

        let scorer = RelevanceScorer::new(dir.path().to_path_buf());
        let recency = scorer.calc_recency(&file);

        // Recently created file should have high recency score
        assert!(recency > 0.9, "Recently created file should have high recency");
    }

    #[test]
    fn test_query_match_filename() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("authentication.rs");
        std::fs::write(&file, "fn test() {}").unwrap();

        let scorer = RelevanceScorer::new(dir.path().to_path_buf());
        let score = scorer.calc_query_match(&file, &["auth"]);

        assert!(score > 0.0, "Filename containing query word should score > 0");
    }

    #[test]
    fn test_query_match_content() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("other.rs");
        std::fs::write(&file, "fn authenticate_user() {}").unwrap();

        let scorer = RelevanceScorer::new(dir.path().to_path_buf());
        let score = scorer.calc_query_match(&file, &["authenticate"]);

        assert!(score > 0.0, "File content containing query word should score > 0");
    }
}
