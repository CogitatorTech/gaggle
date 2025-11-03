use once_cell::sync::Lazy;

#[cfg(test)]
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;

const DEFAULT_CACHE_DIR_NAME: &str = "gaggle";

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
    /// Download lock wait timeout in milliseconds
    #[allow(dead_code)]
    pub download_wait_timeout_ms: u64,
    /// Download lock poll interval in milliseconds
    #[allow(dead_code)]
    pub download_wait_poll_ms: u64,
    // Future: other options
}

impl GaggleConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            cache_dir: Self::get_cache_dir(),
            verbose_logging: Self::get_verbose(),
            http_timeout_secs: Self::get_http_timeout(),
            download_wait_timeout_ms: Self::get_download_wait_timeout_ms(),
            download_wait_poll_ms: Self::get_download_wait_poll_ms(),
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

    /// Get download wait timeout from env (default 30_000 ms)
    fn get_download_wait_timeout_ms() -> u64 {
        env::var("GAGGLE_DOWNLOAD_WAIT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .map(|secs| (secs * 1000.0).round() as u64)
            .unwrap_or(30_000)
    }

    /// Get download wait poll interval from env (default 100 ms)
    fn get_download_wait_poll_ms() -> u64 {
        env::var("GAGGLE_DOWNLOAD_WAIT_POLL")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .map(|secs| (secs * 1000.0).round() as u64)
            .unwrap_or(100)
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
    env::var("GAGGLE_HTTP_RETRY_DELAY")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0).round() as u64)
        .unwrap_or(1000)
}

/// HTTP retry max delay in milliseconds (default 30000)
pub fn http_retry_max_delay_ms() -> u64 {
    env::var("GAGGLE_HTTP_RETRY_MAX_DELAY")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0).round() as u64)
        .unwrap_or(30000)
}

/// Cache size limit in megabytes (default 100GB = 102400 MB)
/// Returns None if unlimited
pub fn cache_size_limit_mb() -> Option<u64> {
    match env::var("GAGGLE_CACHE_SIZE_LIMIT").ok() {
        Some(val) if val.to_lowercase() == "unlimited" => None,
        Some(val) => val.parse().ok(),
        None => Some(102400), // Default 100GB
    }
}

/// Whether cache limit is a soft limit (default true)
/// Soft limit allows download to complete even if it exceeds limit,
/// then triggers cleanup afterwards
pub fn cache_limit_is_soft() -> bool {
    env::var("GAGGLE_CACHE_HARD_LIMIT")
        .ok()
        .map(|v| !matches!(v.to_lowercase().as_str(), "true" | "yes" | "1"))
        .unwrap_or(true)
}

