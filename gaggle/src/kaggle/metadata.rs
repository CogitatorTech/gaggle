// metadata.rs
//
// This module provides functionality for retrieving metadata about Kaggle datasets.
// It includes a struct for representing dataset information, as well as functions
// for fetching metadata from the Kaggle API and for determining the current version
// of a dataset. The module also includes a simple in-memory cache with a TTL
// to reduce the number of API calls for frequently accessed metadata.

use crate::error::GaggleError;
use serde::{Deserialize, Serialize};

use super::api::{build_client, get_api_base, with_retries};
use super::credentials::get_credentials;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A struct that represents information about a Kaggle dataset.
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DatasetInfo {
    /// The reference path of the dataset, in the format `owner/dataset`.
    pub ref_path: String,
    /// The title of the dataset.
    pub title: String,
    /// The size of the dataset in bytes.
    pub size: u64,
    /// The URL of the dataset.
    pub url: String,
    /// The date the dataset was last updated.
    pub last_updated: String,
}

/// Simple in-memory cache for dataset metadata with TTL
static META_CACHE: once_cell::sync::Lazy<RwLock<HashMap<String, (serde_json::Value, Instant)>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Metadata cache TTL (seconds), configurable via GAGGLE_METADATA_TTL (default 600s)
fn metadata_ttl() -> Duration {
    let secs = std::env::var("GAGGLE_METADATA_TTL")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(600);
    Duration::from_secs(secs)
}

/// Retrieves the metadata for a specific dataset.
pub fn get_dataset_metadata(dataset_path: &str) -> Result<serde_json::Value, GaggleError> {
    if crate::config::offline_mode() {
        return Err(GaggleError::HttpRequestError(
            format!(
                "Offline mode enabled; metadata fetch for '{}' is disabled. Unset GAGGLE_OFFLINE to enable network.",
                dataset_path
            ),
        ));
    }

    // Serve from cache when fresh
    if let Some((val, ts)) = META_CACHE.read().get(dataset_path).cloned() {
        if ts.elapsed() < metadata_ttl() {
            return Ok(val);
        }
    }

    let creds = get_credentials()?;
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    let url = format!("{}/datasets/view/{}/{}", get_api_base(), owner, dataset);

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
            "Failed to get dataset metadata: HTTP {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response.json()?;

    // Store in cache
    META_CACHE
        .write()
        .insert(dataset_path.to_string(), (json.clone(), Instant::now()));

    Ok(json)
}

/// Retrieves the current version number of a dataset from the Kaggle API.
pub fn get_current_version(dataset_path: &str) -> Result<String, GaggleError> {
    if crate::config::offline_mode() {
        // In offline mode, try to use cached marker file version if available
        let (owner, dataset) = super::parse_dataset_path(dataset_path)?;
        let cache_dir = crate::config::cache_dir_runtime()
            .join("datasets")
            .join(&owner)
            .join(&dataset);
        let marker = cache_dir.join(".downloaded");
        if let Ok(content) = std::fs::read_to_string(&marker) {
            if !content.is_empty() {
                if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(v) = meta.get("version").and_then(|x| x.as_str()) {
                        return Ok(v.to_string());
                    }
                }
            }
        }
        // Fallback when no cached version is available
        return Ok("unknown".to_string());
    }

    let metadata = get_dataset_metadata(dataset_path)?;

    // Try to extract version from metadata
    // Kaggle API returns version in various fields depending on endpoint
    if let Some(version) = metadata.get("currentVersionNumber") {
        if let Some(v) = version.as_i64() {
            return Ok(v.to_string());
        }
        if let Some(v) = version.as_str() {
            return Ok(v.to_string());
        }
    }

    if let Some(version) = metadata.get("versions") {
        if let Some(arr) = version.as_array() {
            if let Some(latest) = arr.first() {
                if let Some(v) = latest.get("versionNumber") {
                    if let Some(num) = v.as_i64() {
                        return Ok(num.to_string());
                    }
                }
            }
        }
    }

    // Default to "1" if version info not available
    Ok("1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_info_struct() {
        let info = DatasetInfo {
            ref_path: "owner/dataset".to_string(),
            title: "Test Dataset".to_string(),
            size: 1024000,
            url: "https://kaggle.com/datasets/owner/dataset".to_string(),
            last_updated: "2024-01-01".to_string(),
        };

        assert_eq!(info.ref_path, "owner/dataset");
        assert_eq!(info.title, "Test Dataset");
        assert_eq!(info.size, 1024000);
    }

    #[test]
    fn test_dataset_info_serialization() {
        let info = DatasetInfo {
            ref_path: "owner/dataset".to_string(),
            title: "Test".to_string(),
            size: 1000,
            url: "https://test.com".to_string(),
            last_updated: "2024-01-01".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("owner/dataset"));
        assert!(json.contains("Test"));

        let deserialized: DatasetInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ref_path, info.ref_path);
        assert_eq!(deserialized.size, info.size);
    }

    #[test]
    fn test_get_dataset_metadata_invalid_path() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // Invalid path format should be caught by parse_dataset_path
        let result = get_dataset_metadata("invalid");
        assert!(result.is_err());

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_get_dataset_metadata_valid_path_format() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // Valid path format, but will fail at HTTP level
        let result = get_dataset_metadata("owner/dataset");
        assert!(result.is_err());
        // Should be HTTP error, not path parsing error
        if let Err(GaggleError::InvalidDatasetPath(_)) = result {
            panic!("Should not have path validation error");
        }

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }
}
