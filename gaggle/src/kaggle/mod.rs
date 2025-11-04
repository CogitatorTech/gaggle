// mod.rs
//
// This module serves as the main entry point for the Kaggle functionality in the Gaggle
// library. It re-exports the public functions from the other modules in this directory,
// providing a single, consistent interface for interacting with the Kaggle API. It also
// contains the core logic for parsing dataset paths, which is a critical component for
// all of the other functionality in this library.

pub mod api;
pub mod credentials;
pub mod download;
pub mod metadata;
pub mod search;

pub use download::{
    download_dataset, get_dataset_file_path, get_dataset_version_info, is_dataset_current,
    list_dataset_files, update_dataset,
};
pub use metadata::get_dataset_metadata;
pub use search::search_datasets;

/// Parse dataset path like "username/dataset-name"
///
/// # Arguments
/// * `path` - A string in format "owner/dataset-name"
///
/// # Returns
/// A tuple of (owner, dataset) if valid
///
/// # Errors
/// Returns InvalidDatasetPath error if:
/// - Path doesn't contain exactly one '/'
/// - Owner or dataset name is empty after trimming
/// - Path contains traversal segments (. or ..)
/// - Path contains control characters
/// - Path exceeds maximum length (4096 characters)
pub fn parse_dataset_path(path: &str) -> Result<(String, String), crate::error::GaggleError> {
    // Validate maximum path length to prevent resource exhaustion
    const MAX_PATH_LENGTH: usize = 4096;
    if path.len() > MAX_PATH_LENGTH {
        return Err(crate::error::GaggleError::InvalidDatasetPath(format!(
            "Dataset path exceeds maximum length of {} characters",
            MAX_PATH_LENGTH
        )));
    }

    // Normalize surrounding whitespace to avoid accidental control chars in names
    let trimmed = path.trim();
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return Err(crate::error::GaggleError::InvalidDatasetPath(format!(
            "Dataset path must be in format 'owner/dataset-name', got: {}",
            path
        )));
    }

    // Trim inner segments as well (e.g., " owner / dataset ")
    let owner = parts[0].trim();
    let dataset = parts[1].trim();

    // Validate that both owner and dataset are non-empty
    if owner.is_empty() || dataset.is_empty() {
        return Err(crate::error::GaggleError::InvalidDatasetPath(format!(
            "Dataset path cannot have empty owner or dataset name, got: {}",
            path
        )));
    }

    // Security: reject traversal/dot components
    if owner == ".." || dataset == ".." || owner == "." || dataset == "." {
        return Err(crate::error::GaggleError::InvalidDatasetPath(format!(
            "Dataset path contains invalid traversal segments: {}",
            path
        )));
    }

    // Optional: reject ASCII control characters within segments
    if owner.chars().any(|c| c.is_control()) || dataset.chars().any(|c| c.is_control()) {
        return Err(crate::error::GaggleError::InvalidDatasetPath(format!(
            "Dataset path contains control characters: {}",
            path
        )));
    }

    Ok((owner.to_string(), dataset.to_string()))
}

/// Parse dataset path with optional version
/// Supports formats:
///   "owner/dataset" -> (owner, dataset, None)
///   "owner/dataset@v2" -> (owner, dataset, Some("2"))
///   "owner/dataset@5" -> (owner, dataset, Some("5"))
///   "owner/dataset@latest" -> (owner, dataset, None)
pub fn parse_dataset_path_with_version(
    path: &str,
) -> Result<(String, String, Option<String>), crate::error::GaggleError> {
    // Split on @ to extract version
    let parts: Vec<&str> = path.split('@').collect();

    if parts.len() > 2 {
        return Err(crate::error::GaggleError::InvalidDatasetPath(
            "Dataset path can only contain one @ for version specification".to_string(),
        ));
    }

    let dataset_path = parts[0];
    let version = if parts.len() == 2 {
        let v = parts[1].trim();
        if v == "latest" || v.is_empty() {
            None
        } else {
            // Remove 'v' prefix if present (both @v2 and @2 are valid)
            let version_str = v.strip_prefix('v').unwrap_or(v);
            // Validate it's a positive integer (>0)
            match version_str.parse::<u32>() {
                Ok(n) if n > 0 => Some(version_str.to_string()),
                _ => {
                    return Err(crate::error::GaggleError::InvalidDatasetPath(format!(
                        "Invalid version number '{}'. Version must be a positive integer > 0.",
                        v
                    )));
                }
            }
        }
    } else {
        None
    };

    // Parse owner/dataset from the base path
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

    Ok((owner, dataset, version))
}