/// Runtime-resolved download wait timeout in milliseconds
pub fn download_wait_timeout_ms() -> u64 {
    env::var("GAGGLE_DOWNLOAD_WAIT_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0).round() as u64)
        .unwrap_or(CONFIG.download_wait_timeout_ms)
}

/// Runtime-resolved download wait poll interval in milliseconds
pub fn download_wait_poll_interval_ms() -> u64 {
    env::var("GAGGLE_DOWNLOAD_WAIT_POLL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0).round() as u64)
        .unwrap_or(CONFIG.download_wait_poll_ms)
}

/// Whether offline mode is enabled (disables network operations). Controlled by GAGGLE_OFFLINE
pub fn offline_mode() -> bool {
    std::env::var("GAGGLE_OFFLINE")
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Whether strict on-demand mode is enabled. When true, gaggle_get_file_path will NOT fall back to
/// full dataset download if single-file fetch fails.
pub fn strict_on_demand() -> bool {
    std::env::var("GAGGLE_STRICT_ONDEMAND")
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_default_config() {
        let config = GaggleConfig::default();
        assert!(!config.verbose_logging);
        assert_eq!(config.http_timeout_secs, 30);
        assert!(config.download_wait_timeout_ms >= 1000);
        assert!(config.download_wait_poll_ms > 0);
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
        env::remove_var("GAGGLE_HTTP_RETRY_DELAY");
        env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY");
        assert_eq!(http_retry_attempts(), 3);
        assert_eq!(http_retry_delay_ms(), 1000);
        assert_eq!(http_retry_max_delay_ms(), 30_000);
    }

    #[test]
    #[serial]
    fn test_http_retry_env() {
        env::set_var("GAGGLE_HTTP_RETRY_ATTEMPTS", "3");
        env::set_var("GAGGLE_HTTP_RETRY_DELAY", "0.25");
        assert_eq!(http_retry_attempts(), 3);
        assert_eq!(http_retry_delay_ms(), 250);
        env::remove_var("GAGGLE_HTTP_RETRY_ATTEMPTS");
        env::remove_var("GAGGLE_HTTP_RETRY_DELAY");
    }

    #[test]
    #[serial]
    fn test_http_retry_max_delay_configurable() {
        let prev = env::var("GAGGLE_HTTP_RETRY_MAX_DELAY").ok();
        env::set_var("GAGGLE_HTTP_RETRY_MAX_DELAY", "5");
        let max_delay = http_retry_max_delay_ms();
        assert_eq!(max_delay, 5000);
        if let Some(v) = prev {
            env::set_var("GAGGLE_HTTP_RETRY_MAX_DELAY", v);
        } else {
            env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY");
        }
    }

    #[test]
    #[serial]
    fn test_http_retry_max_delay_default() {
        env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY");
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

    #[test]
    #[serial]
    fn test_cache_size_limit_default() {
        env::remove_var("GAGGLE_CACHE_SIZE_LIMIT");
        let limit = cache_size_limit_mb();
        assert_eq!(limit, Some(102400)); // 100GB default
    }

    #[test]
    #[serial]
    fn test_cache_size_limit_custom() {
        env::set_var("GAGGLE_CACHE_SIZE_LIMIT", "50000");
        let limit = cache_size_limit_mb();
        assert_eq!(limit, Some(50000));
        env::remove_var("GAGGLE_CACHE_SIZE_LIMIT");
    }

    #[test]
    #[serial]
    fn test_cache_size_limit_unlimited() {
        env::set_var("GAGGLE_CACHE_SIZE_LIMIT", "unlimited");
        let limit = cache_size_limit_mb();
        assert_eq!(limit, None);
        env::remove_var("GAGGLE_CACHE_SIZE_LIMIT");
    }

    #[test]
    #[serial]
    fn test_cache_limit_soft_by_default() {
        env::remove_var("GAGGLE_CACHE_HARD_LIMIT");
        assert!(cache_limit_is_soft());
    }

    #[test]
    #[serial]
    fn test_cache_limit_hard() {
        env::set_var("GAGGLE_CACHE_HARD_LIMIT", "true");
        assert!(!cache_limit_is_soft());
        env::remove_var("GAGGLE_CACHE_HARD_LIMIT");
    }

    #[test]
    #[serial]
    fn test_download_wait_runtime_overrides() {
        env::set_var("GAGGLE_DOWNLOAD_WAIT_TIMEOUT", "1.234");
        env::set_var("GAGGLE_DOWNLOAD_WAIT_POLL", "0.017");
        assert_eq!(download_wait_timeout_ms(), 1234);
        assert_eq!(download_wait_poll_interval_ms(), 17);
        env::remove_var("GAGGLE_DOWNLOAD_WAIT_TIMEOUT");
        env::remove_var("GAGGLE_DOWNLOAD_WAIT_POLL");
    }

    #[test]
    #[serial]
    fn test_offline_mode_env_parsing() {
        std::env::remove_var("GAGGLE_OFFLINE");
        assert!(!offline_mode());
        std::env::set_var("GAGGLE_OFFLINE", "1");
        assert!(offline_mode());
        std::env::set_var("GAGGLE_OFFLINE", "true");
        assert!(offline_mode());
        std::env::set_var("GAGGLE_OFFLINE", "no");
        assert!(!offline_mode());
        std::env::remove_var("GAGGLE_OFFLINE");
    }

    #[test]
    #[serial]
    fn test_strict_on_demand_env_parsing() {
        std::env::remove_var("GAGGLE_STRICT_ONDEMAND");
        assert!(!strict_on_demand());
        std::env::set_var("GAGGLE_STRICT_ONDEMAND", "1");
        assert!(strict_on_demand());
        std::env::set_var("GAGGLE_STRICT_ONDEMAND", "true");
        assert!(strict_on_demand());
        std::env::set_var("GAGGLE_STRICT_ONDEMAND", "off");
        assert!(!strict_on_demand());
        std::env::remove_var("GAGGLE_STRICT_ONDEMAND");
    }
}
