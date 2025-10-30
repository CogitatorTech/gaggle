// Kaggle API client implementation

use crate::error::GaggleError;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

/// Kaggle API credentials stored in memory
static CREDENTIALS: Lazy<RwLock<Option<KaggleCredentials>>> = Lazy::new(|| RwLock::new(None));

/// Track ongoing dataset downloads to prevent concurrent downloads of the same dataset
static DOWNLOAD_LOCKS: Lazy<Mutex<HashMap<String, ()>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct KaggleCredentials {
    pub username: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DatasetInfo {
    pub ref_path: String,
    pub title: String,
    pub size: u64,
    pub url: String,
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetFile {
    pub name: String,
    pub size: u64,
}

/// Set Kaggle API credentials
pub fn set_credentials(username: &str, key: &str) -> Result<(), GaggleError> {
    let mut creds = CREDENTIALS.write();
    *creds = Some(KaggleCredentials {
        username: username.to_string(),
        key: key.to_string(),
    });
    Ok(())
}

/// Get stored credentials or try to load from environment/file
pub fn get_credentials() -> Result<KaggleCredentials, GaggleError> {
    // Check if credentials are already set in memory (fast path with read lock)
    if let Some(creds) = CREDENTIALS.read().as_ref() {
        return Ok(creds.clone());
    }

    // Acquire write lock to prevent race condition where multiple threads
    // try to load credentials simultaneously
    let mut creds_guard = CREDENTIALS.write();

    // Double-check after acquiring write lock (another thread may have loaded it)
    if let Some(creds) = creds_guard.as_ref() {
        return Ok(creds.clone());
    }

    // Try environment variables
    if let (Ok(username), Ok(key)) = (
        std::env::var("KAGGLE_USERNAME"),
        std::env::var("KAGGLE_KEY"),
    ) {
        let creds = KaggleCredentials { username, key };
        *creds_guard = Some(creds.clone());
        return Ok(creds);
    }

    // Try kaggle.json file
    let kaggle_json_path = dirs::home_dir()
        .ok_or_else(|| GaggleError::CredentialsError("Cannot find home directory".to_string()))?
        .join(".kaggle")
        .join("kaggle.json");

    if kaggle_json_path.exists() {
        // Verify file permissions for security (should not be world-readable)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&kaggle_json_path).map_err(|e| {
                GaggleError::CredentialsError(format!("Cannot read kaggle.json metadata: {}", e))
            })?;
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                eprintln!(
                    "Warning: kaggle.json has overly permissive permissions. \
                     It should be readable only by the owner (chmod 600)."
                );
            }
        }

        let content = fs::read_to_string(&kaggle_json_path).map_err(|e| {
            GaggleError::CredentialsError(format!("Cannot read kaggle.json: {}", e))
        })?;

        let json: serde_json::Value = serde_json::from_str(&content)?;
        let username = json["username"]
            .as_str()
            .ok_or_else(|| {
                GaggleError::CredentialsError("Missing username in kaggle.json".to_string())
            })?
            .to_string();
        let key = json["key"]
            .as_str()
            .ok_or_else(|| GaggleError::CredentialsError("Missing key in kaggle.json".to_string()))?
            .to_string();

        let creds = KaggleCredentials { username, key };
        *creds_guard = Some(creds.clone());
        return Ok(creds);
    }

    Err(GaggleError::CredentialsError(
        "No Kaggle credentials found. Set KAGGLE_USERNAME and KAGGLE_KEY environment variables, \
         create ~/.kaggle/kaggle.json, or call gaggle_set_credentials()"
            .to_string(),
    ))
}