/// Prefetch multiple files within a dataset without downloading the entire archive.
/// Returns a JSON string with an array of objects: {"name": ..., "status": "ok"|"error", "path"?: ..., "error"?: ...}
#[allow(dead_code)]
pub fn prefetch_files(
    dataset_path: &str,
    files: &[&str],
) -> Result<serde_json::Value, crate::error::GaggleError> {
    let mut results = Vec::with_capacity(files.len());
    for f in files {
        match download::get_dataset_file_path(dataset_path, f) {
            Ok(path) => {
                results.push(serde_json::json!({
                    "name": f,
                    "status": "ok",
                    "path": path.to_string_lossy(),
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "name": f,
                    "status": "error",
                    "error": e.to_string(),
                }));
            }
        }
    }
    Ok(serde_json::json!({"dataset": dataset_path, "files": results}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dataset_path_valid() {
        let result = parse_dataset_path("owner/dataset-name");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset-name");
    }

    #[test]
    fn test_parse_dataset_path_with_numbers() {
        let result = parse_dataset_path("user123/dataset456");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "user123");
        assert_eq!(dataset, "dataset456");
    }

    #[test]
    fn test_parse_dataset_path_with_hyphens() {
        let result = parse_dataset_path("my-owner/my-dataset");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "my-owner");
        assert_eq!(dataset, "my-dataset");
    }

    #[test]
    fn test_parse_dataset_path_with_underscores() {
        let result = parse_dataset_path("user_name/data_set");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "user_name");
        assert_eq!(dataset, "data_set");
    }

    #[test]
    fn test_parse_dataset_path_no_slash() {
        let result = parse_dataset_path("ownerdataset");
        assert!(result.is_err());
        match result {
            Err(crate::error::GaggleError::InvalidDatasetPath(msg)) => {
                assert!(msg.contains("must be in format"));
            }
            _ => panic!("Expected InvalidDatasetPath error"),
        }
    }

    #[test]
    fn test_parse_dataset_path_too_many_slashes() {
        let result = parse_dataset_path("owner/dataset/extra");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_empty_owner() {
        let result = parse_dataset_path("/dataset");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_empty_dataset() {
        let result = parse_dataset_path("owner/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_traversal_owner() {
        let result = parse_dataset_path("../dataset");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_traversal_dataset() {
        let result = parse_dataset_path("owner/..");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_special_chars() {
        let result = parse_dataset_path("user@domain.com/dataset-v1.0");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "user@domain.com");
        assert_eq!(dataset, "dataset-v1.0");
    }

    #[test]
    fn test_parse_dataset_path_trims_whitespace() {
        let result = parse_dataset_path("  owner / dataset  ");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset");
    }

    #[test]
    fn test_parse_dataset_path_rejects_dot_segments() {
        assert!(parse_dataset_path("./owner").is_err());
        assert!(parse_dataset_path("owner/.").is_err());
        assert!(parse_dataset_path("./owner/. ").is_err());
    }

    #[test]
    fn test_parse_dataset_path_max_length() {
        // Path that exceeds 4096 characters should be rejected
        let long_path = format!("{}/{}", "a".repeat(2500), "b".repeat(2500));
        assert!(long_path.len() > 4096);
        let result = parse_dataset_path(&long_path);
        assert!(result.is_err());
        match result {
            Err(crate::error::GaggleError::InvalidDatasetPath(msg)) => {
                assert!(msg.contains("maximum length"));
            }
            _ => panic!("Expected InvalidDatasetPath error for oversized path"),
        }
    }

    // Version parsing tests
    #[test]
    fn test_parse_with_version_v_prefix() {
        let (owner, dataset, version) =
            parse_dataset_path_with_version("owner/dataset@v2").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset");
        assert_eq!(version, Some("2".to_string()));
    }

    #[test]
    fn test_parse_with_version_no_v_prefix() {
        let (owner, dataset, version) = parse_dataset_path_with_version("owner/dataset@5").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset");
        assert_eq!(version, Some("5".to_string()));
    }

    #[test]
    fn test_parse_with_version_latest() {
        let (owner, dataset, version) =
            parse_dataset_path_with_version("owner/dataset@latest").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_no_version() {
        let (owner, dataset, version) = parse_dataset_path_with_version("owner/dataset").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_version_invalid_not_number() {
        let result = parse_dataset_path_with_version("owner/dataset@abc");
        assert!(result.is_err());
        if let Err(crate::error::GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("Invalid version number"));
        }
    }

    #[test]
    fn test_parse_version_multiple_at_signs() {
        let result = parse_dataset_path_with_version("owner/dataset@v2@v3");
        assert!(result.is_err());
        if let Err(crate::error::GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("one @"));
        }
    }

    #[test]
    fn test_parse_version_with_hyphenated_name() {
        let (owner, dataset, version) =
            parse_dataset_path_with_version("my-owner/my-dataset@v10").unwrap();
        assert_eq!(owner, "my-owner");
        assert_eq!(dataset, "my-dataset");
        assert_eq!(version, Some("10".to_string()));
    }

    #[test]
    fn test_parse_version_zero_rejected() {
        let result = parse_dataset_path_with_version("owner/dataset@0");
        assert!(result.is_err());
        if let Err(crate::error::GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("> 0"));
        }
    }

    #[test]
    fn test_parse_version_large_number() {
        let (_owner, _dataset, version) =
            parse_dataset_path_with_version("owner/dataset@v999").unwrap();
        assert_eq!(version, Some("999".to_string()));
    }

    #[test]
    fn test_parse_version_empty_after_at() {
        let (_owner, _dataset, version) =
            parse_dataset_path_with_version("owner/dataset@").unwrap();
        assert_eq!(version, None); // Empty after @ treated as latest
    }

    #[test]
    fn test_parse_version_with_whitespace() {
        let (_owner, _dataset, version) =
            parse_dataset_path_with_version("owner/dataset@v2 ").unwrap();
        assert_eq!(version, Some("2".to_string())); // Should trim whitespace
    }

    #[test]
    fn test_parse_dataset_path_exactly_max_length() {
        // Path exactly at limit should be rejected (4097 to test boundary)
        let owner = "a".repeat(2047);
        let dataset = "b".repeat(2048);
        let path = format!("{}/{}", owner, dataset); // 2047 + 1 + 2048 = 4096
        let result = parse_dataset_path(&path);
        // At the limit, should still succeed
        assert!(result.is_ok());
    }
}
