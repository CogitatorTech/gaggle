use once_cell::sync::Lazy;
#[cfg(test)]
use std::cell::RefCell;
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
    // Future: other options
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
            .filter(|s| !s.is_empty()) // Treat empty string as not set
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(DEFAULT_CACHE_DIR_NAME)
            })
    }

    /// Get verbose logging setting from GAGGLE_VERBOSE or default (false)
    fn get_verbose() -> bool {
        if let Ok(val) = env::var("GAGGLE_VERBOSE") {
            match val.to_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => true,
                "false" | "no" | "off" | "0" => false,
                _ => false,
            }
        } else {
            false
        }
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

/// Runtime-resolved cache directory (checks env each call, falls back to CONFIG)
pub fn cache_dir_runtime() -> PathBuf {
    // 1) Test-only thread-local override (highest precedence in tests)
    #[cfg(test)]
    {
        thread_local! {
            static OVERRIDE_CACHE_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
        }
        let mut tls: Option<PathBuf> = None;
        OVERRIDE_CACHE_DIR.with(|c| {
            tls = c.borrow().clone();
        });
        if let Some(p) = tls {
            return p;
        }
    }
    // 2) Environment variable
    if let Ok(val) = env::var("GAGGLE_CACHE_DIR") {
        if !val.is_empty() {
            return PathBuf::from(val);
        }
    }
    // 3) Fallback to static config
    CONFIG.cache_dir.clone()
}

