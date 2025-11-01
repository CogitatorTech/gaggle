use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::str::Utf8Error as StdUtf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum GaggleError {
    /// Error indicating that a requested dataset could not be found.
    #[error("Dataset not found: {0}")]
    DatasetNotFound(String),
    /// Error for when a C string from the FFI boundary is not valid UTF-8.
    #[error("Invalid UTF-8 string")]
    Utf8Error,
    /// Error for when a null pointer is passed as an argument to an FFI function.
    #[error("Null pointer passed")]
    NullPointer,
    /// An I/O error that occurred while reading/writing files.
    #[error("IO error: {0}")]
    IoError(String),
    /// An error during the serialization or deserialization of JSON data.
    #[error("JSON serialization error: {0}")]
    JsonError(String),
    /// An error that occurred during an HTTP request to Kaggle API.
    #[error("HTTP request failed: {0}")]
    HttpRequestError(String),
    /// Error for invalid Kaggle API credentials.
    #[error("Invalid Kaggle credentials: {0}")]
    CredentialsError(String),
    /// Error for invalid dataset path format.
    #[error("Invalid dataset path: {0}")]
    InvalidDatasetPath(String),
    /// Error during ZIP extraction.
    #[error("ZIP extraction failed: {0}")]
    ZipError(String),
    /// Error during CSV parsing.
    #[error("CSV parsing error: {0}")]
    CsvError(String),
}

impl From<StdUtf8Error> for GaggleError {
    fn from(_: StdUtf8Error) -> Self {
        GaggleError::Utf8Error
    }
}

