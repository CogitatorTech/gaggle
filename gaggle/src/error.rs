use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::str::Utf8Error as StdUtf8Error;
use thiserror::Error;

/// `ErrorCode` defines a set of specific error types for programmatic handling.
///
/// Each error code corresponds to a distinct category of issue that may arise
/// during the execution of Gaggle operations. These codes provide a stable,
/// machine-readable way to identify and react to errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ErrorCode {
    /// E001: Indicates invalid or missing Kaggle API credentials.
    E001_InvalidCredentials,
    /// E002: Represents that the requested dataset could not be found on Kaggle.
    E002_DatasetNotFound,
    /// E003: Signifies an HTTP or network error during an API request.
    E003_NetworkError,
    /// E004: Denotes an invalid format for a dataset path.
    E004_InvalidPath,
    /// E005: An error related to file system I/O operations.
    E005_IoError,
    /// E006: A failure in JSON serialization or deserialization.
    E006_JsonError,
    /// E007: An issue with ZIP file extraction or validation.
    E007_ZipError,
    /// E008: An error encountered while parsing a CSV file.
    E008_CsvError,
    /// E009: An invalid UTF-8 string was found at an FFI boundary.
    E009_Utf8Error,
    /// E010: A null pointer was passed to an FFI function.
    E010_NullPointer,
}

impl ErrorCode {
    /// Returns the numeric error code as a string slice.
    pub fn code(&self) -> &'static str {
        match self {
            ErrorCode::E001_InvalidCredentials => "E001",
            ErrorCode::E002_DatasetNotFound => "E002",
            ErrorCode::E003_NetworkError => "E003",
            ErrorCode::E004_InvalidPath => "E004",
            ErrorCode::E005_IoError => "E005",
            ErrorCode::E006_JsonError => "E006",
            ErrorCode::E007_ZipError => "E007",
            ErrorCode::E008_CsvError => "E008",
            ErrorCode::E009_Utf8Error => "E009",
            ErrorCode::E010_NullPointer => "E010",
        }
    }

    /// Returns a brief, human-readable description of the error.
    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::E001_InvalidCredentials => "Invalid Kaggle credentials",
            ErrorCode::E002_DatasetNotFound => "Dataset not found",
            ErrorCode::E003_NetworkError => "Network error",
            ErrorCode::E004_InvalidPath => "Invalid dataset path",
            ErrorCode::E005_IoError => "File system error",
            ErrorCode::E006_JsonError => "JSON error",
            ErrorCode::E007_ZipError => "ZIP extraction error",
            ErrorCode::E008_CsvError => "CSV parsing error",
            ErrorCode::E009_Utf8Error => "UTF-8 encoding error",
            ErrorCode::E010_NullPointer => "Null pointer error",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.description())
    }
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum GaggleError {
    /// Error indicating that a requested dataset could not be found.
    #[error("[E002] Dataset not found: {0}")]
    DatasetNotFound(String),
    /// Error for when a C string from the FFI boundary is not valid UTF-8.
    #[error("[E009] Invalid UTF-8 string")]
    Utf8Error,
    /// Error for when a null pointer is passed as an argument to an FFI function.
    #[error("[E010] Null pointer passed")]
    NullPointer,
    /// An I/O error that occurred while reading/writing files.
    #[error("[E005] IO error: {0}")]
    IoError(String),
    /// An error during the serialization or deserialization of JSON data.
    #[error("[E006] JSON serialization error: {0}")]
    JsonError(String),
    /// An error that occurred during an HTTP request to Kaggle API.
    #[error("[E003] HTTP request failed: {0}")]
    HttpRequestError(String),
    /// Error for invalid Kaggle API credentials.
    #[error("[E001] Invalid Kaggle credentials: {0}")]
    CredentialsError(String),
    /// Error for invalid dataset path format.
    #[error("[E004] Invalid dataset path: {0}")]
    InvalidDatasetPath(String),
    /// Error during ZIP extraction.
    #[error("[E007] ZIP extraction failed: {0}")]
    ZipError(String),
    /// Error during CSV parsing.
    #[error("[E008] CSV parsing error: {0}")]
    CsvError(String),
}

