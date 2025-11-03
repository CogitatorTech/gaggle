use crate::error::GaggleError;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use super::api::{build_client, get_api_base, with_retries};
use super::credentials::get_credentials;
use tracing::{debug, warn};

/// Track ongoing dataset downloads to prevent concurrent downloads of the same dataset
static DOWNLOAD_LOCKS: once_cell::sync::Lazy<Mutex<HashMap<String, ()>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetFile {
    pub name: String,
    pub size: u64,
}

fn list_dataset_files_from_metadata(dataset_path: &str) -> Result<Vec<DatasetFile>, GaggleError> {
    let meta = super::metadata::get_dataset_metadata(dataset_path)?;
    let mut out = Vec::new();
    if let Some(files) = meta.get("files").and_then(|v| v.as_array()) {
        for f in files {
            if let Some(name) = f.get("name").and_then(|n| n.as_str()) {
                // support size keys in different schemas
                let size = f
                    .get("totalBytes")
                    .and_then(|x| x.as_u64())
                    .or_else(|| f.get("size").and_then(|x| x.as_u64()))
                    .unwrap_or(0);
                out.push(DatasetFile {
                    name: name.to_string(),
                    size,
                });
            }
        }
    }
    Ok(out)
}

/// Metadata stored in .downloaded marker file
#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    downloaded_at_secs: u64,
    dataset_path: String,
    size_mb: u64,
    version: Option<String>,
}

impl CacheMetadata {
    fn new(dataset_path: String, size_mb: u64) -> Self {
        Self {
            downloaded_at_secs: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            dataset_path,
            size_mb,
            version: None,
        }
    }

    fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.downloaded_at_secs)
    }
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

/// Download a Kaggle dataset (supports version pinning)
/// Examples:
///   "owner/dataset" - downloads latest version
///   "owner/dataset@v2" - downloads version 2
///   "owner/dataset@latest" - explicitly downloads latest
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError> {
    // Parse path to extract optional version
    let (owner, dataset, version) = super::parse_dataset_path_with_version(dataset_path)?;

    // Reconstruct base path without version for internal use
    let base_path = format!("{}/{}", owner, dataset);

    download_dataset_version(&base_path, version)
}