impl From<std::io::Error> for GaggleError {
    fn from(err: std::io::Error) -> Self {
        GaggleError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for GaggleError {
    fn from(err: serde_json::Error) -> Self {
        GaggleError::JsonError(err.to_string())
    }
}

impl From<reqwest::Error> for GaggleError {
    fn from(err: reqwest::Error) -> Self {
        GaggleError::HttpRequestError(err.to_string())
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Sets the last error for the current thread.
///
/// This stores the given error in a thread-local variable so it can be retrieved
/// later by FFI clients using `gaggle_last_error`.
pub(crate) fn set_last_error(err: &GaggleError) {
    if let Ok(c_string) = CString::new(err.to_string()) {
        LAST_ERROR.with(|cell| {
            *cell.borrow_mut() = Some(c_string);
        });
    }
}

/// Internal function to clear the last error (callable from Rust code)
pub(crate) fn clear_last_error_internal() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Retrieves the last error message set in the current thread.
///
/// After an FFI function returns an error code, this function can be called
/// to get a more descriptive, human-readable error message.
///
/// # Returns
///
/// A pointer to a null-terminated C string containing the last error message.
/// Returns a null pointer if no error has occurred since the last call.
/// The caller **must not** free this pointer, as it is managed by a thread-local static variable.
#[no_mangle]
pub extern "C" fn gaggle_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| match *cell.borrow() {
        Some(ref c_string) => c_string.as_ptr(),
        None => std::ptr::null(),
    })
}

/// Clears the last error for the current thread.
///
/// This is useful for ensuring that old error messages don't persist
/// and get confused with new errors.
#[no_mangle]
pub extern "C" fn gaggle_clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_dataset_not_found_error() {
        let err = GaggleError::DatasetNotFound("test/dataset".to_string());
        assert_eq!(err.to_string(), "Dataset not found: test/dataset");
    }

    #[test]
    fn test_utf8_error() {
        let err = GaggleError::Utf8Error;
        assert_eq!(err.to_string(), "Invalid UTF-8 string");
    }

    #[test]
    fn test_null_pointer_error() {
        let err = GaggleError::NullPointer;
        assert_eq!(err.to_string(), "Null pointer passed");
    }

    #[test]
    fn test_io_error() {
        let err = GaggleError::IoError("file not found".to_string());
        assert_eq!(err.to_string(), "IO error: file not found");
    }

    #[test]
    fn test_json_error() {
        let err = GaggleError::JsonError("invalid json".to_string());
        assert_eq!(err.to_string(), "JSON serialization error: invalid json");
    }

    #[test]
    fn test_http_request_error() {
        let err = GaggleError::HttpRequestError("connection timeout".to_string());
        assert_eq!(err.to_string(), "HTTP request failed: connection timeout");
    }

    #[test]
    fn test_credentials_error() {
        let err = GaggleError::CredentialsError("invalid credentials".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid Kaggle credentials: invalid credentials"
        );
    }

    #[test]
    fn test_invalid_dataset_path_error() {
        let err = GaggleError::InvalidDatasetPath("bad/path/format".to_string());
        assert_eq!(err.to_string(), "Invalid dataset path: bad/path/format");
    }

    #[test]
    fn test_zip_error() {
        let err = GaggleError::ZipError("corrupted zip file".to_string());
        assert_eq!(err.to_string(), "ZIP extraction failed: corrupted zip file");
    }

    #[test]
    fn test_csv_error() {
        let err = GaggleError::CsvError("invalid csv format".to_string());
        assert_eq!(err.to_string(), "CSV parsing error: invalid csv format");
    }

    #[test]
    fn test_clear_last_error() {
        use super::*;

        // Set an error
        let err = GaggleError::NullPointer;
        set_last_error(&err);

        // Verify it's set
        let err_ptr = gaggle_last_error();
        assert!(!err_ptr.is_null());

        // Clear it
        gaggle_clear_last_error();

        // Verify it's cleared
        let err_ptr = gaggle_last_error();
        assert!(err_ptr.is_null());
    }

    #[test]
    fn test_clear_last_error_when_none_set() {
        // Clearing when no error is set should not panic
        gaggle_clear_last_error();
        let err_ptr = gaggle_last_error();
        assert!(err_ptr.is_null());
    }

    #[test]
    fn test_error_cleared_after_multiple_sets() {
        use super::*;

        // Set multiple errors
        set_last_error(&GaggleError::NullPointer);
        set_last_error(&GaggleError::Utf8Error);
        set_last_error(&GaggleError::IoError("test".to_string()));

        // Clear
        gaggle_clear_last_error();

        // Should be null
        assert!(gaggle_last_error().is_null());
    }

    #[test]
    fn test_from_utf8_error() {
        let invalid_utf8 = vec![0xff, 0xfe];
        let utf8_result = std::str::from_utf8(&invalid_utf8);
        assert!(utf8_result.is_err());

        let err: GaggleError = utf8_result.unwrap_err().into();
        assert_eq!(err.to_string(), "Invalid UTF-8 string");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: GaggleError = io_err.into();
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_from_json_error() {
        let json_result: Result<serde_json::Value, _> = serde_json::from_str("{invalid}");
        assert!(json_result.is_err());

        let err: GaggleError = json_result.unwrap_err().into();
        assert!(err.to_string().contains("JSON serialization error"));
    }

    #[test]
    fn test_set_last_error() {
        let err = GaggleError::DatasetNotFound("test".to_string());
        set_last_error(&err);

        let error_ptr = gaggle_last_error();
        assert!(!error_ptr.is_null());

        unsafe {
            let error_cstr = CStr::from_ptr(error_ptr);
            let error_msg = error_cstr.to_str().unwrap();
            assert!(error_msg.contains("Dataset not found"));
        }
    }

    #[test]
    fn test_last_error_null_initially() {
        // Clear previous errors by setting and retrieving
        let err = GaggleError::IoError("test".to_string());
        set_last_error(&err);
        gaggle_last_error();

        // New thread should have no error initially
        let handle = std::thread::spawn(|| gaggle_last_error().is_null());
        assert!(handle.join().unwrap());
    }

    #[test]
    fn test_error_display_formats() {
        let errors = vec![
            GaggleError::DatasetNotFound("owner/dataset".to_string()),
            GaggleError::Utf8Error,
            GaggleError::NullPointer,
            GaggleError::IoError("read error".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_credentials_error_empty_message() {
        let err = GaggleError::CredentialsError(String::new());
        assert_eq!(err.to_string(), "Invalid Kaggle credentials: ");
    }

    #[test]
    fn test_invalid_dataset_path_with_special_chars() {
        let err = GaggleError::InvalidDatasetPath("user@host/dataset#123".to_string());
        assert!(err.to_string().contains("user@host/dataset#123"));
    }

    #[test]
    fn test_http_error_with_status_code() {
        let err = GaggleError::HttpRequestError("HTTP 404: Not Found".to_string());
        assert!(err.to_string().contains("404"));
    }
}