impl GaggleError {
    /// Get the error code for this error
    pub fn code(&self) -> ErrorCode {
        match self {
            GaggleError::DatasetNotFound(_) => ErrorCode::E002_DatasetNotFound,
            GaggleError::Utf8Error => ErrorCode::E009_Utf8Error,
            GaggleError::NullPointer => ErrorCode::E010_NullPointer,
            GaggleError::IoError(_) => ErrorCode::E005_IoError,
            GaggleError::JsonError(_) => ErrorCode::E006_JsonError,
            GaggleError::HttpRequestError(_) => ErrorCode::E003_NetworkError,
            GaggleError::CredentialsError(_) => ErrorCode::E001_InvalidCredentials,
            GaggleError::InvalidDatasetPath(_) => ErrorCode::E004_InvalidPath,
            GaggleError::ZipError(_) => ErrorCode::E007_ZipError,
            GaggleError::CsvError(_) => ErrorCode::E008_CsvError,
        }
    }

    /// Get the numeric error code as a string
    pub fn code_str(&self) -> &'static str {
        self.code().code()
    }
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
        let msg = err.to_string();
        assert!(msg.contains("[E002]"));
        assert!(msg.contains("test/dataset"));
    }

    #[test]
    fn test_utf8_error() {
        let err = GaggleError::Utf8Error;
        let msg = err.to_string();
        assert!(msg.contains("[E009]"));
        assert!(msg.contains("Invalid UTF-8"));
    }

    #[test]
    fn test_null_pointer_error() {
        let err = GaggleError::NullPointer;
        let msg = err.to_string();
        assert!(msg.contains("[E010]"));
        assert!(msg.contains("Null pointer"));
    }

    #[test]
    fn test_io_error() {
        let err = GaggleError::IoError("file not found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E005]"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_json_error() {
        let err = GaggleError::JsonError("invalid json".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E006]"));
        assert!(msg.contains("invalid json"));
    }

    #[test]
    fn test_error_code_enum() {
        let err = GaggleError::CredentialsError("test".to_string());
        assert_eq!(err.code(), ErrorCode::E001_InvalidCredentials);
        assert_eq!(err.code_str(), "E001");
    }

    #[test]
    fn test_error_code_display() {
        let code = ErrorCode::E001_InvalidCredentials;
        let display = format!("{}", code);
        assert!(display.contains("E001"));
        assert!(display.contains("Invalid Kaggle credentials"));
    }

    #[test]
    fn test_all_error_codes() {
        // Verify all error variants have correct codes
        assert_eq!(
            GaggleError::DatasetNotFound("".into()).code(),
            ErrorCode::E002_DatasetNotFound
        );
        assert_eq!(GaggleError::Utf8Error.code(), ErrorCode::E009_Utf8Error);
        assert_eq!(GaggleError::NullPointer.code(), ErrorCode::E010_NullPointer);
        assert_eq!(
            GaggleError::IoError("".into()).code(),
            ErrorCode::E005_IoError
        );
        assert_eq!(
            GaggleError::JsonError("".into()).code(),
            ErrorCode::E006_JsonError
        );
        assert_eq!(
            GaggleError::HttpRequestError("".into()).code(),
            ErrorCode::E003_NetworkError
        );
        assert_eq!(
            GaggleError::CredentialsError("".into()).code(),
            ErrorCode::E001_InvalidCredentials
        );
        assert_eq!(
            GaggleError::InvalidDatasetPath("".into()).code(),
            ErrorCode::E004_InvalidPath
        );
        assert_eq!(
            GaggleError::ZipError("".into()).code(),
            ErrorCode::E007_ZipError
        );
        assert_eq!(
            GaggleError::CsvError("".into()).code(),
            ErrorCode::E008_CsvError
        );
    }

    #[test]
    fn test_http_request_error() {
        let err = GaggleError::HttpRequestError("connection timeout".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E003]"));
        assert!(msg.contains("connection timeout"));
    }

    #[test]
    fn test_credentials_error() {
        let err = GaggleError::CredentialsError("invalid credentials".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E001]"));
        assert!(msg.contains("invalid credentials"));
    }

    #[test]
    fn test_invalid_dataset_path_error() {
        let err = GaggleError::InvalidDatasetPath("bad/path/format".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E004]"));
        assert!(msg.contains("bad/path/format"));
    }

    #[test]
    fn test_zip_error() {
        let err = GaggleError::ZipError("corrupted zip file".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E007]"));
        assert!(msg.contains("corrupted zip file"));
    }

    #[test]
    fn test_csv_error() {
        let err = GaggleError::CsvError("invalid csv format".to_string());
        let msg = err.to_string();
        assert!(msg.contains("[E008]"));
        assert!(msg.contains("invalid csv format"));
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
        let msg = err.to_string();
        assert!(msg.contains("[E009]"));
        assert!(msg.contains("Invalid UTF-8"));
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
        let msg = err.to_string();
        assert!(msg.contains("[E001]"));
        assert!(msg.contains("Invalid Kaggle credentials"));
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