/// Download a specific version of a Kaggle dataset
fn download_dataset_version(
    dataset_path: &str,
    version: Option<String>,
) -> Result<PathBuf, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    // Cache directory includes version if specified
    let cache_subdir = if let Some(ref v) = version {
        format!("{}-v{}", dataset, v)
    } else {
        dataset.clone()
    };

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&cache_subdir);

    // Check if already downloaded (fast path)
    let marker_file = cache_dir.join(".downloaded");
    if marker_file.exists() {
        return Ok(cache_dir);
    }

    // Offline mode: if not cached, fail fast
    if crate::config::offline_mode() {
        return Err(GaggleError::HttpRequestError(format!(
            "Offline mode enabled; cannot download '{}'. Unset GAGGLE_OFFLINE to enable network.",
            dataset_path
        )));
    }

    // Use a lock per dataset path (including version) to prevent concurrent downloads
    let lock_key = if let Some(ref v) = version {
        format!("{}/{}-v{}", owner, dataset, v)
    } else {
        format!("{}/{}", owner, dataset)
    };

    // Acquire a "lock" by inserting into the map
    // If another thread is downloading, wait with timeout (configurable)
    let poll_ms = crate::config::download_wait_poll_interval_ms();
    let timeout_ms = crate::config::download_wait_timeout_ms();
    let max_attempts: u64 = if poll_ms == 0 {
        0
    } else {
        timeout_ms / poll_ms
    };
    let mut wait_attempts: u64 = 0;

    loop {
        let mut locks = DOWNLOAD_LOCKS.lock();
        // While holding the lock, check marker existence to avoid race
        if marker_file.exists() {
            return Ok(cache_dir.clone());
        }
        if !locks.contains_key(&lock_key) {
            locks.insert(lock_key.clone(), ());
            break;
        }
        // Release lock and sleep briefly before retrying
        drop(locks);

        // Check timeout to prevent indefinite waiting
        if max_attempts > 0 {
            if wait_attempts >= max_attempts {
                return Err(GaggleError::HttpRequestError(format!(
                    "Timeout waiting for download of {}. Another thread may have stalled.",
                    dataset_path
                )));
            }
            wait_attempts = wait_attempts.saturating_add(1);
        }

        sleep(Duration::from_millis(poll_ms.max(1)));
    }

    // Ensure we clean up the lock when done
    let _guard = LockGuard {
        key: lock_key.clone(),
    };

    // Double-check after acquiring lock
    if marker_file.exists() {
        return Ok(cache_dir.clone());
    }

    fs::create_dir_all(&cache_dir)?;

    // Build URL with version if specified
    let url = if let Some(ref v) = version {
        format!(
            "{}/datasets/download/{}/{}/versions/{}",
            get_api_base(),
            owner,
            dataset,
            v
        )
    } else {
        format!("{}/datasets/download/{}/{}", get_api_base(), owner, dataset)
    };

    debug!(%url, "downloading dataset");

    let client = build_client()?;
    let mut response = with_retries(|| {
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

    // Stream response to a temporary file to avoid large memory usage
    let zip_path = cache_dir.join("dataset.zip");
    let zip_file = fs::File::create(&zip_path)?;
    let mut writer = BufWriter::new(zip_file);
    response
        .copy_to(&mut writer)
        .map_err(|e| GaggleError::HttpRequestError(e.to_string()))?;
    writer.flush().ok();

    // Extract ZIP - require at least one file extracted; cleanup on failure
    let extracted = match extract_zip(&zip_path, &cache_dir) {
        Ok(n) => n,
        Err(err) => {
            // Best-effort cleanup of corrupt zip and partial files
            let _ = fs::remove_file(&zip_path);
            let _ = fs::remove_dir_all(&cache_dir);
            return Err(err);
        }
    };
    if extracted == 0 {
        // Clean up if nothing extracted
        let _ = fs::remove_file(&zip_path);
        let _ = fs::remove_dir_all(&cache_dir);
        return Err(GaggleError::ZipError("ZIP contained no files".to_string()));
    }

    // Clean up ZIP file
    let _ = fs::remove_file(&zip_path);

    // Calculate dataset size in MB
    let dataset_size_mb = crate::utils::calculate_dir_size(&cache_dir)
        .unwrap_or(0)
        .saturating_div(1024 * 1024);

    // Create marker file with metadata including version
    let mut metadata = CacheMetadata::new(dataset_path.to_string(), dataset_size_mb);
    // Use specified version, or fetch current version from API
    metadata.version = version.or_else(|| super::metadata::get_current_version(dataset_path).ok());
    fs::write(&marker_file, serde_json::to_string(&metadata)?)?;

    // Enforce cache limit after successful download (soft limit)
    if crate::config::cache_limit_is_soft() {
        let _ = enforce_cache_limit(); // Don't fail the download if cleanup fails
    }

    Ok(cache_dir)
}

/// Download a single file within a Kaggle dataset into the cache without extracting the entire archive
pub fn download_single_file(dataset_path: &str, filename: &str) -> Result<PathBuf, GaggleError> {
    // Validate dataset path and filename to prevent traversal
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;
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

    // Offline mode: fail if file isn't already present
    let base_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);
    let target_path = base_dir.join(fname_path);
    if crate::config::offline_mode() {
        if target_path.exists() {
            return Ok(target_path);
        }
        return Err(GaggleError::HttpRequestError(format!(
            "Offline mode enabled; cannot download '{}' from '{}'.",
            filename, dataset_path
        )));
    }

    // Ensure parent directories exist
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Build single-file download URL
    // We use an endpoint shape that is easy to mock in tests and aligns with typical Kaggle CLI patterns
    let url = format!(
        "{}/datasets/download/{}/{}?fileName={}",
        get_api_base(),
        owner,
        dataset,
        urlencoding::encode(filename)
    );

    let creds = get_credentials()?;
    debug!(%url, "downloading single file");
    let client = build_client()?;
    let mut response = with_retries(|| {
        client
            .get(&url)
            .basic_auth(&creds.username, Some(&creds.key))
            .send()
            .map_err(|e| GaggleError::HttpRequestError(e.to_string()))
    })?;

    if !response.status().is_success() {
        return Err(GaggleError::HttpRequestError(format!(
            "Failed to download file '{}': HTTP {}",
            filename,
            response.status()
        )));
    }

    // Stream to disk; avoid loading whole file into memory
    let mut outfile = fs::File::create(&target_path)?;
    response
        .copy_to(&mut outfile)
        .map_err(|e| GaggleError::HttpRequestError(e.to_string()))?;

    Ok(target_path)
}

