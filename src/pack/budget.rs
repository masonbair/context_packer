use crate::error::{PackerError, Result};
use crate::tokens::TokenCounter;
use std::sync::Arc;

/// Manages token budget for context packing
pub struct TokenBudget {
    total: usize,
    used: usize,
    reserved: usize,
    counter: Arc<dyn TokenCounter>,
}

impl TokenBudget {
    /// Create a new budget with total tokens and reserved space for architecture summary
    pub fn new(total: usize, counter: Arc<dyn TokenCounter>) -> Self {
        Self {
            total,
            used: 0,
            reserved: 500, // Default reserve for architecture summary
            counter,
        }
    }

    /// Create budget with custom reserved amount
    pub fn with_reserved(total: usize, reserved: usize, counter: Arc<dyn TokenCounter>) -> Self {
        Self {
            total,
            used: 0,
            reserved,
            counter,
        }
    }

    /// Get remaining available tokens (excluding reserved)
    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.used + self.reserved)
    }

    /// Get total budget
    pub fn total(&self) -> usize {
        self.total
    }

    /// Get used tokens
    pub fn used(&self) -> usize {
        self.used
    }

    /// Check if content fits in remaining budget
    pub fn can_fit(&self, content: &str) -> Result<bool> {
        let tokens = self.counter.count(content)?;
        Ok(self.used + tokens + self.reserved <= self.total)
    }

    /// Add content to budget, returns token count if successful
    pub fn add(&mut self, content: &str) -> Result<usize> {
        let tokens = self.counter.count(content)?;
        if self.used + tokens + self.reserved > self.total {
            return Err(PackerError::BudgetExceeded {
                used: self.used + tokens,
                budget: self.total.saturating_sub(self.reserved),
            });
        }
        self.used += tokens;
        Ok(tokens)
    }

    /// Add content to reserved space (e.g., architecture summary)
    pub fn add_reserved(&mut self, content: &str) -> Result<usize> {
        let tokens = self.counter.count(content)?;
        // Reserved content uses from reserved pool, doesn't affect main budget
        self.used += tokens;
        // Reduce reserved as we use it
        self.reserved = self.reserved.saturating_sub(tokens);
        Ok(tokens)
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f64 {
        (self.used as f64 / self.total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::TiktokenCounter;

    fn make_budget(total: usize) -> TokenBudget {
        let counter = Arc::new(TiktokenCounter::new().unwrap());
        TokenBudget::new(total, counter)
    }

    #[test]
    fn test_budget_creation() {
        let budget = make_budget(8000);
        assert_eq!(budget.total(), 8000);
        assert_eq!(budget.used(), 0);
    }

    #[test]
    fn test_remaining_accounts_for_reserved() {
        let budget = make_budget(8000);
        // Default reserved is 500
        assert_eq!(budget.remaining(), 7500);
    }

    #[test]
    fn test_can_fit_small_content() {
        let budget = make_budget(8000);
        let result = budget.can_fit("Hello, world!");
        assert!(result.is_ok());
        assert!(result.unwrap(), "Small content should fit in large budget");
    }

    #[test]
    fn test_cannot_fit_large_content() {
        let budget = make_budget(10);
        // Generate content larger than budget
        let large_content = "word ".repeat(100);
        let result = budget.can_fit(&large_content);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "Large content should not fit in small budget");
    }

    #[test]
    fn test_add_content_updates_used() {
        let mut budget = make_budget(8000);
        let initial_used = budget.used();

        budget.add("Hello, world!").unwrap();

        assert!(budget.used() > initial_used, "Used should increase after adding content");
    }

    #[test]
    fn test_add_exceeds_budget_returns_error() {
        let mut budget = make_budget(10);
        let large_content = "word ".repeat(100);

        let result = budget.add(&large_content);

        assert!(result.is_err(), "Should error when exceeding budget");
    }

    #[test]
    fn test_usage_percent() {
        let mut budget = make_budget(1000);
        budget.add("test content here").unwrap();

        let percent = budget.usage_percent();
        assert!(percent > 0.0, "Usage percent should be positive after adding content");
        assert!(percent < 100.0, "Usage percent should be less than 100");
    }

    #[test]
    fn test_remaining_decreases_after_add() {
        let mut budget = make_budget(8000);
        let initial_remaining = budget.remaining();

        budget.add("Some test content").unwrap();

        assert!(budget.remaining() < initial_remaining, "Remaining should decrease after add");
    }
}
