use crate::error::GaggleError;

use super::api::{build_client, get_api_base, with_retries};
use super::credentials::get_credentials;

/// Search for datasets on Kaggle
pub fn search_datasets(
    query: &str,
    page: i32,
    page_size: i32,
) -> Result<serde_json::Value, GaggleError> {
    // Strict offline: fail fast
    if crate::config::offline_mode() {
        return Err(GaggleError::HttpRequestError(
            "Offline mode enabled; search is disabled. Unset GAGGLE_OFFLINE to enable network."
                .to_string(),
        ));
    }

    // Validate inputs
    if page < 1 {
        return Err(GaggleError::InvalidDatasetPath(format!(
            "Page number must be >= 1, got: {}",
            page
        )));
    }
    if !(1..=100).contains(&page_size) {
        return Err(GaggleError::InvalidDatasetPath(format!(
            "Page size must be between 1 and 100, got: {}",
            page_size
        )));
    }

    let creds = get_credentials()?;

    let url = format!(
        "{}/datasets/list?search={}&page={}&pageSize={}",
        get_api_base(),
        urlencoding::encode(query),
        page,
        page_size
    );

    let client = build_client()?;
    let response = with_retries(|| {
        client
            .get(&url)
            .basic_auth(&creds.username, Some(&creds.key))
            .send()
            .map_err(|e| GaggleError::HttpRequestError(e.to_string()))
    })?;

    if !response.status().is_success() {
        return Err(GaggleError::HttpRequestError(format!(
            "Failed to search datasets: HTTP {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response.json()?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_datasets_validates_page() {
        // Mock credentials to avoid actual API calls
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        let result = search_datasets("query", 0, 10);
        assert!(result.is_err());
        if let Err(GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("Page number must be >= 1"));
        }

        let result = search_datasets("query", -5, 10);
        assert!(result.is_err());

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_search_datasets_validates_page_size() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // Page size too small
        let result = search_datasets("query", 1, 0);
        assert!(result.is_err());
        if let Err(GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("Page size must be between 1 and 100"));
        }

        // Page size too large
        let result = search_datasets("query", 1, 101);
        assert!(result.is_err());
        if let Err(GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("Page size must be between 1 and 100"));
        }

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_search_datasets_valid_parameters() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // These should pass validation (even though they'll fail at HTTP level without mock)
        let result = search_datasets("valid query", 1, 10);
        // Will likely fail at HTTP level, but validation should pass
        // Could also succeed if there's a real kaggle.json file
        match result {
            Ok(_) => {
                // Unexpectedly succeeded (maybe real credentials exist)
            }
            Err(e) => {
                // Should be HTTP error, not validation error
                if let GaggleError::InvalidDatasetPath(_) = e {
                    panic!("Should not have validation error for valid params");
                }
            }
        }

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_search_datasets_page_boundary_values() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // Minimum valid values
        let result = search_datasets("query", 1, 1);
        // HTTP error expected, but might succeed with real credentials
        match result {
            Ok(_) => {} // Succeeded with real credentials
            Err(e) => {
                // Should not be a validation error
                if let GaggleError::InvalidDatasetPath(_) = e {
                    panic!("Should not have validation error for page=1, size=1");
                }
            }
        }

        // Maximum valid values - just test that validation passes
        let _result = search_datasets("query", i32::MAX, 100);
        // Don't assert on result since it depends on network/credentials

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_search_datasets_url_encoding() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // Query with special characters that need encoding
        let _result = search_datasets("machine learning & AI", 1, 10);
        // Should not panic on URL encoding
        // Don't assert failure since it might succeed with real credentials

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }
}