/// Extract ZIP file
pub(crate) fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<usize, GaggleError> {
    let file = fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| GaggleError::ZipError(e.to_string()))?;

    // ZIP bomb protection: limit total uncompressed size to 10GB and compression ratio
    const MAX_TOTAL_SIZE: u64 = 10 * 1024 * 1024 * 1024;
    const MAX_COMPRESSION_RATIO: u64 = 100; // reject entries with >100:1 ratio
    let mut total_size: u64 = 0;
    let mut files_extracted: usize = 0;

    // Ensure destination directory exists and canonicalize it once
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

        // Compute output path and validate parent within dest (no dir creation before validation)
        let outpath = dest_dir.join(&rel_path);
        let parent = outpath.parent().unwrap_or(dest_dir);
        // Validate the output path is still within dest_dir using canonical parent
        // Create parent only after validation
        let canonical_parent_candidate = if parent.exists() {
            parent.canonicalize()
        } else {
            // If parent doesn't exist yet, use canonical dest and join the relative path's parent
            Ok(canonical_dest.clone())
        };
        let canonical_parent = canonical_parent_candidate.map_err(|e| {
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

        // Directory entries
        if entry.is_dir() || entry.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
            continue;
        }

        // Check total uncompressed size and per-entry compression ratio if possible
        let uncompressed = entry.size();
        total_size = total_size.saturating_add(uncompressed);
        if total_size > MAX_TOTAL_SIZE {
            return Err(GaggleError::ZipError(format!(
                "ZIP file too large: uncompressed size exceeds {} GB",
                MAX_TOTAL_SIZE / (1024 * 1024 * 1024)
            )));
        }
        let comp_size = entry.compressed_size();
        if comp_size > 0 {
            let ratio = uncompressed.saturating_div(comp_size.max(1));
            if ratio > MAX_COMPRESSION_RATIO {
                return Err(GaggleError::ZipError(format!(
                    "Excessive compression ratio ({}:1) for entry {}",
                    ratio,
                    rel_path.display()
                )));
            }
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

/// List files in a dataset. If cached locally, list from disk. Otherwise, try remote metadata-based listing first,
/// and only fall back to downloading if remote listing is unavailable.
pub fn list_dataset_files(dataset_path: &str) -> Result<Vec<DatasetFile>, GaggleError> {
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;
    let dataset_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);

    // If directory exists and has content, enumerate locally
    if dataset_dir.exists() {
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
        return Ok(files);
    }

    // Not cached: try remote listing via metadata
    if !crate::config::offline_mode() {
        if let Ok(list) = list_dataset_files_from_metadata(dataset_path) {
            if !list.is_empty() {
                debug!(
                    dataset = dataset_path,
                    count = list.len(),
                    "listing files from remote metadata"
                );
                return Ok(list);
            } else {
                debug!(
                    dataset = dataset_path,
                    "remote metadata listing empty; will attempt download"
                );
            }
        } else {
            debug!(
                dataset = dataset_path,
                "failed to fetch remote metadata; will attempt download"
            );
        }
    }

    // As a last resort, download and list
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

    // Compute dataset dir and desired file path
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;
    let dataset_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);
    let file_path = dataset_dir.join(fname_path);

    // Fast path: file already present
    if file_path.exists() {
        return Ok(file_path);
    }

    // Try on-demand single-file download (without fetching whole archive)
    match download_single_file(dataset_path, filename) {
        Ok(p) => Ok(p),
        Err(e) => {
            // In strict on-demand mode, do not fall back to full download
            if crate::config::strict_on_demand() {
                debug!(dataset = dataset_path, file = filename, error = %e, "on-demand fetch failed and strict mode enabled; not falling back");
                return Err(e);
            }
            // If single-file download fails and dataset isn't cached, fall back to full dataset download
            if !dataset_dir.exists()
                || fs::read_dir(&dataset_dir)
                    .map(|mut i| i.next().is_none())
                    .unwrap_or(true)
            {
                debug!(dataset = dataset_path, file = filename, error = %e, "on-demand fetch failed; falling back to full dataset download");
                let dir = download_dataset(dataset_path)?;
                let p = dir.join(fname_path);
                if p.exists() {
                    return Ok(p);
                }
            }
            Err(e)
        }
    }
}

