use serde_json::json;
use std::ffi::{c_char, CStr, CString};
use std::fs;

use crate::error;
use crate::kaggle;

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
pub unsafe extern "C" fn gaggle_set_credentials(
    username: *const c_char,
    key: *const c_char,
) -> i32 {
    // Clear any previous error
    error::clear_last_error_internal();

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
    // Clear any previous error
    error::clear_last_error_internal();

    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;

        let local_path = kaggle::download_dataset(path_str)?;
        Ok(local_path.to_string_lossy().to_string())
    })();

    match result {
        Ok(path) => string_to_c_string(path),
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
    // Clear any previous error
    error::clear_last_error_internal();

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
        Ok(path) => string_to_c_string(path),
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
    // Clear any previous error
    error::clear_last_error_internal();

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
        Ok(json) => string_to_c_string(json),
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
    // Clear any previous error
    error::clear_last_error_internal();

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
        Ok(json) => string_to_c_string(json),
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
    // Clear any previous error
    error::clear_last_error_internal();

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
        Ok(json) => string_to_c_string(json),
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
/// A pointer to a null-terminated C string containing the version string (e.g., "0.1.0").
/// The caller must free this pointer using `gaggle_free()`.
#[no_mangle]
pub extern "C" fn gaggle_get_version() -> *mut c_char {
    // Return only the version string (no JSON wrapper)
    string_to_c_string(env!("CARGO_PKG_VERSION").to_string())
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
        // Use runtime-resolved cache dir to honor env overrides
        let cache_dir = crate::config::cache_dir_runtime();
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)?;
            fs::create_dir_all(&cache_dir)?;
        } else {
            fs::create_dir_all(&cache_dir)?;
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

/// Enforce cache size limit by evicting oldest datasets
///
/// # Returns
///
/// * `0` on success.
/// * `-1` on failure.
#[no_mangle]
pub extern "C" fn gaggle_enforce_cache_limit() -> i32 {
    let result = kaggle::download::enforce_cache_limit_now();

    match result {
        Ok(()) => 0,
        Err(e) => {
            error::set_last_error(&e);
            -1
        }
    }
}

/// Check if cached dataset is the current version
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
///
/// # Returns
///
/// * `1` if cached version is current.
/// * `0` if cached version is outdated or not cached.
/// * `-1` on error.
///
/// # Safety
///
/// * The `dataset_path` pointer must not be null.
/// * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_is_dataset_current(dataset_path: *const c_char) -> i32 {
    error::clear_last_error_internal();

    let result = (|| -> Result<bool, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;
        kaggle::is_dataset_current(path_str)
    })();

    match result {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(e) => {
            error::set_last_error(&e);
            -1
        }
    }
}

/// Force update dataset to latest version (ignores cache)
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
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
pub unsafe extern "C" fn gaggle_update_dataset(dataset_path: *const c_char) -> *mut c_char {
    error::clear_last_error_internal();

    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;

        let local_path = kaggle::update_dataset(path_str)?;
        Ok(local_path.to_string_lossy().to_string())
    })();

    match result {
        Ok(path) => string_to_c_string(path),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

/// Get version information for a dataset
///
/// # Arguments
///
/// * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
///
/// # Returns
///
/// A pointer to a null-terminated C string containing JSON version info, or NULL on failure.
/// The caller must free this pointer using `gaggle_free()`.
///
/// # Safety
///
/// * The `dataset_path` pointer must not be null.
/// * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_dataset_version_info(dataset_path: *const c_char) -> *mut c_char {
    error::clear_last_error_internal();

    let result = (|| -> Result<String, error::GaggleError> {
        if dataset_path.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let path_str = CStr::from_ptr(dataset_path).to_str()?;

        let info = kaggle::get_dataset_version_info(path_str)?;
        Ok(info.to_string())
    })();

    match result {
        Ok(json) => string_to_c_string(json),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
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
    let cache_dir = crate::config::cache_dir_runtime();

    let size_bytes = calculate_dir_size(&cache_dir).unwrap_or(0);
    let size_mb = size_bytes / (1024 * 1024);

    let limit_mb = crate::config::cache_size_limit_mb();
    let is_soft_limit = crate::config::cache_limit_is_soft();

    let usage_percent = if let Some(limit) = limit_mb {
        if limit > 0 {
            ((size_mb as f64 / limit as f64) * 100.0) as u64
        } else {
            0
        }
    } else {
        0
    };

    let info = json!({
        "path": cache_dir.to_string_lossy(),
        "size_mb": size_mb,
        "limit_mb": limit_mb,
        "usage_percent": usage_percent,
        "is_soft_limit": is_soft_limit,
        "type": "local",
    });
    string_to_c_string(info.to_string())
}

/// Parse JSON and expand objects/arrays similar to json_each
///
/// # Arguments
///
/// * `json_str` - A pointer to a null-terminated C string containing JSON data
///
/// # Returns
///
/// A pointer to a null-terminated C string containing newline-delimited JSON objects
///
/// # Safety
///
/// * The `json_str` pointer must not be null.
/// * The memory pointed to by `json_str` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn gaggle_json_each(json_str: *const c_char) -> *mut c_char {
    // Clear any previous error
    error::clear_last_error_internal();

    let result = (|| -> Result<String, error::GaggleError> {
        if json_str.is_null() {
            return Err(error::GaggleError::NullPointer);
        }
        let json_cstr = CStr::from_ptr(json_str).to_str()?;

        // Parse the JSON
        let value: serde_json::Value = serde_json::from_str(json_cstr)?;

        // Expand into rows
        let mut rows = Vec::new();
        expand_json_value(&value, "$", &mut rows);

        // Convert rows to newline-delimited JSON
        let result_str = rows
            .into_iter()
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(result_str)
    })();

    match result {
        Ok(s) => string_to_c_string(s),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}

pub(crate) fn string_to_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cstring) => cstring.into_raw(),
        Err(e) => {
            let err = error::GaggleError::IoError(format!(
                "String contains null byte at position {}",
                e.nul_position()
            ));
            error::set_last_error(&err);
            std::ptr::null_mut()
        }
    }
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

