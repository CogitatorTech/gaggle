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

    #[test]
    fn test_config_from_env_default() {
        // Clear environment variables
        std::env::remove_var("GAGGLE_CACHE_DIR");
        std::env::remove_var("GAGGLE_VERBOSE");
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");

        let config = GaggleConfig::from_env();
        assert!(!config.verbose_logging);
        assert_eq!(config.http_timeout_secs, 30);
    }

    #[test]
    fn test_get_cache_dir_default() {
        std::env::remove_var("GAGGLE_CACHE_DIR");
        let cache_dir = GaggleConfig::get_cache_dir();
        assert!(cache_dir.to_str().unwrap().contains(DEFAULT_CACHE_DIR_NAME));
    }

    #[test]
    fn test_get_cache_dir_from_env() {
        std::env::set_var("GAGGLE_CACHE_DIR", "/tmp/test_cache");
        let cache_dir = GaggleConfig::get_cache_dir();
        assert_eq!(cache_dir, PathBuf::from("/tmp/test_cache"));
        std::env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    fn test_get_verbose_false() {
        std::env::remove_var("GAGGLE_VERBOSE");
        assert!(!GaggleConfig::get_verbose());
    }

    #[test]
    fn test_get_verbose_true() {
        std::env::set_var("GAGGLE_VERBOSE", "true");
        assert!(GaggleConfig::get_verbose());
        std::env::remove_var("GAGGLE_VERBOSE");
    }

    #[test]
    fn test_get_verbose_one() {
        std::env::set_var("GAGGLE_VERBOSE", "1");
        // "1" doesn't parse as a bool in Rust, so it falls back to default (false)
        let result = GaggleConfig::get_verbose();
        std::env::remove_var("GAGGLE_VERBOSE");
        assert!(!result); // Should be false because "1" doesn't parse as bool
    }

    #[test]
    fn test_get_verbose_invalid() {
        std::env::set_var("GAGGLE_VERBOSE", "invalid");
        assert!(!GaggleConfig::get_verbose());
        std::env::remove_var("GAGGLE_VERBOSE");
    }

    #[test]
    fn test_get_http_timeout_default() {
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");
        assert_eq!(GaggleConfig::get_http_timeout(), 30);
    }

    #[test]
    fn test_get_http_timeout_custom() {
        std::env::set_var("GAGGLE_HTTP_TIMEOUT", "60");
        assert_eq!(GaggleConfig::get_http_timeout(), 60);
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    fn test_get_http_timeout_zero() {
        std::env::set_var("GAGGLE_HTTP_TIMEOUT", "0");
        assert_eq!(GaggleConfig::get_http_timeout(), 0);
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    fn test_get_http_timeout_large_value() {
        std::env::set_var("GAGGLE_HTTP_TIMEOUT", "3600");
        assert_eq!(GaggleConfig::get_http_timeout(), 3600);
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    fn test_get_http_timeout_invalid() {
        std::env::set_var("GAGGLE_HTTP_TIMEOUT", "not_a_number");
        assert_eq!(GaggleConfig::get_http_timeout(), 30); // Falls back to default
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    fn test_get_http_timeout_negative() {
        std::env::set_var("GAGGLE_HTTP_TIMEOUT", "-1");
        assert_eq!(GaggleConfig::get_http_timeout(), 30); // Falls back to default
        std::env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    fn test_cache_dir_path_format() {
        let config = GaggleConfig::default();
        let path_str = config.cache_dir.to_str().unwrap();
        assert!(!path_str.is_empty());
        assert!(path_str.contains(DEFAULT_CACHE_DIR_NAME));
    }

    #[test]
    fn test_config_clone() {
        let config1 = GaggleConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.verbose_logging, config2.verbose_logging);
        assert_eq!(config1.http_timeout_secs, config2.http_timeout_secs);
    }

    #[test]
    fn test_config_debug_format() {
        let config = GaggleConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("GaggleConfig"));
    }

    #[test]
    fn test_multiple_config_instances() {
        let config1 = GaggleConfig::from_env();
        let config2 = GaggleConfig::from_env();
        assert_eq!(config1.http_timeout_secs, config2.http_timeout_secs);
    }

    #[test]
    fn test_cache_dir_with_special_env_var() {
        std::env::set_var("GAGGLE_CACHE_DIR", "/tmp/test_gaggle_$HOME");
        let cache_dir = GaggleConfig::get_cache_dir();
        // Should treat it as literal path, not expand $HOME
        assert_eq!(cache_dir, PathBuf::from("/tmp/test_gaggle_$HOME"));
        std::env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    fn test_empty_cache_dir_env() {
        std::env::set_var("GAGGLE_CACHE_DIR", "");
        let cache_dir = GaggleConfig::get_cache_dir();
        // Empty string should be treated as valid
        assert_eq!(cache_dir, PathBuf::from(""));
        std::env::remove_var("GAGGLE_CACHE_DIR");
    }
}
