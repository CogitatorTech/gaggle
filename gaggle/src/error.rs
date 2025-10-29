// Contains the GaggleError enum and thread-local error handling logic.

use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::str::Utf8Error as StdUtf8Error;
use thiserror::Error;

/// Represents all possible errors that can occur within the Gaggle library.
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

