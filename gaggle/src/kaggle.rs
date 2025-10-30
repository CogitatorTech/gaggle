// Kaggle API client implementation

use crate::error::GaggleError;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::env;

/// Kaggle API credentials stored in memory
static CREDENTIALS: Lazy<RwLock<Option<KaggleCredentials>>> = Lazy::new(|| RwLock::new(None));

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
    // Check if credentials are already set in memory
    if let Some(creds) = CREDENTIALS.read().as_ref() {
        return Ok(creds.clone());
    }

    // Try environment variables
    if let (Ok(username), Ok(key)) = (
        std::env::var("KAGGLE_USERNAME"),
        std::env::var("KAGGLE_KEY"),
    ) {
        let creds = KaggleCredentials { username, key };
        *CREDENTIALS.write() = Some(creds.clone());
        return Ok(creds);
    }

    // Try kaggle.json file
    let kaggle_json_path = dirs::home_dir()
        .ok_or_else(|| GaggleError::CredentialsError("Cannot find home directory".to_string()))?
        .join(".kaggle")
        .join("kaggle.json");

    if kaggle_json_path.exists() {
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
        *CREDENTIALS.write() = Some(creds.clone());
        return Ok(creds);
    }

    Err(GaggleError::CredentialsError(
        "No Kaggle credentials found. Set KAGGLE_USERNAME and KAGGLE_KEY environment variables, \
         create ~/.kaggle/kaggle.json, or call kaggle_set_credentials()"
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
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Helper: get API base URL (overridable at runtime via env for testing)
fn get_api_base() -> String {
    env::var("GAGGLE_API_BASE").unwrap_or_else(|_| "https://www.kaggle.com/api/v1".to_string())
}

/// Helper: build a reqwest client with timeout and UA
fn build_client() -> Result<Client, GaggleError> {
    let timeout = Duration::from_secs(crate::config::http_timeout_runtime_secs());
    let ua = format!(
        "Gaggle/{} (+https://github.com/CogitatorTech/gaggle)",
        env!("CARGO_PKG_VERSION")
    );
    Ok(
        reqwest::blocking::ClientBuilder::new()
            .timeout(timeout)
            .user_agent(ua)
            .build()?
    )
}

/// Download a Kaggle dataset
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);
    fs::create_dir_all(&cache_dir)?;

    // Check if already downloaded
    let marker_file = cache_dir.join(".downloaded");
    if marker_file.exists() {
        return Ok(cache_dir);
    }

    // Download using Kaggle API
    let url = format!(
        "{}/datasets/download/{}/{}",
        get_api_base(), owner, dataset
    );

    let client = build_client()?;
    let response = client
        .get(&url)
        .basic_auth(&creds.username, Some(&creds.key))
        .send()?;

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

    // Extract ZIP
    extract_zip(&zip_path, &cache_dir)?;

    // Clean up ZIP file
    fs::remove_file(&zip_path)?;

    // Create marker file
    fs::write(&marker_file, "")?;

    Ok(cache_dir)
}

/// Extract ZIP file
fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), GaggleError> {
    let file = fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| GaggleError::ZipError(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| GaggleError::ZipError(e.to_string()))?;

        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
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
    let creds = get_credentials()?;

    let url = format!(
        "{}/datasets/list?search={}&page={}&pageSize={}",
        get_api_base(),
        urlencoding::encode(query),
        page,
        page_size
    );

    let client = build_client()?;
    let response = client
        .get(&url)
        .basic_auth(&creds.username, Some(&creds.key))
        .send()?;

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

    let url = format!(
        "{}/datasets/view/{}/{}",
        get_api_base(), owner, dataset
    );

    let client = build_client()?;
    let response = client
        .get(&url)
        .basic_auth(&creds.username, Some(&creds.key))
        .send()?;

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
    use std::thread;
    use std::io::Write;
    use mockito::{Server as MockServer, Matcher};

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
        // The function accepts it but owner is empty string
        if result.is_ok() {
            let (owner, _) = result.unwrap();
            assert_eq!(owner, "");
        }
    }

    #[test]
    fn test_parse_dataset_path_empty_dataset() {
        let result = parse_dataset_path("owner/");
        // "owner/" splits to ["owner", ""], so it has 2 parts
        // The function accepts it but dataset is empty string
        if result.is_ok() {
            let (_, dataset) = result.unwrap();
            assert_eq!(dataset, "");
        }
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
    fn test_download_dataset_with_mock_zip() {
        let _ = set_credentials("testuser", "testkey");

        // Prepare a small zip in memory
        let mut zip_buf: Vec<u8> = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut zip_buf);
            let mut zipw = zip::ZipWriter::new(cursor);
            let options = zip::write::SimpleFileOptions::default();
            zipw.start_file("file.txt", options).unwrap();
            zipw.write_all(b"hello world").unwrap();
            zipw.finish().unwrap();
        }

        let mut server = MockServer::new();
        let _m = server
            .mock("GET", "/datasets/download/owner/dataset")
            .with_status(200)
            .with_body(zip_buf.clone())
            .create();

        // Use temp cache dir
        let tempdir = tempfile::TempDir::new().unwrap();
        env::set_var("GAGGLE_CACHE_DIR", tempdir.path());
        env::set_var("GAGGLE_API_BASE", server.url());

        let res = download_dataset("owner/dataset");

        env::remove_var("GAGGLE_API_BASE");
        env::remove_var("GAGGLE_CACHE_DIR");

        assert!(res.is_ok());
        let dir = res.unwrap();
        assert!(dir.join("file.txt").exists());
        assert!(dir.join(".downloaded").exists());
    }

    #[test]
    fn test_http_timeout_respected() {
        let _ = set_credentials("testuser", "testkey");

        // Use tiny_http to delay response
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr();
        let base = format!("http://{}", addr);

        let handle = thread::spawn(move || {
            if let Ok(mut req) = server.recv() {
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
}
