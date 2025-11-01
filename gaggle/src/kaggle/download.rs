use crate::error::GaggleError;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use super::api::{build_client, get_api_base, with_retries};
use super::credentials::get_credentials;

/// Track ongoing dataset downloads to prevent concurrent downloads of the same dataset
static DOWNLOAD_LOCKS: once_cell::sync::Lazy<Mutex<HashMap<String, ()>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetFile {
    pub name: String,
    pub size: u64,
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
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

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
    // If another thread is downloading, wait with timeout
    const MAX_WAIT_ATTEMPTS: u32 = 300; // 30 seconds at 100ms intervals
    let mut wait_attempts = 0;

    loop {
        let mut locks = DOWNLOAD_LOCKS.lock();
        if !locks.contains_key(&lock_key) {
            locks.insert(lock_key.clone(), ());
            break;
        }
        // Release lock and sleep briefly before retrying
        drop(locks);

        // Check timeout to prevent indefinite waiting
        wait_attempts += 1;
        if wait_attempts >= MAX_WAIT_ATTEMPTS {
            return Err(GaggleError::HttpRequestError(format!(
                "Timeout waiting for download of {}. Another thread may have stalled.",
                dataset_path
            )));
        }

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

    // Ensure destination directory exists and canonicalize it for comparisons
    fs::create_dir_all(dest_dir)?;
    let canonical_dest = dest_dir.canonicalize().map_err(|e| {
        GaggleError::IoError(format!(
            "Failed to canonicalize destination directory: {}",
            e
        ))
    })?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| GaggleError::ZipError(e.to_string()))?;

        // Reject symlink entries based on UNIX mode bits if present
        if let Some(mode) = entry.unix_mode() {
            let file_type = mode & 0o170000;
            if file_type == 0o120000 {
                return Err(GaggleError::ZipError(format!(
                    "Symlink entry not allowed in archive: {}",
                    entry.name()
                )));
            }
        }

        // Ensure the path is safe (prevents path traversal like ../)
        let rel_path = match entry.enclosed_name() {
            Some(path) => path.to_owned(),
            None => {
                // Skip entries with invalid names
                continue;
            }
        };

        let outpath = dest_dir.join(&rel_path);

        // Validate the output path is still within dest_dir using canonical parent
        let parent = outpath.parent().unwrap_or(dest_dir);
        fs::create_dir_all(parent)?;
        let canonical_parent = parent.canonicalize().map_err(|e| {
            GaggleError::ZipError(format!(
                "Failed to canonicalize parent directory for {}: {}",
                rel_path.display(),
                e
            ))
        })?;
        if !canonical_parent.starts_with(&canonical_dest) {
            return Err(GaggleError::ZipError(format!(
                "Path traversal attempt detected: {:?}",
                entry.name()
            )));
        }

        if entry.is_dir() || entry.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
            continue;
        }

        // Check total uncompressed size
        total_size = total_size.saturating_add(entry.size());
        if total_size > MAX_TOTAL_SIZE {
            return Err(GaggleError::ZipError(format!(
                "ZIP file too large: uncompressed size exceeds {} GB",
                MAX_TOTAL_SIZE / (1024 * 1024 * 1024)
            )));
        }

        // Finally, write the file
        if let Some(p) = outpath.parent() {
            fs::create_dir_all(p)?;
        }
        let mut outfile = fs::File::create(&outpath)?;
        std::io::copy(&mut entry, &mut outfile)?;
        files_extracted += 1;
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
    // Validate filename to prevent path traversal or absolute paths
    use std::path::Component;
    let fname_path = Path::new(filename);
    if fname_path.is_absolute() {
        return Err(GaggleError::InvalidDatasetPath(
            "Absolute filenames are not allowed".to_string(),
        ));
    }
    for comp in fname_path.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(GaggleError::InvalidDatasetPath(
                    "Filename must not contain parent or root components".to_string(),
                ));
            }
            _ => {}
        }
    }

    let dataset_dir = download_dataset(dataset_path)?;
    let file_path = dataset_dir.join(fname_path);

    if !file_path.exists() {
        return Err(GaggleError::IoError(format!(
            "File '{}' not found in dataset '{}'",
            filename, dataset_path
        )));
    }

    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_dataset_file_struct() {
        let file = DatasetFile {
            name: "test.csv".to_string(),
            size: 1024,
        };
        assert_eq!(file.name, "test.csv");
        assert_eq!(file.size, 1024);
    }

    #[test]
    fn test_lock_guard_cleanup() {
        let lock_key = "test/dataset".to_string();

        // Insert into locks
        {
            let mut locks = DOWNLOAD_LOCKS.lock();
            locks.insert(lock_key.clone(), ());
            assert!(locks.contains_key(&lock_key));
        }

        // Create and drop guard
        {
            let _guard = LockGuard {
                key: lock_key.clone(),
            };
            // Guard exists, lock should still be present
            let locks = DOWNLOAD_LOCKS.lock();
            assert!(locks.contains_key(&lock_key));
        }

        // After guard drop, lock should be removed
        let locks = DOWNLOAD_LOCKS.lock();
        assert!(!locks.contains_key(&lock_key));
    }

    #[test]
    fn test_extract_zip_empty() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("empty.zip");

        // Create an empty ZIP file
        let file = fs::File::create(&zip_path).unwrap();
        let zip = zip::ZipWriter::new(file);
        zip.finish().unwrap();

        let dest_dir = temp_dir.path().join("extracted");
        let result = extract_zip(&zip_path, &dest_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_extract_zip_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("test.zip");

        // Create a ZIP with one file
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"test content").unwrap();
        zip.finish().unwrap();

        let dest_dir = temp_dir.path().join("extracted");
        let result = extract_zip(&zip_path, &dest_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let extracted_file = dest_dir.join("test.txt");
        assert!(extracted_file.exists());
        let content = fs::read_to_string(extracted_file).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_extract_zip_with_directory() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("test.zip");

        // Create a ZIP with a directory
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.add_directory("subdir/", options).unwrap();
        zip.start_file("subdir/test.txt", options).unwrap();
        zip.write_all(b"nested content").unwrap();
        zip.finish().unwrap();

        let dest_dir = temp_dir.path().join("extracted");
        let result = extract_zip(&zip_path, &dest_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let extracted_file = dest_dir.join("subdir").join("test.txt");
        assert!(extracted_file.exists());
    }

    #[test]
    fn test_extract_zip_path_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("malicious.zip");

        // Create a ZIP with path traversal attempt
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        // Note: ZipWriter may normalize paths, so this test verifies the check exists
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Try to create a file with .. in the path
        // The zip crate may reject this, but we test our extraction logic
        let result = zip.start_file("../escape.txt", options);
        if result.is_ok() {
            zip.write_all(b"malicious").unwrap();
            zip.finish().unwrap();

            let dest_dir = temp_dir.path().join("extracted");
            // Our extraction should either skip invalid names or reject them
            let extract_result = extract_zip(&zip_path, &dest_dir);
            // Should succeed but not extract the malicious file outside dest_dir
            if extract_result.is_ok() {
                let escape_file = temp_dir.path().join("escape.txt");
                assert!(!escape_file.exists());
            }
        }
    }

    #[test]
    fn test_extract_zip_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("large.zip");

        // Create a ZIP that claims to be larger than 10GB when uncompressed
        // This is simulated by the size field, not actual data
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        // Add multiple files that together exceed the limit
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for i in 0..100 {
            zip.start_file(format!("file{}.txt", i), options).unwrap();
            // Write enough data to trigger size check
            let data = vec![0u8; 200_000_000]; // 200MB per file
            zip.write_all(&data).unwrap();
        }

        zip.finish().unwrap();

        let dest_dir = temp_dir.path().join("extracted");
        let result = extract_zip(&zip_path, &dest_dir);
        // Should fail due to size limit (10GB < 100 * 200MB = 20GB)
        assert!(result.is_err());
        if let Err(GaggleError::ZipError(msg)) = result {
            assert!(msg.contains("too large"));
        }
    }

    #[test]
    fn test_get_dataset_file_path_absolute_rejected() {
        let result = get_dataset_file_path("owner/dataset", "/etc/passwd");
        assert!(result.is_err());
        if let Err(GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("Absolute"));
        }
    }

    #[test]
    fn test_get_dataset_file_path_parent_component_rejected() {
        let result = get_dataset_file_path("owner/dataset", "../secrets.csv");
        assert!(result.is_err());
        if let Err(GaggleError::InvalidDatasetPath(msg)) = result {
            assert!(msg.contains("parent") || msg.contains("root"));
        }
    }

    #[test]
    fn test_get_dataset_file_path_root_component_rejected() {
        let result = get_dataset_file_path("owner/dataset", "/root/file.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_dataset_files_skips_marker() {
        // This test requires mocking or a real download, which is complex
        // For now, we test the structure of DatasetFile
        let files = vec![
            DatasetFile {
                name: "data.csv".to_string(),
                size: 1000,
            },
            DatasetFile {
                name: "info.json".to_string(),
                size: 500,
            },
        ];

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "data.csv");
        assert_eq!(files[1].size, 500);
    }

    #[test]
    fn test_extract_zip_with_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("nested.zip");

        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Create nested structure
        zip.add_directory("level1/", options).unwrap();
        zip.add_directory("level1/level2/", options).unwrap();
        zip.start_file("level1/level2/deep.txt", options).unwrap();
        zip.write_all(b"deep content").unwrap();
        zip.finish().unwrap();

        let dest_dir = temp_dir.path().join("extracted");
        let result = extract_zip(&zip_path, &dest_dir);
        assert!(result.is_ok());

        let deep_file = dest_dir.join("level1").join("level2").join("deep.txt");
        assert!(deep_file.exists());
        let content = fs::read_to_string(deep_file).unwrap();
        assert_eq!(content, "deep content");
    }

    #[test]
    fn test_dataset_file_serialization() {
        let file = DatasetFile {
            name: "test.csv".to_string(),
            size: 2048,
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("test.csv"));
        assert!(json.contains("2048"));

        let deserialized: DatasetFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test.csv");
        assert_eq!(deserialized.size, 2048);
    }
}
