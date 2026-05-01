use crate::error::{PackerError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A cached context entry
#[derive(Serialize, Deserialize, Debug)]
pub struct CacheEntry {
    pub query: String,
    pub model: String,
    pub budget: usize,
    pub file_hashes: HashMap<PathBuf, String>,
    pub packed_context: String,
    pub tokens_used: usize,
    pub created_at: i64,
}

/// Cache manager for storing and retrieving packed contexts
pub struct CacheManager {
    cache_dir: PathBuf,
    max_size_mb: usize,
    max_age_days: u32,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Result<Self> {
        let cache_dir = directories::ProjectDirs::from("com", "ai-tools", "context-packer")
            .map(|d| d.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".cache"));

        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            max_size_mb: 100,
            max_age_days: 7,
        })
    }

    /// Create cache manager with custom directory
    pub fn with_dir(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            max_size_mb: 100,
            max_age_days: 7,
        })
    }

    /// Get a cached context if valid
    pub fn get(&self, query: &str, model: &str, budget: usize) -> Result<Option<CacheEntry>> {
        let key = self.make_key(query, model, budget);
        let path = self.cache_dir.join(&key);

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path)?;
        let entry: CacheEntry = match bincode::deserialize(&data) {
            Ok(e) => e,
            Err(_) => {
                // Corrupted cache, remove it
                let _ = fs::remove_file(&path);
                return Ok(None);
            }
        };

        // Validate file hashes
        if self.validate_hashes(&entry.file_hashes)? {
            Ok(Some(entry))
        } else {
            // Files changed, invalidate cache
            let _ = fs::remove_file(&path);
            Ok(None)
        }
    }

    /// Store a packed context in cache
    pub fn store(&self, entry: &CacheEntry) -> Result<()> {
        let key = self.make_key(&entry.query, &entry.model, entry.budget);
        let path = self.cache_dir.join(&key);

        let data = bincode::serialize(entry)
            .map_err(|e| PackerError::Config(format!("Cache serialization failed: {}", e)))?;

        fs::write(&path, data)?;
        Ok(())
    }

    /// Create file hashes for a set of files
    pub fn hash_files(&self, files: &[PathBuf]) -> Result<HashMap<PathBuf, String>> {
        let mut hashes = HashMap::new();
        for path in files {
            if path.exists() {
                let content = fs::read_to_string(path)?;
                let hash = self.hash_content(&content);
                hashes.insert(path.clone(), hash);
            }
        }
        Ok(hashes)
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let mut total_entries = 0;
        let mut total_size = 0u64;
        let mut oldest: Option<i64> = None;
        let mut newest: Option<i64> = None;

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("cache") {
                total_entries += 1;
                total_size += entry.metadata()?.len();

                if let Ok(data) = fs::read(&path) {
                    if let Ok(cache_entry) = bincode::deserialize::<CacheEntry>(&data) {
                        match oldest {
                            None => oldest = Some(cache_entry.created_at),
                            Some(o) if cache_entry.created_at < o => {
                                oldest = Some(cache_entry.created_at)
                            }
                            _ => {}
                        }
                        match newest {
                            None => newest = Some(cache_entry.created_at),
                            Some(n) if cache_entry.created_at > n => {
                                newest = Some(cache_entry.created_at)
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(CacheStats {
            total_entries,
            total_size_bytes: total_size,
            cache_dir: self.cache_dir.clone(),
            oldest_entry: oldest,
            newest_entry: newest,
        })
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<usize> {
        let mut cleared = 0;
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("cache") {
                fs::remove_file(&path)?;
                cleared += 1;
            }
        }
        Ok(cleared)
    }

    /// Clear cache entries older than N days
    pub fn clear_older_than(&self, days: u32) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let max_age_secs = (days as i64) * 24 * 60 * 60;
        let mut cleared = 0;

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("cache") {
                if let Ok(data) = fs::read(&path) {
                    if let Ok(cache_entry) = bincode::deserialize::<CacheEntry>(&data) {
                        if now - cache_entry.created_at > max_age_secs {
                            fs::remove_file(&path)?;
                            cleared += 1;
                        }
                    }
                }
            }
        }
        Ok(cleared)
    }

    fn make_key(&self, query: &str, model: &str, budget: usize) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hasher.update(model.as_bytes());
        hasher.update(budget.to_string().as_bytes());
        format!("{:x}.cache", hasher.finalize())
    }

    fn validate_hashes(&self, hashes: &HashMap<PathBuf, String>) -> Result<bool> {
        for (path, expected) in hashes {
            if !path.exists() {
                return Ok(false);
            }
            let content = fs::read_to_string(path)?;
            let actual = self.hash_content(&content);
            if actual != *expected {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn hash_content(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            cache_dir: PathBuf::from(".cache"),
            max_size_mb: 100,
            max_age_days: 7,
        })
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub cache_dir: PathBuf,
    pub oldest_entry: Option<i64>,
    pub newest_entry: Option<i64>,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache Directory: {}", self.cache_dir.display())?;
        writeln!(f, "Total Entries: {}", self.total_entries)?;
        writeln!(
            f,
            "Total Size: {:.2} MB",
            self.total_size_bytes as f64 / 1_048_576.0
        )?;

        if let Some(oldest) = self.oldest_entry {
            let dt = chrono::DateTime::from_timestamp(oldest, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            writeln!(f, "Oldest Entry: {}", dt)?;
        }
        if let Some(newest) = self.newest_entry {
            let dt = chrono::DateTime::from_timestamp(newest, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            writeln!(f, "Newest Entry: {}", dt)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_manager_creation() {
        let dir = tempdir().unwrap();
        let cache = CacheManager::with_dir(dir.path().to_path_buf());
        assert!(cache.is_ok());
    }

    #[test]
    fn test_cache_store_and_retrieve() {
        let dir = tempdir().unwrap();
        let cache = CacheManager::with_dir(dir.path().to_path_buf()).unwrap();

        let entry = CacheEntry {
            query: "test query".to_string(),
            model: "claude".to_string(),
            budget: 8000,
            file_hashes: HashMap::new(),
            packed_context: "packed content".to_string(),
            tokens_used: 500,
            created_at: chrono::Utc::now().timestamp(),
        };

        cache.store(&entry).unwrap();
        let retrieved = cache.get("test query", "claude", 8000).unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.query, "test query");
        assert_eq!(retrieved.packed_context, "packed content");
    }

    #[test]
    fn test_cache_miss() {
        let dir = tempdir().unwrap();
        let cache = CacheManager::with_dir(dir.path().to_path_buf()).unwrap();

        let retrieved = cache.get("nonexistent", "claude", 8000).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_invalidation_on_file_change() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let file_path = dir.path().join("test.rs");

        fs::write(&file_path, "original content").unwrap();

        let cache = CacheManager::with_dir(cache_dir).unwrap();

        let mut file_hashes = HashMap::new();
        file_hashes.insert(file_path.clone(), cache.hash_content("original content"));

        let entry = CacheEntry {
            query: "test".to_string(),
            model: "claude".to_string(),
            budget: 8000,
            file_hashes,
            packed_context: "packed".to_string(),
            tokens_used: 100,
            created_at: chrono::Utc::now().timestamp(),
        };

        cache.store(&entry).unwrap();

        // Should be valid initially
        let retrieved = cache.get("test", "claude", 8000).unwrap();
        assert!(retrieved.is_some());

        // Change the file
        fs::write(&file_path, "modified content").unwrap();

        // Should be invalidated
        let retrieved = cache.get("test", "claude", 8000).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let dir = tempdir().unwrap();
        let cache = CacheManager::with_dir(dir.path().to_path_buf()).unwrap();

        // Store multiple entries
        for i in 0..3 {
            let entry = CacheEntry {
                query: format!("query {}", i),
                model: "claude".to_string(),
                budget: 8000,
                file_hashes: HashMap::new(),
                packed_context: "content".to_string(),
                tokens_used: 100,
                created_at: chrono::Utc::now().timestamp(),
            };
            cache.store(&entry).unwrap();
        }

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 3);

        let cleared = cache.clear().unwrap();
        assert_eq!(cleared, 3);

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_hash_files() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let cache = CacheManager::with_dir(dir.path().join("cache")).unwrap();
        let hashes = cache.hash_files(&[file_path.clone()]).unwrap();

        assert_eq!(hashes.len(), 1);
        assert!(hashes.contains_key(&file_path));
    }
}
