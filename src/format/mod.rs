use crate::pack::PackedContext;

/// Formatter trait for different output formats
pub trait Formatter {
    fn format(&self, context: &PackedContext, query: &str) -> String;
}

/// Markdown formatter for Claude
pub struct ClaudeFormatter;

impl Formatter for ClaudeFormatter {
    fn format(&self, context: &PackedContext, query: &str) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("# Context: {}\n\n", query));
        output.push_str(&format!(
            "**Token Budget:** {} / {} used ({:.1}%)\n\n",
            context.tokens_budget,
            context.tokens_used,
            (context.tokens_used as f64 / context.tokens_budget as f64) * 100.0
        ));
        output.push_str("---\n\n");

        // Architecture summary if present
        if let Some(ref summary) = context.architecture_summary {
            output.push_str("## Architecture Overview\n\n");
            output.push_str(summary);
            output.push_str("\n\n---\n\n");
        }

        // Relevant code
        output.push_str("## Relevant Code\n\n");

        for file in &context.included_files {
            let priority = if file.score >= 0.8 {
                "HIGH"
            } else if file.score >= 0.5 {
                "MEDIUM"
            } else {
                "LOW"
            };

            let path_str = file.path.display();
            let extension = file
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("txt");

            output.push_str(&format!(
                "### {} (Priority: {}, {} tokens)\n\n",
                path_str, priority, file.tokens
            ));
            output.push_str(&format!("```{}\n", extension));
            output.push_str(&file.content);
            if !file.content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("```\n\n");
        }

        // Omitted files summary
        if !context.omitted_files.is_empty() {
            output.push_str("---\n\n");
            output.push_str("## Omitted Files\n\n");
            for file in &context.omitted_files {
                output.push_str(&format!(
                    "- {} (score: {:.2}, reason: {})\n",
                    file.path.display(),
                    file.score,
                    file.reason
                ));
            }
        }

        output
    }
}

/// Compact JSON formatter for GPT
pub struct GptFormatter;

impl Formatter for GptFormatter {
    fn format(&self, context: &PackedContext, query: &str) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", query));
        output.push_str(&format!(
            "Budget: {}/{} tokens ({:.0}%)\n\n",
            context.tokens_used,
            context.tokens_budget,
            (context.tokens_used as f64 / context.tokens_budget as f64) * 100.0
        ));

        if let Some(ref summary) = context.architecture_summary {
            output.push_str("## Overview\n");
            output.push_str(summary);
            output.push_str("\n\n");
        }

        for (i, file) in context.included_files.iter().enumerate() {
            let extension = file
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("txt");

            output.push_str(&format!("## {}. {}\n", i + 1, file.path.display()));
            output.push_str(&format!("```{}\n", extension));
            output.push_str(&file.content);
            if !file.content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("```\n\n");
        }

        output
    }
}

/// Get formatter for a model
pub fn get_formatter(model: &str) -> Box<dyn Formatter> {
    match model {
        "gpt4" | "gpt35" | "gpt-4" | "gpt-3.5-turbo" => Box::new(GptFormatter),
        _ => Box::new(ClaudeFormatter), // Default to Claude format
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{IncludedFile, OmittedFile, PackedContext};
    use std::path::PathBuf;

    fn make_test_context() -> PackedContext {
        PackedContext {
            architecture_summary: Some("Test architecture".to_string()),
            included_files: vec![IncludedFile {
                path: PathBuf::from("src/main.rs"),
                content: "fn main() {}".to_string(),
                tokens: 10,
                score: 0.9,
            }],
            omitted_files: vec![OmittedFile {
                path: PathBuf::from("src/unused.rs"),
                tokens: 100,
                score: 0.1,
                reason: "Low relevance".to_string(),
            }],
            tokens_used: 510,
            tokens_budget: 8000,
        }
    }

    #[test]
    fn test_claude_formatter_header() {
        let formatter = ClaudeFormatter;
        let context = make_test_context();
        let output = formatter.format(&context, "test query");

        assert!(output.contains("# Context: test query"));
        assert!(output.contains("Token Budget:"));
    }

    #[test]
    fn test_claude_formatter_includes_code() {
        let formatter = ClaudeFormatter;
        let context = make_test_context();
        let output = formatter.format(&context, "test");

        assert!(output.contains("fn main() {}"));
        assert!(output.contains("```rs"));
    }

    #[test]
    fn test_claude_formatter_includes_architecture() {
        let formatter = ClaudeFormatter;
        let context = make_test_context();
        let output = formatter.format(&context, "test");

        assert!(output.contains("## Architecture Overview"));
        assert!(output.contains("Test architecture"));
    }

    #[test]
    fn test_claude_formatter_shows_omitted() {
        let formatter = ClaudeFormatter;
        let context = make_test_context();
        let output = formatter.format(&context, "test");

        assert!(output.contains("## Omitted Files"));
        assert!(output.contains("src/unused.rs"));
    }

    #[test]
    fn test_gpt_formatter_compact() {
        let formatter = GptFormatter;
        let context = make_test_context();
        let output = formatter.format(&context, "test");

        // GPT format is more compact, uses numbered lists
        assert!(output.contains("# test"));
        assert!(output.contains("1. src/main.rs"));
    }

    #[test]
    fn test_get_formatter_returns_correct_type() {
        let claude_fmt = get_formatter("claude");
        let gpt_fmt = get_formatter("gpt4");

        // Both should produce valid output
        let context = make_test_context();
        let claude_output = claude_fmt.format(&context, "test");
        let gpt_output = gpt_fmt.format(&context, "test");

        assert!(claude_output.contains("Context:"));
        assert!(!gpt_output.contains("Context:")); // GPT format doesn't use "Context:"
    }
}