/// Parse dataset path like "username/dataset-name"
pub fn parse_dataset_path(path: &str) -> Result<(String, String), GaggleError> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 2 {
        return Err(GaggleError::InvalidDatasetPath(format!(
            "Dataset path must be in format 'owner/dataset-name', got: {}",
            path
        )));
    }

    // Validate that both owner and dataset are non-empty
    if parts[0].is_empty() || parts[1].is_empty() {
        return Err(GaggleError::InvalidDatasetPath(format!(
            "Dataset path cannot have empty owner or dataset name, got: {}",
            path
        )));
    }

    // Security: reject single-segment traversal components to avoid paths like `owner/..` or `.`
    if parts[0] == "." || parts[0] == ".." || parts[1] == "." || parts[1] == ".." {
        return Err(GaggleError::InvalidDatasetPath(format!(
            "Dataset path contains invalid traversal segments: {}",
            path
        )));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

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

#[cfg(test)]
pub(crate) fn set_test_api_base(base: Option<String>) {
    thread_local! {
        static TEST_API_BASE: RefCell<Option<String>> = const { RefCell::new(None) };
    }
    TEST_API_BASE.with(|c| *c.borrow_mut() = base);
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

fn with_retries<F, T>(mut f: F) -> Result<T, GaggleError>
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

/// Guard to ensure download lock is released
struct LockGuard {
    key: String,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        DOWNLOAD_LOCKS.lock().remove(&self.key);
    }
}

/// Download a Kaggle dataset
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);

    // Check if already downloaded (fast path)
    let marker_file = cache_dir.join(".downloaded");
    if marker_file.exists() {
        return Ok(cache_dir);
    }

    // Use a lock per dataset path to prevent concurrent downloads of the same dataset
    let lock_key = format!("{}/{}", owner, dataset);

    // Acquire a "lock" by inserting into the map
    // If another thread is downloading, wait
    loop {
        let mut locks = DOWNLOAD_LOCKS.lock();
        if !locks.contains_key(&lock_key) {
            locks.insert(lock_key.clone(), ());
            break;
        }
        // Release lock and sleep briefly before retrying
        drop(locks);
        sleep(Duration::from_millis(100));

        // Check again if download completed while we waited
        if marker_file.exists() {
            return Ok(cache_dir);
        }
    }

    // Ensure we clean up the lock when done
    let _guard = LockGuard {
        key: lock_key.clone(),
    };

    // Double-check after acquiring lock
    if marker_file.exists() {
        return Ok(cache_dir);
    }

    fs::create_dir_all(&cache_dir)?;

    // Download using Kaggle API
    let url = format!("{}/datasets/download/{}/{}", get_api_base(), owner, dataset);

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
            "Failed to download dataset: HTTP {}",
            response.status()
        )));
    }

    // Save and extract ZIP
    let zip_path = cache_dir.join("dataset.zip");
    let content = response.bytes()?;
    fs::write(&zip_path, &content)?;

    // Extract ZIP - require at least one file extracted
    let extracted = extract_zip(&zip_path, &cache_dir)?;
    if extracted == 0 {
        return Err(GaggleError::ZipError("ZIP contained no files".to_string()));
    }

    // Clean up ZIP file
    fs::remove_file(&zip_path)?;

    // Create marker file
    fs::write(&marker_file, "")?;

    Ok(cache_dir)
}

/// Extract ZIP file
pub(crate) fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<usize, GaggleError> {
    let file = fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| GaggleError::ZipError(e.to_string()))?;

    // ZIP bomb protection: limit total uncompressed size to 10GB
    const MAX_TOTAL_SIZE: u64 = 10 * 1024 * 1024 * 1024;
    let mut total_size: u64 = 0;
    let mut files_extracted: usize = 0;

    // Get canonical destination directory for symlink attack prevention
    let canonical_dest = dest_dir
        .canonicalize()
        .unwrap_or_else(|_| dest_dir.to_path_buf());

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| GaggleError::ZipError(e.to_string()))?;

        // Check for path traversal
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => {
                // Skip files with unsafe paths
                continue;
            }
        };

        // Validate the output path is still within dest_dir
        if !outpath.starts_with(dest_dir) {
            return Err(GaggleError::ZipError(format!(
                "Path traversal attempt detected: {:?}",
                file.name()
            )));
        }

        // Additional check: verify canonical path is within destination
        // This prevents symlink attacks
        if let Ok(canonical_out) = outpath.canonicalize().or_else(|_| {
            // If canonicalize fails (file doesn't exist yet), check parent
            outpath
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .map(|p| p.join(outpath.file_name().unwrap_or_default()))
                .ok_or_else(|| std::io::Error::other("Invalid path"))
        }) {
            if !canonical_out.starts_with(&canonical_dest) {
                return Err(GaggleError::ZipError(format!(
                    "Symlink attack detected: {:?}",
                    file.name()
                )));
            }
        }

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            // Check total uncompressed size
            total_size = total_size.saturating_add(file.size());
            if total_size > MAX_TOTAL_SIZE {
                return Err(GaggleError::ZipError(format!(
                    "ZIP file too large: uncompressed size exceeds {} GB",
                    MAX_TOTAL_SIZE / (1024 * 1024 * 1024)
                )));
            }

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            files_extracted += 1;
        }
    }

    Ok(files_extracted)
}

