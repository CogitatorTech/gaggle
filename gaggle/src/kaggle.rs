// Kaggle API client implementation

use crate::config::CONFIG;
use crate::error::GaggleError;
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

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
    if let (Ok(username), Ok(key)) = (std::env::var("KAGGLE_USERNAME"), std::env::var("KAGGLE_KEY")) {
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
        let content = fs::read_to_string(&kaggle_json_path)
            .map_err(|e| GaggleError::CredentialsError(format!("Cannot read kaggle.json: {}", e)))?;

        let json: serde_json::Value = serde_json::from_str(&content)?;
        let username = json["username"]
            .as_str()
            .ok_or_else(|| GaggleError::CredentialsError("Missing username in kaggle.json".to_string()))?
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
         create ~/.kaggle/kaggle.json, or call kaggle_set_credentials()".to_string()
    ))
}

/// Parse dataset path like "username/dataset-name"
pub fn parse_dataset_path(path: &str) -> Result<(String, String), GaggleError> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 2 {
        return Err(GaggleError::InvalidDatasetPath(
            format!("Dataset path must be in format 'owner/dataset-name', got: {}", path)
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Download a Kaggle dataset
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

    let cache_dir = CONFIG.cache_dir.join("datasets").join(&owner).join(&dataset);
    fs::create_dir_all(&cache_dir)?;

    // Check if already downloaded
    let marker_file = cache_dir.join(".downloaded");
    if marker_file.exists() {
        return Ok(cache_dir);
    }

    // Download using Kaggle API
    let url = format!("https://www.kaggle.com/api/v1/datasets/download/{}/{}", owner, dataset);

    let client = Client::new();
    let response = client
        .get(&url)
        .basic_auth(&creds.username, Some(&creds.key))
        .send()?;

    if !response.status().is_success() {
        return Err(GaggleError::HttpRequestError(
            format!("Failed to download dataset: HTTP {}", response.status())
        ));
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
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| GaggleError::ZipError(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
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

        if path.is_file() && path.file_name().unwrap() != ".downloaded" {
            let metadata = fs::metadata(&path)?;
            files.push(DatasetFile {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                size: metadata.len(),
            });
        }
    }

    Ok(files)
}

/// Get the local path to a specific file in a dataset
pub fn get_dataset_file_path(dataset_path: &str, filename: &str) -> Result<PathBuf, GaggleError> {
    let dataset_dir = download_dataset(dataset_path)?;
    let file_path = dataset_dir.join(filename);

    if !file_path.exists() {
        return Err(GaggleError::IoError(
            format!("File '{}' not found in dataset '{}'", filename, dataset_path)
        ));
    }

    Ok(file_path)
}

/// Search for datasets on Kaggle
pub fn search_datasets(query: &str, page: i32, page_size: i32) -> Result<serde_json::Value, GaggleError> {
    let creds = get_credentials()?;

    let url = format!(
        "https://www.kaggle.com/api/v1/datasets/list?search={}&page={}&pageSize={}",
        urlencoding::encode(query),
        page,
        page_size
    );

    let client = Client::new();
    let response = client
        .get(&url)
        .basic_auth(&creds.username, Some(&creds.key))
        .send()?;

    if !response.status().is_success() {
        return Err(GaggleError::HttpRequestError(
            format!("Failed to search datasets: HTTP {}", response.status())
        ));
    }

    let json: serde_json::Value = response.json()?;
    Ok(json)
}

/// Get metadata for a specific dataset
pub fn get_dataset_metadata(dataset_path: &str) -> Result<serde_json::Value, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

    let url = format!("https://www.kaggle.com/api/v1/datasets/view/{}/{}", owner, dataset);

    let client = Client::new();
    let response = client
        .get(&url)
        .basic_auth(&creds.username, Some(&creds.key))
        .send()?;

    if !response.status().is_success() {
        return Err(GaggleError::HttpRequestError(
            format!("Failed to get dataset metadata: HTTP {}", response.status())
        ));
    }

    let json: serde_json::Value = response.json()?;
    Ok(json)
}

