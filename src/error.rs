use thiserror::Error;

#[derive(Error, Debug)]
pub enum PackerError {
    #[error("Token budget exceeded: used {used}, budget {budget}")]
    BudgetExceeded { used: usize, budget: usize },

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Token counting failed: {0}")]
    TokenCountError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walk directory error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, PackerError>;
