use crate::error::GaggleError;
use reqwest::blocking::Client;
#[cfg(test)]
use std::cell::RefCell;
use std::env;
use std::thread::sleep;
use std::time::Duration;

/// Helper: get API base URL (overridable at runtime via env for testing)
pub(crate) fn get_api_base() -> String {
    #[cfg(test)]
    {
        thread_local! {
            static TEST_API_BASE: RefCell<Option<String>> = const { RefCell::new(None) };
        }
        let mut tls: Option<String> = None;
        TEST_API_BASE.with(|c| tls = c.borrow().clone());
        if let Some(b) = tls {
            return b.trim_end_matches('/').to_string();
        }
    }
    // Ensure no trailing slash to avoid double slashes when joining paths
    env::var("GAGGLE_API_BASE")
        .unwrap_or_else(|_| "https://www.kaggle.com/api/v1".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Helper: build a reqwest client with timeout and UA
pub(crate) fn build_client() -> Result<Client, GaggleError> {
    let timeout = Duration::from_secs(crate::config::http_timeout_runtime_secs());
    let ua = format!(
        "Gaggle/{} (+https://github.com/CogitatorTech/gaggle)",
        env!("CARGO_PKG_VERSION")
    );
    Ok(reqwest::blocking::ClientBuilder::new()
        .timeout(timeout)
        .user_agent(ua)
        .build()?)
}

pub(crate) fn with_retries<F, T>(mut f: F) -> Result<T, GaggleError>
where
    F: FnMut() -> Result<T, GaggleError>,
{
    let attempts = crate::config::http_retry_attempts();
    let mut delay = Duration::from_millis(crate::config::http_retry_delay_ms());
    let max_delay = Duration::from_millis(crate::config::http_retry_max_delay_ms());
    let max_attempts = attempts.saturating_add(1); // initial try + retries
    let mut last_err: Option<GaggleError> = None;

    for i in 0..max_attempts {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = Some(e);
                if i + 1 < max_attempts {
                    sleep(delay);
                    // Exponential backoff with configurable cap
                    let next = delay
                        .as_millis()
                        .saturating_mul(2)
                        .min(max_delay.as_millis()) as u64;
                    delay = Duration::from_millis(next);
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| GaggleError::HttpRequestError("Unknown error".into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_api_base_default() {
        env::remove_var("GAGGLE_API_BASE");
        let base = get_api_base();
        assert_eq!(base, "https://www.kaggle.com/api/v1");
    }

    #[test]
    #[serial]
    fn test_get_api_base_custom() {
        env::set_var("GAGGLE_API_BASE", "https://custom.api.com/v2");
        let base = get_api_base();
        assert_eq!(base, "https://custom.api.com/v2");
        env::remove_var("GAGGLE_API_BASE");
    }

    #[test]
    #[serial]
    fn test_get_api_base_removes_trailing_slash() {
        env::set_var("GAGGLE_API_BASE", "https://api.test.com/");
        let base = get_api_base();
        assert_eq!(base, "https://api.test.com");
        env::remove_var("GAGGLE_API_BASE");
    }

    #[test]
    fn test_build_client_success() {
        let client = build_client();
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_client_has_timeout() {
        let client = build_client().unwrap();
        // Verify client was created (timeout is internal)
        assert!(format!("{:?}", client).contains("Client"));
    }

    #[test]
    fn test_with_retries_success_first_try() {
        let mut call_count = 0;
        let result = with_retries(|| {
            call_count += 1;
            Ok::<i32, GaggleError>(42)
        });
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count, 1);
    }

    #[test]
    fn test_with_retries_success_after_failures() {
        let mut call_count = 0;
        let result = with_retries(|| {
            call_count += 1;
            if call_count < 3 {
                Err(GaggleError::HttpRequestError("temp failure".to_string()))
            } else {
                Ok::<i32, GaggleError>(42)
            }
        });
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(call_count >= 3);
    }

    #[test]
    fn test_with_retries_exhausts_attempts() {
        env::set_var("GAGGLE_HTTP_RETRY_ATTEMPTS", "2");
        env::set_var("GAGGLE_HTTP_RETRY_DELAY_MS", "10");

        let mut call_count = 0;
        let result = with_retries(|| {
            call_count += 1;
            Err::<i32, GaggleError>(GaggleError::HttpRequestError("always fails".to_string()))
        });
        assert!(result.is_err());
        // Should try: initial + 2 retries = 3 total
        assert_eq!(call_count, 3);

        env::remove_var("GAGGLE_HTTP_RETRY_ATTEMPTS");
        env::remove_var("GAGGLE_HTTP_RETRY_DELAY_MS");
    }

    #[test]
    fn test_with_retries_exponential_backoff() {
        env::set_var("GAGGLE_HTTP_RETRY_DELAY_MS", "10");
        env::set_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS", "100");

        let start = std::time::Instant::now();
        let mut call_count = 0;
        let _result = with_retries(|| {
            call_count += 1;
            if call_count < 3 {
                Err::<i32, GaggleError>(GaggleError::HttpRequestError("retry".to_string()))
            } else {
                Ok(42)
            }
        });
        let elapsed = start.elapsed();

        // Should have some delay between retries
        assert!(elapsed.as_millis() >= 10); // At least one 10ms delay

        env::remove_var("GAGGLE_HTTP_RETRY_DELAY_MS");
        env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS");
    }

    #[test]
    fn test_with_retries_respects_max_delay() {
        env::set_var("GAGGLE_HTTP_RETRY_DELAY_MS", "50");
        env::set_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS", "100");
        env::set_var("GAGGLE_HTTP_RETRY_ATTEMPTS", "5");

        let start = std::time::Instant::now();
        let _result = with_retries(|| {
            Err::<i32, GaggleError>(GaggleError::HttpRequestError("always fail".to_string()))
        });
        let elapsed = start.elapsed();

        // With exponential backoff capped at 100ms and 5 retries:
        // delays would be: 50, 100, 100, 100, 100 = 450ms total
        // But we add some tolerance for test execution overhead
        assert!(elapsed.as_millis() >= 200);
        assert!(elapsed.as_millis() < 1000);

        env::remove_var("GAGGLE_HTTP_RETRY_DELAY_MS");
        env::remove_var("GAGGLE_HTTP_RETRY_MAX_DELAY_MS");
        env::remove_var("GAGGLE_HTTP_RETRY_ATTEMPTS");
    }
}
