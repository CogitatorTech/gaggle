// The public C API layer and module declarations for Gaggle - Kaggle Dataset DuckDB Extension

use serde_json::json;
use std::ffi::{c_char, CStr, CString};
use std::fs;

// Declare the internal modules
mod config;
mod error;
mod kaggle;

// Re-export the public FFI utility functions
pub use error::gaggle_last_error;

/// Set Kaggle API credentials
///
/// # Arguments
///
/// * `username` - A pointer to a null-terminated C string representing the Kaggle username.
/// * `key` - A pointer to a null-terminated C string representing the Kaggle API key.
///
/// # Returns
///
/// * `0` on success.
/// * `-1` on failure. Call `gaggle_last_error()` to get a descriptive error message.
///
/// # Safety
///
/// * The `username` and `key` pointers must not be null.
/// * The memory pointed to by `username` and `key` must be valid, null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn gaggle_set_credentials(username: *const c_char, key: *const c_char) -> i32 {
    let result = (|| -> Result<(), error::GaggleError> {
        if username.is_null() || key.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let username_str = CStr::from_ptr(username).to_str()?;
        let key_str = CStr::from_ptr(key).to_str()?;

        kaggle::set_credentials(username_str, key_str)
    })();

    match result {
        Ok(()) => 0,
        Err(e) => {
            error::set_last_error(&e);
            -1
        }
    }
}

/// Download a Kaggle dataset and return its local cache path
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path (e.g., "owner/dataset-name").
///
/// # Returns
///
/// A pointer to a null-terminated C string containing the local path, or NULL on failure.
/// The caller must free this pointer using `gaggle_free()`.
///
/// # Safety
///
/// * The `dataset_path` pointer must not be null.
/// * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_download_dataset(dataset_path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;

        let local_path = kaggle::download_dataset(path_str)?;
        Ok(local_path.to_string_lossy().to_string())
    })();

    match result {
        Ok(path) => CString::new(path).unwrap().into_raw(),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

/// Get the local path to a specific file in a downloaded dataset
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
/// * `filename` - A pointer to a null-terminated C string representing the filename.
///
/// # Returns
///
/// A pointer to a null-terminated C string containing the file path, or NULL on failure.
/// The caller must free this pointer using `gaggle_free()`.
///
/// # Safety
///
/// * The pointers must not be null.
/// * The memory pointed to must be valid, null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn gaggle_get_file_path(
    dataset_path: *const c_char,
    filename: *const c_char,
) -> *mut c_char {
    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() || filename.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;
        let filename_str = CStr::from_ptr(filename).to_str()?;

        let file_path = kaggle::get_dataset_file_path(path_str, filename_str)?;
        Ok(file_path.to_string_lossy().to_string())
    })();

    match result {
        Ok(path) => CString::new(path).unwrap().into_raw(),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

/// List files in a Kaggle dataset
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
///
/// # Returns
///
/// A pointer to a null-terminated C string containing JSON array of files, or NULL on failure.
/// The caller must free this pointer using `gaggle_free()`.
///
/// # Safety
///
/// * The `dataset_path` pointer must not be null.
/// * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_list_files(dataset_path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;

        let files = kaggle::list_dataset_files(path_str)?;
        let json = serde_json::to_string(&files)?;
        Ok(json)
    })();

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

/// Search for Kaggle datasets
///
/// # Arguments
///
/// * `query` - A pointer to a null-terminated C string representing the search query.
/// * `page` - Page number (1-indexed).
/// * `page_size` - Number of results per page.
///
/// # Returns
///
/// A pointer to a null-terminated C string containing JSON search results, or NULL on failure.
/// The caller must free this pointer using `gaggle_free()`.
///
/// # Safety
///
/// * The `query` pointer must not be null.
/// * The memory pointed to by `query` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_search(
    query: *const c_char,
    page: i32,
    page_size: i32,
) -> *mut c_char {
    let result = (|| -> Result<String, error::GaggleError> {
        if query.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let query_str = CStr::from_ptr(query).to_str()?;

        let results = kaggle::search_datasets(query_str, page, page_size)?;
        let json = serde_json::to_string(&results)?;
        Ok(json)
    })();

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

/// Get metadata for a specific Kaggle dataset
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
///
/// # Returns
///
/// A pointer to a null-terminated C string containing JSON metadata, or NULL on failure.
/// The caller must free this pointer using `gaggle_free()`.
///
/// # Safety
///
/// * The `dataset_path` pointer must not be null.
/// * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_get_dataset_info(dataset_path: *const c_char) -> *mut c_char {
    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;

        let metadata = kaggle::get_dataset_metadata(path_str)?;
        let json = serde_json::to_string(&metadata)?;
        Ok(json)
    })();

    match result {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

/// Get version information
///
/// # Returns
///
/// A pointer to a null-terminated C string containing JSON version info.
/// The caller must free this pointer using `gaggle_free()`.
#[no_mangle]
pub extern "C" fn gaggle_get_version() -> *mut c_char {
    let version_info = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": "Gaggle - Kaggle Dataset DuckDB Extension",
    });

    CString::new(version_info.to_string())
        .unwrap()
        .into_raw()
}

/// Frees a heap-allocated C string
///
/// # Safety
///
/// The `ptr` must be a non-null pointer to a C string that was previously allocated
/// by a Gaggle function.
#[no_mangle]
pub unsafe extern "C" fn gaggle_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Clear the dataset cache
///
/// # Returns
///
/// * `0` on success.
/// * `-1` on failure.
#[no_mangle]
pub extern "C" fn gaggle_clear_cache() -> i32 {
    let result = (|| -> Result<(), error::GaggleError> {
        let cache_dir = &config::CONFIG.cache_dir;
        if cache_dir.exists() {
            fs::remove_dir_all(cache_dir)?;
            fs::create_dir_all(cache_dir)?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => 0,
        Err(e) => {
            error::set_last_error(&e);
            -1
        }
    }
}

/// Get cache information
///
/// # Returns
///
/// A pointer to a null-terminated C string containing JSON cache info.
/// The caller must free this pointer using `gaggle_free()`.
#[no_mangle]
pub extern "C" fn gaggle_get_cache_info() -> *mut c_char {
    let cache_dir = &config::CONFIG.cache_dir;

    let size = calculate_dir_size(cache_dir).unwrap_or(0);
    let info = json!({
        "cache_dir": cache_dir.to_string_lossy(),
        "size_bytes": size,
        "size_mb": size / (1024 * 1024),
    });

    CString::new(info.to_string())
        .unwrap()
        .into_raw()
}

fn calculate_dir_size(path: &std::path::Path) -> Result<u64, std::io::Error> {
    let mut total = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                total += calculate_dir_size(&entry.path())?;
            } else {
                total += metadata.len();
            }
        }
    }
    Ok(total)
}