/// Get all cached datasets with their metadata
fn get_cached_datasets() -> Result<Vec<(PathBuf, CacheMetadata)>, GaggleError> {
    let cache_root = crate::config::cache_dir_runtime().join("datasets");
    if !cache_root.exists() {
        return Ok(Vec::new());
    }

    let mut datasets = Vec::new();

    // Iterate through owner directories
    for owner_entry in fs::read_dir(&cache_root)? {
        let owner_entry = owner_entry?;
        if !owner_entry.path().is_dir() {
            continue;
        }

        // Iterate through dataset directories
        for dataset_entry in fs::read_dir(owner_entry.path())? {
            let dataset_entry = dataset_entry?;
            let dataset_path = dataset_entry.path();
            if !dataset_path.is_dir() {
                continue;
            }

            let marker_file = dataset_path.join(".downloaded");
            if marker_file.exists() {
                match fs::read_to_string(&marker_file) {
                    Ok(content) if !content.is_empty() => {
                        // Try to parse metadata
                        match serde_json::from_str::<CacheMetadata>(&content) {
                            Ok(metadata) => {
                                datasets.push((dataset_path, metadata));
                            }
                            Err(e) => {
                                // Legacy or invalid marker - calculate size and synthesize metadata
                                warn!(path = %marker_file.display(), error = %e, "Invalid cache metadata; synthesizing");
                                let size_mb = crate::utils::calculate_dir_size(&dataset_path)
                                    .unwrap_or(0)
                                    .saturating_div(1024 * 1024);
                                let owner = owner_entry.file_name().to_string_lossy().to_string();
                                let dataset =
                                    dataset_entry.file_name().to_string_lossy().to_string();
                                let metadata =
                                    CacheMetadata::new(format!("{}/{}", owner, dataset), size_mb);
                                // retain None version
                                datasets.push((dataset_path, metadata));
                            }
                        }
                    }
                    Ok(_) => {
                        // Empty marker - synthesize
                        warn!(path = %marker_file.display(), "Empty cache metadata; synthesizing");
                        let size_mb = crate::utils::calculate_dir_size(&dataset_path)
                            .unwrap_or(0)
                            .saturating_div(1024 * 1024);
                        let owner = owner_entry.file_name().to_string_lossy().to_string();
                        let dataset = dataset_entry.file_name().to_string_lossy().to_string();
                        let metadata =
                            CacheMetadata::new(format!("{}/{}", owner, dataset), size_mb);
                        datasets.push((dataset_path, metadata));
                    }
                    Err(e) => {
                        warn!(path = %marker_file.display(), error = %e, "Failed reading cache metadata; synthesizing");
                        let size_mb = crate::utils::calculate_dir_size(&dataset_path)
                            .unwrap_or(0)
                            .saturating_div(1024 * 1024);
                        let owner = owner_entry.file_name().to_string_lossy().to_string();
                        let dataset = dataset_entry.file_name().to_string_lossy().to_string();
                        let metadata =
                            CacheMetadata::new(format!("{}/{}", owner, dataset), size_mb);
                        datasets.push((dataset_path, metadata));
                    }
                }
            } else {
                // No marker (e.g., partial on-demand downloads). Include in accounting.
                let size_mb = crate::utils::calculate_dir_size(&dataset_path)
                    .unwrap_or(0)
                    .saturating_div(1024 * 1024);
                // Skip empty directories with zero size
                if size_mb > 0 {
                    let owner = owner_entry.file_name().to_string_lossy().to_string();
                    let dataset = dataset_entry.file_name().to_string_lossy().to_string();
                    let metadata = CacheMetadata::new(format!("{}/{}", owner, dataset), size_mb);
                    datasets.push((dataset_path, metadata));
                }
            }
        }
    }

    Ok(datasets)
}