/// Runtime-resolved HTTP timeout in seconds
pub fn http_timeout_runtime_secs() -> u64 {
    env::var("GAGGLE_HTTP_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CONFIG.http_timeout_secs)
}

/// HTTP retry attempts (default 3)
pub fn http_retry_attempts() -> u32 {
    env::var("GAGGLE_HTTP_RETRY_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3)
}

/// HTTP retry delay in milliseconds (default 1000)
pub fn http_retry_delay_ms() -> u64 {
    env::var("GAGGLE_HTTP_RETRY_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000)
}

/// HTTP retry max delay in milliseconds (default 30000)
pub fn http_retry_max_delay_ms() -> u64 {
    env::var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_default_config() {
        let config = GaggleConfig::default();
        assert!(!config.verbose_logging);
        assert_eq!(config.http_timeout_secs, 30);
    }

    #[test]
    #[serial]
    fn test_cache_dir_ends_with_gaggle_cache() {
        let config = GaggleConfig::default();
        assert!(config
            .cache_dir
            .to_str()
            .unwrap()
            .ends_with(DEFAULT_CACHE_DIR_NAME));
    }

    #[test]
    #[serial]
    fn test_config_from_env_default() {
        // Clear environment variables
        env::remove_var("GAGGLE_CACHE_DIR");
        env::remove_var("GAGGLE_VERBOSE");
        env::remove_var("GAGGLE_HTTP_TIMEOUT");

        let config = GaggleConfig::from_env();
        assert!(!config.verbose_logging);
        assert_eq!(config.http_timeout_secs, 30);
    }

    #[test]
    #[serial]
    fn test_get_cache_dir_default() {
        env::remove_var("GAGGLE_CACHE_DIR");
        let cache_dir = GaggleConfig::get_cache_dir();
        assert!(cache_dir.to_str().unwrap().contains(DEFAULT_CACHE_DIR_NAME));
    }

    #[test]
    #[serial]
    fn test_get_cache_dir_from_env() {
        env::set_var("GAGGLE_CACHE_DIR", "/tmp/test_cache");
        let cache_dir = GaggleConfig::get_cache_dir();
        assert_eq!(cache_dir, PathBuf::from("/tmp/test_cache"));
        env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    #[serial]
    fn test_get_verbose_false() {
        env::remove_var("GAGGLE_VERBOSE");
        assert!(!GaggleConfig::get_verbose());
    }

    #[test]
    #[serial]
    fn test_get_verbose_true() {
        env::set_var("GAGGLE_VERBOSE", "true");
        assert!(GaggleConfig::get_verbose());
        env::remove_var("GAGGLE_VERBOSE");
    }

    #[test]
    #[serial]
    fn test_get_verbose_one() {
        env::set_var("GAGGLE_VERBOSE", "1");
        let result = GaggleConfig::get_verbose();
        env::remove_var("GAGGLE_VERBOSE");
        assert!(result); // '1' should be treated as true
    }

    #[test]
    #[serial]
    fn test_get_verbose_invalid() {
        env::set_var("GAGGLE_VERBOSE", "invalid");
        assert!(!GaggleConfig::get_verbose());
        env::remove_var("GAGGLE_VERBOSE");
    }

    #[test]
    #[serial]
    fn test_get_http_timeout_default() {
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
        assert_eq!(GaggleConfig::get_http_timeout(), 30);
    }

    #[test]
    #[serial]
    fn test_get_http_timeout_custom() {
        env::set_var("GAGGLE_HTTP_TIMEOUT", "60");
        assert_eq!(GaggleConfig::get_http_timeout(), 60);
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    #[serial]
    fn test_get_http_timeout_zero() {
        env::set_var("GAGGLE_HTTP_TIMEOUT", "0");
        assert_eq!(GaggleConfig::get_http_timeout(), 0);
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    #[serial]
    fn test_get_http_timeout_large_value() {
        env::set_var("GAGGLE_HTTP_TIMEOUT", "3600");
        assert_eq!(GaggleConfig::get_http_timeout(), 3600);
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    #[serial]
    fn test_get_http_timeout_invalid() {
        env::set_var("GAGGLE_HTTP_TIMEOUT", "not_a_number");
        assert_eq!(GaggleConfig::get_http_timeout(), 30); // Falls back to default
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    #[serial]
    fn test_get_http_timeout_negative() {
        env::set_var("GAGGLE_HTTP_TIMEOUT", "-1");
        assert_eq!(GaggleConfig::get_http_timeout(), 30); // Falls back to default
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }

    #[test]
    #[serial]
    fn test_http_retry_defaults() {
        env::remove_var("GAGGLE_HTTP_RETRY_ATTEMPTS");
        env::remove_var("GAGGLE_HTTP_RETRY_DELAY_MS");
        env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS");
        assert_eq!(http_retry_attempts(), 3);
        assert_eq!(http_retry_delay_ms(), 1000);
        assert_eq!(http_retry_max_delay_ms(), 30_000);
    }

    #[test]
    #[serial]
    fn test_http_retry_env() {
        env::set_var("GAGGLE_HTTP_RETRY_ATTEMPTS", "3");
        env::set_var("GAGGLE_HTTP_RETRY_DELAY_MS", "250");
        assert_eq!(http_retry_attempts(), 3);
        assert_eq!(http_retry_delay_ms(), 250);
        env::remove_var("GAGGLE_HTTP_RETRY_ATTEMPTS");
        env::remove_var("GAGGLE_HTTP_RETRY_DELAY_MS");
    }

    #[test]
    #[serial]
    fn test_http_retry_max_delay_configurable() {
        let prev = env::var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS").ok();
        env::set_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS", "5000");
        let max_delay = http_retry_max_delay_ms();
        assert_eq!(max_delay, 5000);
        if let Some(v) = prev {
            env::set_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS", v);
        } else {
            env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS");
        }
    }

    #[test]
    #[serial]
    fn test_http_retry_max_delay_default() {
        env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS");
        let max_delay = http_retry_max_delay_ms();
        assert_eq!(max_delay, 30_000);
    }

    #[test]
    #[serial]
    fn test_cache_dir_path_format() {
        let config = GaggleConfig::default();
        let path_str = config.cache_dir.to_str().unwrap();
        assert!(!path_str.is_empty());
        assert!(path_str.contains(DEFAULT_CACHE_DIR_NAME));
    }

    #[test]
    #[serial]
    fn test_config_clone() {
        let config1 = GaggleConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.verbose_logging, config2.verbose_logging);
        assert_eq!(config1.http_timeout_secs, config2.http_timeout_secs);
    }

    #[test]
    #[serial]
    fn test_config_debug_format() {
        let config = GaggleConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("GaggleConfig"));
    }

    #[test]
    #[serial]
    fn test_multiple_config_instances() {
        let config1 = GaggleConfig::from_env();
        let config2 = GaggleConfig::from_env();
        assert_eq!(config1.http_timeout_secs, config2.http_timeout_secs);
    }

    #[test]
    #[serial]
    fn test_cache_dir_with_special_env_var() {
        env::set_var("GAGGLE_CACHE_DIR", "/tmp/test_gaggle_$HOME");
        let cache_dir = GaggleConfig::get_cache_dir();
        // Should treat it as literal path, not expand $HOME
        assert_eq!(cache_dir, PathBuf::from("/tmp/test_gaggle_$HOME"));
        env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    #[serial]
    fn test_empty_cache_dir_env() {
        env::set_var("GAGGLE_CACHE_DIR", "");
        let cache_dir = GaggleConfig::get_cache_dir();
        // Empty string in env var should be treated as "not set" and use default
        assert!(cache_dir.to_str().unwrap().contains(DEFAULT_CACHE_DIR_NAME));
        env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    #[serial]
    fn test_verbose_parsing_one_zero() {
        env::set_var("GAGGLE_VERBOSE", "1");
        assert!(GaggleConfig::get_verbose());
        env::set_var("GAGGLE_VERBOSE", "0");
        assert!(!GaggleConfig::get_verbose());
        env::remove_var("GAGGLE_VERBOSE");
    }

    #[test]
    #[serial]
    fn test_cache_dir_runtime_env_override() {
        let temp = tempfile::TempDir::new().unwrap();
        env::set_var("GAGGLE_CACHE_DIR", temp.path());
        let dir = cache_dir_runtime();
        assert_eq!(dir, temp.path());
        env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    #[serial]
    fn test_http_timeout_runtime_env_override() {
        env::set_var("GAGGLE_HTTP_TIMEOUT", "42");
        assert_eq!(http_timeout_runtime_secs(), 42);
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
    }
}
