use crate::error::{PackerError, Result};
use tiktoken_rs::get_bpe_from_model;

/// Token counter abstraction
pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> Result<usize>;
}

/// Default token counter using tiktoken (cl100k_base - works for GPT-4, Claude-like)
pub struct TiktokenCounter {
    bpe: tiktoken_rs::CoreBPE,
}

impl TiktokenCounter {
    pub fn new() -> Result<Self> {
        let bpe = get_bpe_from_model("gpt-4")
            .map_err(|e| PackerError::TokenCountError(e.to_string()))?;
        Ok(Self { bpe })
    }

    pub fn for_model(model: &str) -> Result<Self> {
        // Map model names to tiktoken models
        let tiktoken_model = match model {
            "claude" | "claude-3" | "claude-opus" | "claude-sonnet" => "gpt-4", // cl100k_base is close enough
            "gpt4" | "gpt-4" => "gpt-4",
            "gpt35" | "gpt-3.5-turbo" => "gpt-3.5-turbo",
            "gemini" => "gpt-4", // cl100k_base approximation
            _ => "gpt-4",
        };

        let bpe = get_bpe_from_model(tiktoken_model)
            .map_err(|e| PackerError::TokenCountError(e.to_string()))?;
        Ok(Self { bpe })
    }
}

impl TokenCounter for TiktokenCounter {
    fn count(&self, text: &str) -> Result<usize> {
        Ok(self.bpe.encode_with_special_tokens(text).len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter_creation() {
        let counter = TiktokenCounter::new();
        assert!(counter.is_ok());
    }

    #[test]
    fn test_token_counting_basic() {
        let counter = TiktokenCounter::new().unwrap();
        let count = counter.count("Hello, world!").unwrap();
        assert!(count > 0, "Token count should be positive");
        assert!(count < 10, "Simple phrase should have few tokens");
    }

    #[test]
    fn test_token_counting_code() {
        let counter = TiktokenCounter::new().unwrap();
        let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let count = counter.count(code).unwrap();
        assert!(count > 5, "Code should have multiple tokens");
    }

    #[test]
    fn test_token_counting_empty() {
        let counter = TiktokenCounter::new().unwrap();
        let count = counter.count("").unwrap();
        assert_eq!(count, 0, "Empty string should have 0 tokens");
    }

    #[test]
    fn test_for_model_claude() {
        let counter = TiktokenCounter::for_model("claude");
        assert!(counter.is_ok());
    }

    #[test]
    fn test_for_model_gpt4() {
        let counter = TiktokenCounter::for_model("gpt4");
        assert!(counter.is_ok());
    }
}