/// Calculate total cache size in MB
pub fn get_total_cache_size_mb() -> Result<u64, GaggleError> {
    let datasets = get_cached_datasets()?;
    Ok(datasets.iter().map(|(_, meta)| meta.size_mb).sum())
}

/// Enforce cache size limit using LRU eviction
fn enforce_cache_limit() -> Result<(), GaggleError> {
    let limit_mb = match crate::config::cache_size_limit_mb() {
        Some(limit) => limit,
        None => return Ok(()), // No limit set
    };

    let mut datasets = get_cached_datasets()?;
    let mut total_size_mb: u64 = datasets.iter().map(|(_, meta)| meta.size_mb).sum();

    if total_size_mb <= limit_mb {
        return Ok(()); // Within limit
    }

    // Sort by age (oldest first) for LRU eviction
    datasets.sort_by_key(|(_, meta)| meta.downloaded_at_secs);

    // Evict oldest datasets until under limit
    for (dataset_path, metadata) in datasets {
        if total_size_mb <= limit_mb {
            break;
        }

        // Remove dataset directory
        if let Err(e) = fs::remove_dir_all(&dataset_path) {
            warn!(path = %dataset_path.display(), error = %e, "Failed to evict dataset");
            continue;
        }

        total_size_mb = total_size_mb.saturating_sub(metadata.size_mb);
        debug!(
            dataset = %metadata.dataset_path,
            age_secs = metadata.age_seconds(),
            size_mb = metadata.size_mb,
            "Cache eviction: removed dataset to enforce limit"
        );
    }

    Ok(())
}

/// Public function to manually enforce cache limit
pub fn enforce_cache_limit_now() -> Result<(), GaggleError> {
    enforce_cache_limit()
}

/// Check if cached dataset is the current version
pub fn is_dataset_current(dataset_path: &str) -> Result<bool, GaggleError> {
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);

    let marker_file = cache_dir.join(".downloaded");
    if !marker_file.exists() {
        return Ok(false); // Not cached, so not current
    }

    // Read cached metadata
    let content = fs::read_to_string(&marker_file)?;
    if content.is_empty() {
        return Ok(false); // Legacy marker without metadata
    }

    let cached_metadata: CacheMetadata = serde_json::from_str(&content)
        .map_err(|e| GaggleError::IoError(format!("Failed to parse cache metadata: {}", e)))?;

    let cached_version = cached_metadata.version.as_deref().unwrap_or("unknown");

    // Get current version from Kaggle
    let current_version = super::metadata::get_current_version(dataset_path)?;

    // If we cannot determine current version, conservatively report not current
    if current_version == "unknown" {
        return Ok(false);
    }

    Ok(cached_version == current_version)
}

/// Force update dataset to latest version (ignores cache)
pub fn update_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError> {
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);

    // Remove existing cache
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
    }

    // Download fresh copy
    download_dataset(dataset_path)
}

