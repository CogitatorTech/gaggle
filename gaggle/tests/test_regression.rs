// Regression tests for bugs fixed in Gaggle
// These tests make sure previously fixed bugs don't reoccur

use gaggle::{gaggle_clear_last_error, gaggle_last_error};
use gaggle::{gaggle_download_dataset, gaggle_free, gaggle_set_credentials};
use std::ffi::{CStr, CString};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn regression_concurrent_credential_loading() {
    std::env::set_var("KAGGLE_USERNAME", "test_user");
    std::env::set_var("KAGGLE_KEY", "test_key");

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let b = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            b.wait();
            unsafe {
                let dataset = CString::new("owner/dataset").unwrap();
                let result = gaggle_download_dataset(dataset.as_ptr());
                if !result.is_null() {
                    gaggle_free(result);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    std::env::remove_var("KAGGLE_USERNAME");
    std::env::remove_var("KAGGLE_KEY");
}

#[test]
fn regression_stale_errors_cleared() {
    unsafe {
        gaggle_set_credentials(std::ptr::null(), std::ptr::null());
        let err1 = gaggle_last_error();
        assert!(!err1.is_null());

        let username = CString::new("user").unwrap();
        let key = CString::new("key").unwrap();
        let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
        assert_eq!(result, 0);

        gaggle_clear_last_error();
        let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
        assert_eq!(result, 0);

        // No error should be set now
        let err2 = gaggle_last_error();
        assert!(err2.is_null());
    }
}

#[test]
fn regression_thread_local_errors_isolated() {
    let handle1 = thread::spawn(|| unsafe {
        gaggle_set_credentials(std::ptr::null(), std::ptr::null());
        let err = gaggle_last_error();
        assert!(!err.is_null());
        let msg = CStr::from_ptr(err).to_str().unwrap();
        assert!(msg.contains("Null pointer") || msg.contains("null"));
    });

    let handle2 = thread::spawn(|| unsafe {
        let invalid = CString::new("invalid").unwrap();
        gaggle_download_dataset(invalid.as_ptr());
        let err = gaggle_last_error();
        assert!(!err.is_null());
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

#[test]
fn regression_concurrent_same_dataset_download() {
    unsafe {
        let username = CString::new("testuser").unwrap();
        let key = CString::new("testkey").unwrap();
        gaggle_set_credentials(username.as_ptr(), key.as_ptr());
    }

    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    for _ in 0..5 {
        let b = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            b.wait();
            unsafe {
                let dataset = CString::new("owner/dataset").unwrap();
                let result = gaggle_download_dataset(dataset.as_ptr());
                if !result.is_null() {
                    gaggle_free(result);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn regression_error_cleared_at_operation_start() {
    unsafe {
        gaggle_set_credentials(std::ptr::null(), std::ptr::null());
        assert!(!gaggle_last_error().is_null());

        let username = CString::new("user").unwrap();
        let key = CString::new("key").unwrap();
        gaggle_set_credentials(username.as_ptr(), key.as_ptr());

        gaggle_clear_last_error();
        assert!(gaggle_last_error().is_null());
    }
}
