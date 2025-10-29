// Centralized configuration management for Gaggle

use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;

const DEFAULT_CACHE_DIR_NAME: &str = "gaggle_cache";

pub static CONFIG: Lazy<GaggleConfig> = Lazy::new(GaggleConfig::from_env);

/// Configuration options for Gaggle
#[derive(Debug, Clone)]
pub struct GaggleConfig {
    /// Directory for caching downloaded datasets
    pub cache_dir: PathBuf,
    /// Enable verbose logging
    #[allow(dead_code)]
    pub verbose_logging: bool,
    /// HTTP timeout in seconds
    #[allow(dead_code)]
    pub http_timeout_secs: u64,
}

impl GaggleConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            cache_dir: Self::get_cache_dir(),
            verbose_logging: Self::get_verbose(),
            http_timeout_secs: Self::get_http_timeout(),
        }
    }

    /// Get cache directory from GAGGLE_CACHE_DIR or default
    fn get_cache_dir() -> PathBuf {
        env::var("GAGGLE_CACHE_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(DEFAULT_CACHE_DIR_NAME)
            })
    }

    /// Get verbose logging setting from GAGGLE_VERBOSE or default (false)
    fn get_verbose() -> bool {
        env::var("GAGGLE_VERBOSE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false)
    }

    /// Get HTTP timeout from GAGGLE_HTTP_TIMEOUT or default (30 seconds)
    fn get_http_timeout() -> u64 {
        env::var("GAGGLE_HTTP_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30)
    }
}

impl Default for GaggleConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GaggleConfig::default();
        assert!(!config.verbose_logging);
        assert_eq!(config.http_timeout_secs, 30);
    }

    #[test]
    fn test_cache_dir_ends_with_gaggle_cache() {
        let config = GaggleConfig::default();
        assert!(config
            .cache_dir
            .to_str()
            .unwrap()
            .ends_with(DEFAULT_CACHE_DIR_NAME));
    }
}