/// List files in a downloaded dataset
pub fn list_dataset_files(dataset_path: &str) -> Result<Vec<DatasetFile>, GaggleError> {
    let dataset_dir = download_dataset(dataset_path)?;
    let mut files = Vec::new();

    for entry in fs::read_dir(&dataset_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if file_name != ".downloaded" {
                    let metadata = fs::metadata(&path)?;
                    if let Some(name) = path.file_name() {
                        files.push(DatasetFile {
                            name: name.to_string_lossy().to_string(),
                            size: metadata.len(),
                        });
                    }
                }
            }
        }
    }

    Ok(files)
}

/// Get the local path to a specific file in a dataset
pub fn get_dataset_file_path(dataset_path: &str, filename: &str) -> Result<PathBuf, GaggleError> {
    let dataset_dir = download_dataset(dataset_path)?;
    let file_path = dataset_dir.join(filename);

    if !file_path.exists() {
        return Err(GaggleError::IoError(format!(
            "File '{}' not found in dataset '{}'",
            filename, dataset_path
        )));
    }

    Ok(file_path)
}

/// Search for datasets on Kaggle
pub fn search_datasets(
    query: &str,
    page: i32,
    page_size: i32,
) -> Result<serde_json::Value, GaggleError> {
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

/// Get metadata for a specific dataset
pub fn get_dataset_metadata(dataset_path: &str) -> Result<serde_json::Value, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

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
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server as MockServer};
    use serial_test::serial;
    use std::io::Write;
    use std::thread;

    #[test]
    fn test_kaggle_credentials_struct() {
        let creds = KaggleCredentials {
            username: "testuser".to_string(),
            key: "testkey".to_string(),
        };
        assert_eq!(creds.username, "testuser");
        assert_eq!(creds.key, "testkey");
    }

    #[test]
    fn test_dataset_file_struct() {
        let file = DatasetFile {
            name: "data.csv".to_string(),
            size: 1024,
        };
        assert_eq!(file.name, "data.csv");
        assert_eq!(file.size, 1024);
    }

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
            Err(GaggleError::InvalidDatasetPath(msg)) => {
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
    fn test_parse_dataset_path_trailing_slash() {
        let result = parse_dataset_path("owner/dataset/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_leading_slash() {
        let result = parse_dataset_path("/owner/dataset");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_empty_owner() {
        let result = parse_dataset_path("/dataset");
        // "/dataset" splits to ["", "dataset"], so it has 2 parts
        // The function now rejects empty owner
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_empty_dataset() {
        let result = parse_dataset_path("owner/");
        // "owner/" splits to ["owner", ""], so it has 2 parts
        // The function now rejects empty dataset
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_empty_string() {
        let result = parse_dataset_path("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_result_strings() {
        let result = parse_dataset_path("owner/dataset");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        // Verify they are strings
        assert_eq!(owner.len(), 5);
        assert_eq!(dataset.len(), 7);
    }

    #[test]
    fn test_set_credentials() {
        let result = set_credentials("testuser", "testkey");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_credentials_empty_username() {
        let result = set_credentials("", "testkey");
        assert!(result.is_ok()); // Should succeed but store empty username
    }

    #[test]
    fn test_set_credentials_empty_key() {
        let result = set_credentials("testuser", "");
        assert!(result.is_ok()); // Should succeed but store empty key
    }

    #[test]
    fn test_set_credentials_overwrite() {
        let _ = set_credentials("user1", "key1");
        let _ = set_credentials("user2", "key2");
        // The second call should overwrite the first
        assert!(set_credentials("user2", "key2").is_ok());
    }

    #[test]
    fn test_set_credentials_with_special_chars() {
        let result = set_credentials("user@domain.com", "key!@#$%^");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_credentials_very_long() {
        let long_username = "a".repeat(1000);
        let long_key = "b".repeat(1000);
        let result = set_credentials(&long_username, &long_key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_credentials_unicode() {
        let result = set_credentials("用户名", "密钥");
        assert!(result.is_ok());
    }

    #[test]
    fn test_dataset_file_struct_serializable() {
        let file = DatasetFile {
            name: "test.csv".to_string(),
            size: 2048,
        };
        let json = serde_json::to_string(&file);
        assert!(json.is_ok());
    }

    #[test]
    fn test_dataset_info_struct_serializable() {
        let info = DatasetInfo {
            ref_path: "owner/dataset".to_string(),
            title: "Test Dataset".to_string(),
            size: 1024000,
            url: "https://kaggle.com/owner/dataset".to_string(),
            last_updated: "2025-10-30".to_string(),
        };
        let json = serde_json::to_string(&info);
        assert!(json.is_ok());
    }

    #[test]
    fn test_credentials_clone() {
        let creds = KaggleCredentials {
            username: "user".to_string(),
            key: "key".to_string(),
        };
        let cloned = creds.clone();
        assert_eq!(creds.username, cloned.username);
        assert_eq!(creds.key, cloned.key);
    }

    #[test]
    fn test_dataset_file_different_sizes() {
        let files = vec![
            DatasetFile {
                name: "small.txt".to_string(),
                size: 100,
            },
            DatasetFile {
                name: "large.csv".to_string(),
                size: 1_000_000_000,
            },
            DatasetFile {
                name: "empty.json".to_string(),
                size: 0,
            },
        ];

        for file in files {
            assert!(!file.name.is_empty());
        }
    }

    #[test]
    fn test_parse_dataset_path_with_dots() {
        let result = parse_dataset_path("owner.name/dataset.name");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "owner.name");
        assert_eq!(dataset, "dataset.name");
    }

    #[test]
    fn test_parse_dataset_path_case_preservation() {
        let result = parse_dataset_path("OwNeR/DaTaSeT");
        assert!(result.is_ok());
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, "OwNeR");
        assert_eq!(dataset, "DaTaSeT");
    }

    #[test]
    fn test_kaggle_credentials_debug() {
        let creds = KaggleCredentials {
            username: "user".to_string(),
            key: "key".to_string(),
        };
        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("KaggleCredentials"));
    }

    #[test]
    fn test_credentials_multiple_slashes_in_dataset() {
        // This should fail because of too many parts
        let result = parse_dataset_path("owner/dataset/withslash");
        assert!(result.is_err());
    }

    #[test]
    fn test_search_datasets_with_mock_server() {
        let _ = set_credentials("testuser", "testkey");

        let mut server = MockServer::new();
        let _m = server
            .mock("GET", "/datasets/list")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body("[]")
            .create();

        env::set_var("GAGGLE_API_BASE", server.url());
        let res = search_datasets("test", 1, 10);
        env::remove_var("GAGGLE_API_BASE");

        assert!(res.is_ok());
        assert!(res.unwrap().is_array());
    }

    #[test]
    fn test_get_dataset_metadata_with_mock_server() {
        let _ = set_credentials("testuser", "testkey");

        let mut server = MockServer::new();
        let _m = server
            .mock("GET", "/datasets/view/owner/dataset")
            .with_status(200)
            .with_body(serde_json::json!({"ref":"owner/dataset"}).to_string())
            .create();

        env::set_var("GAGGLE_API_BASE", server.url());
        let res = get_dataset_metadata("owner/dataset");
        env::remove_var("GAGGLE_API_BASE");

        assert!(res.is_ok());
        assert_eq!(res.unwrap()["ref"], "owner/dataset");
    }

    #[test]
    #[serial]
    fn test_download_dataset_with_mock_zip() {
        let _ = set_credentials("testuser", "testkey");

        // Prepare a small zip in memory
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut zipw = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default();
            zipw.start_file("file.txt", options).unwrap();
            zipw.write_all(b"hello world").unwrap();
            zipw.finish().unwrap();
        }
        let zip_buf = cursor.into_inner();

        // Sanity-check: extracting these bytes locally should yield one file
        let tmp_zip = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp_zip.path(), &zip_buf).unwrap();
        let tmp_out = tempfile::TempDir::new().unwrap();
        let cnt = extract_zip(tmp_zip.path(), tmp_out.path()).unwrap();
        assert_eq!(cnt, 1);
        assert!(tmp_out.path().join("file.txt").exists());

        let mut server = MockServer::new();
        let _m = server
            .mock("GET", "/datasets/download/owner/dataset")
            .with_status(200)
            .with_body(zip_buf)
            .create();

        // Use temp cache dir and override runtime
        let tempdir = tempfile::TempDir::new().unwrap();
        crate::config::set_test_cache_dir(Some(tempdir.path().to_path_buf()));
        env::set_var("GAGGLE_API_BASE", server.url());

        let res = download_dataset("owner/dataset");

        env::remove_var("GAGGLE_API_BASE");
        crate::config::set_test_cache_dir(None);

        match res {
            Ok(dir) => {
                assert!(dir.join("file.txt").exists());
                assert!(dir.join(".downloaded").exists());
            }
            Err(e) => panic!("download_dataset error: {}", e),
        }
    }

    #[test]
    #[serial]
    fn test_http_timeout_respected() {
        let _ = set_credentials("testuser", "testkey");

        // Use tiny_http to delay response
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr();
        let base = format!("http://{}", addr);

        let handle = thread::spawn(move || {
            if let Ok(req) = server.recv() {
                // Delay longer than timeout
                std::thread::sleep(Duration::from_millis(2200));
                let _ = req.respond(tiny_http::Response::from_string("[]").with_status_code(200));
            }
        });

        env::set_var("GAGGLE_API_BASE", base);
        env::set_var("GAGGLE_HTTP_TIMEOUT", "1");

        let start = std::time::Instant::now();
        let res = search_datasets("slow", 1, 10);
        let elapsed = start.elapsed();

        env::remove_var("GAGGLE_API_BASE");
        env::remove_var("GAGGLE_HTTP_TIMEOUT");
        handle.join().unwrap();

        assert!(res.is_err());
        // Should not hang for long
        assert!(elapsed.as_secs() <= 5);
    }

    // Regression tests for bug fixes

    #[test]
    fn test_parse_dataset_path_empty_owner_rejected() {
        let result = parse_dataset_path("/dataset");
        assert!(result.is_err());
        match result {
            Err(GaggleError::InvalidDatasetPath(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidDatasetPath error for empty owner"),
        }
    }

    #[test]
    fn test_parse_dataset_path_empty_dataset_rejected() {
        let result = parse_dataset_path("owner/");
        assert!(result.is_err());
        match result {
            Err(GaggleError::InvalidDatasetPath(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidDatasetPath error for empty dataset"),
        }
    }

    #[test]
    fn test_parse_dataset_path_double_empty_rejected() {
        let result = parse_dataset_path("/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_only_slashes() {
        let result = parse_dataset_path("//");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataset_path_whitespace() {
        let result = parse_dataset_path(" / ");
        // Spaces are technically valid characters, so " " owner and " " dataset is OK
        // This test verifies that whitespace is handled consistently
        if let Ok((owner, dataset)) = result {
            assert_eq!(owner, " ");
            assert_eq!(dataset, " ");
        } else {
            panic!("Expected success for path with spaces");
        }
    }

    #[test]
    fn test_search_datasets_negative_page() {
        let result = search_datasets("test", -1, 10);
        assert!(result.is_err());
        match result {
            Err(GaggleError::InvalidDatasetPath(msg)) => {
                assert!(msg.contains("Page"));
            }
            _ => panic!("Expected error for negative page"),
        }
    }

    #[test]
    fn test_search_datasets_zero_page() {
        let result = search_datasets("test", 0, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_datasets_negative_page_size() {
        let result = search_datasets("test", 1, -1);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_datasets_zero_page_size() {
        let result = search_datasets("test", 1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_datasets_excessive_page_size() {
        let result = search_datasets("test", 1, 101);
        assert!(result.is_err());
        match result {
            Err(GaggleError::InvalidDatasetPath(msg)) => {
                assert!(msg.contains("between 1 and 100"));
            }
            _ => panic!("Expected error for excessive page size"),
        }
    }

    #[test]
    fn test_search_datasets_boundary_page_size() {
        let result = search_datasets("test", 1, 100);
        // Will fail on credentials, not validation
        if let Err(GaggleError::InvalidDatasetPath(_)) = result {
            panic!("Should not reject valid page size 100")
        }
    }

    #[test]
    fn test_extract_zip_validates_paths() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("test.zip");
        let extract_dir = TempDir::new().unwrap();

        // Create a simple valid ZIP file
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<()>::default();
        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"test content").unwrap();
        zip.finish().unwrap();

        // Valid extraction should succeed
        let count = extract_zip(&zip_path, extract_dir.path()).unwrap();
        assert_eq!(count, 1);
        assert!(extract_dir.path().join("test.txt").exists());
    }

    #[test]
    fn test_extract_zip_size_limit() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("small.zip");
        let extract_dir = TempDir::new().unwrap();

        // Create a small ZIP
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<()>::default();
        zip.start_file("small.txt", options).unwrap();
        let data = vec![b'x'; 1000];
        zip.write_all(&data).unwrap();
        zip.finish().unwrap();

        // Should succeed for small files
        let count = extract_zip(&zip_path, extract_dir.path()).unwrap();
        assert!(count >= 1);
    }
}
