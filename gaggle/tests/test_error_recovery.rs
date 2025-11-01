use gaggle::{gaggle_clear_last_error, gaggle_last_error};
use gaggle::{gaggle_download_dataset, gaggle_search, gaggle_set_credentials};
use std::ffi::{CStr, CString};

#[test]
fn test_error_cleared_between_operations() {
    unsafe {
        let result = gaggle_set_credentials(std::ptr::null(), std::ptr::null());
        assert_eq!(result, -1);

        let err_ptr = gaggle_last_error();
        assert!(!err_ptr.is_null());

        gaggle_clear_last_error();

        let err_ptr = gaggle_last_error();
        assert!(err_ptr.is_null());

        let username = CString::new("testuser").unwrap();
        let key = CString::new("testkey").unwrap();
        let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
        assert_eq!(result, 0);

        let err_ptr = gaggle_last_error();
        assert!(err_ptr.is_null());
    }
}

#[test]
fn test_invalid_dataset_path_sets_error() {
    unsafe {
        let username = CString::new("testuser").unwrap();
        let key = CString::new("testkey").unwrap();
        gaggle_set_credentials(username.as_ptr(), key.as_ptr());

        let invalid_path = CString::new("nodatasetpath").unwrap();
        let result = gaggle_download_dataset(invalid_path.as_ptr());
        assert!(result.is_null());

        let err_ptr = gaggle_last_error();
        assert!(!err_ptr.is_null());
        let err_str = CStr::from_ptr(err_ptr).to_str().unwrap();
        assert!(err_str.contains("format") || err_str.contains("slash"));
    }
}

#[test]
fn test_search_invalid_parameters() {
    unsafe {
        let username = CString::new("testuser").unwrap();
        let key = CString::new("testkey").unwrap();
        gaggle_set_credentials(username.as_ptr(), key.as_ptr());

        let query = CString::new("test").unwrap();
        let result = gaggle_search(query.as_ptr(), -1, 10);
        assert!(result.is_null());

        let err_ptr = gaggle_last_error();
        assert!(!err_ptr.is_null());
    }
}

#[test]
fn test_operations_after_error_recovery() {
    unsafe {
        let username = CString::new("user").unwrap();
        let key = CString::new("key").unwrap();
        gaggle_set_credentials(username.as_ptr(), key.as_ptr());

        let result = gaggle_search(std::ptr::null(), 1, 10);
        assert!(result.is_null());

        gaggle_clear_last_error();

        let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
        assert_eq!(result, 0);
    }
}

#[test]
fn test_multiple_errors_in_sequence() {
    unsafe {
        gaggle_set_credentials(std::ptr::null(), std::ptr::null());
        let err1 = gaggle_last_error();
        assert!(!err1.is_null());

        gaggle_search(std::ptr::null(), 1, 10);
        let err2 = gaggle_last_error();
        assert!(!err2.is_null());

        let err1_str = CStr::from_ptr(err1).to_str().unwrap();
        let err2_str = CStr::from_ptr(err2).to_str().unwrap();
        assert!(!err1_str.is_empty());
        assert!(!err2_str.is_empty());
    }
}