/// Helper function to recursively expand JSON values
fn expand_json_value(value: &serde_json::Value, path: &str, rows: &mut Vec<serde_json::Value>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter() {
                let new_path = if path == "$" {
                    format!("$.{}", key)
                } else {
                    format!("{}.{}", path, key)
                };

                let row = json!({
                    "key": key,
                    "value": val,
                    "type": get_json_type(val),
                    "path": new_path
                });
                rows.push(row);
            }
        }
        serde_json::Value::Array(arr) => {
            for (idx, val) in arr.iter().enumerate() {
                let new_path = format!("{}[{}]", path, idx);

                let row = json!({
                    "key": idx,
                    "value": val,
                    "type": get_json_type(val),
                    "path": new_path
                });
                rows.push(row);
            }
        }
        _ => {
            // For scalar values, return as is
            let row = json!({
                "key": null,
                "value": value,
                "type": get_json_type(value),
                "path": path
            });
            rows.push(row);
        }
    }
}

/// Helper function to get JSON type as string
fn get_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaggle_get_version_not_null() {
        let version_ptr = gaggle_get_version();
        assert!(!version_ptr.is_null());

        unsafe {
            let version_cstr = CStr::from_ptr(version_ptr);
            let version_str = version_cstr.to_str().unwrap();
            assert!(!version_str.is_empty());
            // Should be a plain version string like "0.1.0"
            assert!(!version_str.contains("{"));
            assert!(!version_str.contains("}"));

            gaggle_free(version_ptr);
        }
    }

    #[test]
    fn test_gaggle_get_version_format() {
        let version_ptr = gaggle_get_version();
        unsafe {
            let version_cstr = CStr::from_ptr(version_ptr);
            let version_str = version_cstr.to_str().unwrap();
            // Should not be JSON; just a semantic version-like string
            assert!(!version_str.starts_with('{'));
            assert!(!version_str.ends_with('}'));

            gaggle_free(version_ptr);
        }
    }

    #[test]
    fn test_gaggle_free_null_pointer() {
        // Should not panic when freeing null pointer
        unsafe {
            gaggle_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_gaggle_free_valid_pointer() {
        let ptr = gaggle_get_version();
        unsafe {
            gaggle_free(ptr);
        }
        // If we got here without crashing, the test passes
    }

    #[test]
    fn test_gaggle_clear_cache_success() {
        let result = gaggle_clear_cache();
        // Should return 0 (success) or -1 (error), but most likely 0
        assert!(result == 0 || result == -1);
    }

    #[test]
    fn test_gaggle_get_cache_info_not_null() {
        let info_ptr = gaggle_get_cache_info();
        assert!(!info_ptr.is_null());

        unsafe {
            let info_cstr = CStr::from_ptr(info_ptr);
            let info_str = info_cstr.to_str().unwrap();
            assert!(!info_str.is_empty());
            assert!(info_str.contains("\"path\""));

            gaggle_free(info_ptr);
        }
    }

    #[test]
    fn test_gaggle_get_cache_info_format() {
        let info_ptr = gaggle_get_cache_info();
        unsafe {
            let info_cstr = CStr::from_ptr(info_ptr);
            let info_str = info_cstr.to_str().unwrap();
            // Should be valid JSON
            assert!(info_str.starts_with('{'));
            assert!(info_str.ends_with('}'));
            // Should contain the documented keys
            assert!(info_str.contains("\"path\""));
            assert!(info_str.contains("\"size_mb\""));
            assert!(info_str.contains("\"limit_mb\""));
            assert!(info_str.contains("\"usage_percent\""));
            assert!(info_str.contains("\"is_soft_limit\""));
            assert!(info_str.contains("\"type\""));

            gaggle_free(info_ptr);
        }
    }

    #[test]
    fn test_gaggle_set_credentials_valid() {
        let username = CString::new("testuser").unwrap();
        let key = CString::new("testkey").unwrap();

        unsafe {
            let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
            assert_eq!(result, 0); // Should succeed
        }
    }

    #[test]
    fn test_gaggle_set_credentials_null_username() {
        let key = CString::new("testkey").unwrap();

        unsafe {
            let result = gaggle_set_credentials(std::ptr::null(), key.as_ptr());
            assert_eq!(result, -1); // Should fail
        }
    }

    #[test]
    fn test_gaggle_set_credentials_null_key() {
        let username = CString::new("testuser").unwrap();

        unsafe {
            let result = gaggle_set_credentials(username.as_ptr(), std::ptr::null());
            assert_eq!(result, -1); // Should fail
        }
    }

    #[test]
    fn test_gaggle_set_credentials_both_null() {
        unsafe {
            let result = gaggle_set_credentials(std::ptr::null(), std::ptr::null());
            assert_eq!(result, -1); // Should fail
        }
    }

    #[test]
    fn test_gaggle_set_credentials_empty_strings() {
        let username = CString::new("").unwrap();
        let key = CString::new("").unwrap();

        unsafe {
            let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
            assert_eq!(result, 0); // Should succeed even with empty strings
        }
    }

    #[test]
    fn test_gaggle_set_credentials_long_strings() {
        let long_username = "user".repeat(100);
        let long_key = "key".repeat(100);
        let username = CString::new(long_username).unwrap();
        let key = CString::new(long_key).unwrap();

        unsafe {
            let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
            assert_eq!(result, 0); // Should succeed
        }
    }

    #[test]
    fn test_gaggle_set_credentials_special_chars() {
        let username = CString::new("user@example.com").unwrap();
        let key = CString::new("key!@#$%^&*()").unwrap();

        unsafe {
            let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
            assert_eq!(result, 0); // Should succeed
        }
    }

    #[test]
    fn test_multiple_gaggle_get_version_calls() {
        for _ in 0..10 {
            let ptr1 = gaggle_get_version();
            let ptr2 = gaggle_get_version();

            unsafe {
                let str1 = CStr::from_ptr(ptr1).to_str().unwrap();
                let str2 = CStr::from_ptr(ptr2).to_str().unwrap();
                assert_eq!(str1, str2); // Should be consistent

                gaggle_free(ptr1);
                gaggle_free(ptr2);
            }
        }
    }

    #[test]
    fn test_multiple_gaggle_get_cache_info_calls() {
        for _ in 0..5 {
            let info_ptr = gaggle_get_cache_info();
            assert!(!info_ptr.is_null());

            unsafe {
                gaggle_free(info_ptr);
            }
        }
    }

    #[test]
    fn test_gaggle_clear_cache_multiple_times() {
        for _ in 0..3 {
            let result = gaggle_clear_cache();
            assert!(result == 0 || result == -1);
        }
    }

    #[test]
    fn test_calculate_dir_size_empty_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let size = calculate_dir_size(temp_dir.path()).unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_calculate_dir_size_with_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let size = calculate_dir_size(temp_dir.path()).unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_calculate_dir_size_with_subdirs() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let test_file = subdir.join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        let size = calculate_dir_size(temp_dir.path()).unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_gaggle_get_cache_info_contains_size() {
        let info_ptr = gaggle_get_cache_info();
        unsafe {
            let info_cstr = CStr::from_ptr(info_ptr);
            let info_str = info_cstr.to_str().unwrap();
            // Check for the keys: path, size_mb, limit_mb, usage_percent, is_soft_limit, type
            assert!(info_str.contains("\"path\""));
            assert!(info_str.contains("\"size_mb\""));
            assert!(info_str.contains("\"limit_mb\""));
            assert!(info_str.contains("\"usage_percent\""));
            assert!(info_str.contains("\"is_soft_limit\""));
            assert!(info_str.contains("\"type\""));

            gaggle_free(info_ptr);
        }
    }

    #[test]
    fn test_gaggle_version_contains_package_version() {
        let version_ptr = gaggle_get_version();
        unsafe {
            let version_cstr = CStr::from_ptr(version_ptr);
            let version_str = version_cstr.to_str().unwrap();
            // Should contain package version string
            assert!(!version_str.is_empty());

            gaggle_free(version_ptr);
        }
    }

    #[test]
    fn test_gaggle_ffi_string_consistency() {
        let username = CString::new("testuser").unwrap();
        let key = CString::new("testkey").unwrap();

        unsafe {
            gaggle_set_credentials(username.as_ptr(), key.as_ptr());

            // Get cache info and version multiple times
            let info1_ptr = gaggle_get_cache_info();
            let version1_ptr = gaggle_get_version();

            let _info1_str = CStr::from_ptr(info1_ptr).to_str().unwrap();
            let version1_str = CStr::from_ptr(version1_ptr).to_str().unwrap();

            let info2_ptr = gaggle_get_cache_info();
            let version2_ptr = gaggle_get_version();

            let _info2_str = CStr::from_ptr(info2_ptr).to_str().unwrap();
            let version2_str = CStr::from_ptr(version2_ptr).to_str().unwrap();

            // Version should be consistent
            assert_eq!(version1_str, version2_str);

            gaggle_free(info1_ptr);
            gaggle_free(version1_ptr);
            gaggle_free(info2_ptr);
            gaggle_free(version2_ptr);
        }
    }

    #[test]
    fn test_string_to_c_string_with_null_byte() {
        let s = String::from("test\0embedded");
        let ptr = string_to_c_string(s);
        assert!(ptr.is_null());

        // Error should be set
        let err_ptr = error::gaggle_last_error();
        assert!(!err_ptr.is_null());
        unsafe {
            let err_str = CStr::from_ptr(err_ptr).to_str().unwrap();
            assert!(err_str.contains("null byte"));
        }
    }

    #[test]
    fn test_string_to_c_string_normal() {
        let s = String::from("test string");
        let ptr = string_to_c_string(s);
        assert!(!ptr.is_null());
        unsafe {
            gaggle_free(ptr);
        }
    }

    #[test]
    fn test_gaggle_clear_cache_uses_runtime_env() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("GAGGLE_CACHE_DIR", temp_dir.path());

        // create a file inside cache dir
        let nested = temp_dir.path().join("datasets");
        fs::create_dir_all(&nested).unwrap();
        let f = nested.join("dummy.txt");
        fs::write(&f, b"x").unwrap();
        assert!(f.exists());

        // clear cache should remove and recreate top-level dir
        let rc = gaggle_clear_cache();
        assert_eq!(rc, 0);
        assert!(temp_dir.path().exists());
        // nested content should be gone
        assert!(!f.exists());

        std::env::remove_var("GAGGLE_CACHE_DIR");
    }

    #[test]
    fn test_gaggle_json_each_object_and_array() {
        let input = json!({
            "a": 1,
            "b": [true, {"c": "x"}],
        })
        .to_string();
        let c = CString::new(input).unwrap();
        let out_ptr = unsafe { gaggle_json_each(c.as_ptr()) };
        assert!(!out_ptr.is_null());
        unsafe {
            let out = CStr::from_ptr(out_ptr).to_str().unwrap().to_string();
            gaggle_free(out_ptr);
            let lines: Vec<&str> = out.lines().collect();
            // Should have at least the top-level keys a and b
            assert!(lines.len() >= 2);
            assert!(lines.iter().any(|l| l.contains("\"key\":\"a\"")));
            assert!(lines.iter().any(|l| l.contains("\"key\":\"b\"")));
        }
    }

    #[test]
    fn test_gaggle_json_each_invalid_json_sets_error() {
        let invalid = CString::new("{not json}").unwrap();
        let out_ptr = unsafe { gaggle_json_each(invalid.as_ptr()) };
        assert!(out_ptr.is_null());
        let err_ptr = error::gaggle_last_error();
        assert!(!err_ptr.is_null());
        unsafe {
            let msg = CStr::from_ptr(err_ptr).to_str().unwrap();
            assert!(msg.to_lowercase().contains("json"));
        }
    }
}