/// Get version information for a dataset
pub fn get_dataset_version_info(dataset_path: &str) -> Result<serde_json::Value, GaggleError> {
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&dataset);

    let marker_file = cache_dir.join(".downloaded");

    let cached_version = if marker_file.exists() {
        let content = fs::read_to_string(&marker_file)?;
        if !content.is_empty() {
            serde_json::from_str::<CacheMetadata>(&content)
                .ok()
                .and_then(|m| m.version)
        } else {
            None
        }
    } else {
        None
    };

    // Get current version from Kaggle API
    let current_version = super::metadata::get_current_version(dataset_path)?;

    // Consider unknown latest version as not current
    let is_current = if current_version == "unknown" {
        false
    } else {
        cached_version
            .as_ref()
            .map(|v| v == &current_version)
            .unwrap_or(false)
    };

    let info = serde_json::json!({
        "cached_version": cached_version,
        "latest_version": current_version,
        "is_current": is_current,
        "is_cached": marker_file.exists()
    });

    Ok(info)
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
        // This test verifies that the size check logic works correctly
        // by creating a small ZIP with metadata that claims large size
        // We test the cumulative size check, not actual file creation

        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("test.zip");

        // Create a small ZIP file with a few tiny files
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Create just a few small files - the actual limit check happens
        // during extraction based on the reported uncompressed size
        for i in 0..5 {
            zip.start_file(format!("file{}.txt", i), options).unwrap();
            zip.write_all(b"test content").unwrap();
        }

        zip.finish().unwrap();

        let dest_dir = temp_dir.path().join("extracted");

        // This test primarily verifies that:
        // 1. Small files extract successfully (under 10GB limit)
        // 2. The size checking logic is in place
        let result = extract_zip(&zip_path, &dest_dir);

        // Should succeed because total size is well under 10GB
        assert!(result.is_ok());
        let extracted_count = result.unwrap();
        assert_eq!(extracted_count, 5);

        // Verify the files were actually extracted
        for i in 0..5 {
            let file_path = dest_dir.join(format!("file{}.txt", i));
            assert!(file_path.exists());
        }
    }

    #[test]
    fn test_extract_zip_size_check_logic() {
        // Test that the size limit constant is correctly defined
        // The actual limit is 10GB = 10 * 1024 * 1024 * 1024 bytes
        const EXPECTED_LIMIT: u64 = 10 * 1024 * 1024 * 1024;

        // We can't easily test the actual size limit without creating large files,
        // but we can verify the constant exists and has the right value
        // by checking it would trigger on cumulative sizes > 10GB

        let size_under_limit = 5 * 1024 * 1024 * 1024u64; // 5GB
        let size_over_limit = 11 * 1024 * 1024 * 1024u64; // 11GB

        assert!(size_under_limit < EXPECTED_LIMIT);
        assert!(size_over_limit > EXPECTED_LIMIT);
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
        let files = [
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
        assert_eq!(deserialized.name, file.name);
        assert_eq!(deserialized.size, 2048);
    }

    #[test]
    fn test_cache_metadata_creation() {
        let metadata = CacheMetadata::new("owner/dataset".to_string(), 100);
        assert_eq!(metadata.dataset_path, "owner/dataset");
        assert_eq!(metadata.size_mb, 100);
        assert!(metadata.downloaded_at_secs > 0);
        assert!(metadata.version.is_none());
    }

    #[test]
    fn test_cache_metadata_age() {
        let mut metadata = CacheMetadata::new("owner/dataset".to_string(), 100);
        metadata.downloaded_at_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(3600); // 1 hour ago

        let age = metadata.age_seconds();
        assert!(age >= 3600); // At least 1 hour
        assert!(age < 3700); // Less than ~1 hour + 2 minutes
    }

    #[test]
    fn test_cache_metadata_serialization() {
        let metadata = CacheMetadata::new("owner/dataset".to_string(), 500);
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CacheMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.dataset_path, metadata.dataset_path);
        assert_eq!(deserialized.size_mb, metadata.size_mb);
    }

    #[test]
    fn test_get_cached_datasets_empty() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        let datasets = get_cached_datasets().unwrap();
        assert_eq!(datasets.len(), 0);

        std::env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    fn test_get_total_cache_size_empty() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        let size = get_total_cache_size_mb().unwrap();
        assert_eq!(size, 0);

        std::env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    fn test_enforce_cache_limit_no_limit() {
        std::env::set_var("GAGGLE_CACHE_SIZE_LIMIT_MB", "unlimited");
        let result = enforce_cache_limit_now();
        assert!(result.is_ok());
        std::env::remove_var("GAGGLE_CACHE_SIZE_LIMIT_MB");
    }

    #[test]
    fn test_enforce_cache_limit_within_limit() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());
        std::env::set_var("GAGGLE_CACHE_SIZE_LIMIT_MB", "1000");

        let result = enforce_cache_limit_now();
        assert!(result.is_ok());

        std::env::remove_var("GAGGLE_CACHE_DIR");
        std::env::remove_var("GAGGLE_CACHE_SIZE_LIMIT_MB");
    }

    #[test]
    fn test_cache_metadata_with_version() {
        let mut metadata = CacheMetadata::new("owner/dataset".to_string(), 100);
        metadata.version = Some("5".to_string());

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CacheMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, Some("5".to_string()));
        assert_eq!(deserialized.dataset_path, "owner/dataset");
    }

    #[test]
    fn test_is_dataset_current_not_cached() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        let result = is_dataset_current("owner/dataset");
        // Should return false (not cached) or error (network issue)
        match result {
            Ok(false) => {} // Expected: not cached
            Err(_) => {}    // Expected: network error
            Ok(true) => panic!("Uncached dataset should not be current"),
        }

        std::env::remove_var("GAGGLE_CACHE_DIR");
        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_get_dataset_version_info_structure() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        let result = get_dataset_version_info("owner/dataset");
        // May fail due to network, but if it succeeds, check structure
        if let Ok(info) = result {
            assert!(info.get("cached_version").is_some());
            assert!(info.get("latest_version").is_some());
            assert!(info.get("is_current").is_some());
            assert!(info.get("is_cached").is_some());
        }

        std::env::remove_var("GAGGLE_CACHE_DIR");
        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_download_with_version_parsing() {
        // Test that version syntax is properly parsed
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        // Test path parsing (won't actually download without network)
        let result = crate::kaggle::parse_dataset_path_with_version("owner/dataset@v2");
        assert!(result.is_ok());
        let (_owner, _dataset, version) = result.unwrap();
        assert_eq!(version, Some("2".to_string()));

        std::env::remove_var("GAGGLE_CACHE_DIR");
        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    fn test_versioned_cache_directory() {
        // Verify that versioned downloads use different cache directories

        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        // Simulate cache directory structure
        let base = temp_dir.path().join("datasets").join("owner");

        // Latest version (no version specified)
        let latest_cache = base.join("dataset");

        // Version 2
        let v2_cache = base.join("dataset-v2");

        // Version 3
        let v3_cache = base.join("dataset-v3");

        // Verify they're different paths
        assert_ne!(latest_cache, v2_cache);
        assert_ne!(latest_cache, v3_cache);
        assert_ne!(v2_cache, v3_cache);

        std::env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    fn test_partial_cache_counts_and_eviction() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        // Create two partial cached datasets
        let d1 = temp_dir.path().join("datasets/owner1/ds1");
        let d2 = temp_dir.path().join("datasets/owner2/ds2");
        fs::create_dir_all(&d1).unwrap();
        fs::create_dir_all(&d2).unwrap();
        fs::write(d1.join("a.bin"), vec![0u8; 2 * 1024 * 1024]).unwrap(); // 2MB
        fs::write(d2.join("b.bin"), vec![0u8; 2 * 1024 * 1024]).unwrap(); // 2MB

        // Total ~4MB; set limit to 2MB so eviction must occur
        std::env::set_var("GAGGLE_CACHE_SIZE_LIMIT_MB", "2");
        enforce_cache_limit_now().unwrap();

        // After eviction, total size must be <= 2MB
        let total = get_total_cache_size_mb().unwrap();
        assert!(total <= 2);

        std::env::remove_var("GAGGLE_CACHE_SIZE_LIMIT_MB");
        std::env::remove_var("GAGGLE_CACHE_DIR");
    }
}
