use gaggle::parse_dataset_path;

#[test]
fn test_path_traversal_attempts_rejected() {
    let attacks = vec![
        "../../../etc/passwd",
        "owner/../dataset",
        "owner/../../etc",
        "./owner/dataset",
        "owner/./dataset",
        "owner/..",
        "../owner/dataset",
    ];

    for attack in attacks {
        let result = parse_dataset_path(attack);
        if let Ok((owner, dataset)) = result {
            assert!(!owner.contains(".."));
            assert!(!dataset.contains(".."));
        }
    }
}

#[test]
fn test_null_byte_injection_rejected() {
    use std::ffi::CString;

    let attempt = "owner/dataset\0malicious";
    let result = CString::new(attempt);
    assert!(result.is_err());
}

#[test]
fn test_special_characters_in_dataset_path() {
    let paths = vec![
        ("owner-name", "dataset-name"),
        ("owner_name", "dataset_name"),
        ("owner.name", "dataset.name"),
        ("owner123", "dataset456"),
    ];

    for (expected_owner, expected_dataset) in paths {
        let path = format!("{}/{}", expected_owner, expected_dataset);
        let result = parse_dataset_path(&path);
        assert!(result.is_ok(), "Failed to parse valid path: {}", path);
        let (owner, dataset) = result.unwrap();
        assert_eq!(owner, expected_owner);
        assert_eq!(dataset, expected_dataset);
    }
}

#[test]
fn test_overly_long_dataset_paths() {
    let long_owner = "a".repeat(1000);
    let long_dataset = "b".repeat(1000);
    let path = format!("{}/{}", long_owner, long_dataset);

    let result = parse_dataset_path(&path);
    assert!(result.is_ok());
    let (owner, dataset) = result.unwrap();
    assert_eq!(owner.len(), 1000);
    assert_eq!(dataset.len(), 1000);
}

#[test]
fn test_unicode_dataset_paths() {
    let paths = vec![
        ("用户", "数据集"),
        ("użytkownik", "zbiór"),
        ("utilisateur", "données"),
        ("пользователь", "данные"),
    ];

    for (owner, dataset) in paths {
        let path = format!("{}/{}", owner, dataset);
        let result = parse_dataset_path(&path);
        assert!(result.is_ok(), "Failed to parse Unicode path: {}", path);
        let (parsed_owner, parsed_dataset) = result.unwrap();
        assert_eq!(parsed_owner, owner);
        assert_eq!(parsed_dataset, dataset);
    }
}

#[test]
fn test_dataset_path_with_control_characters() {
    let paths_with_control = vec![
        "owner/dataset\n",
        "owner/dataset\r",
        "owner/dataset\t",
        "owner\n/dataset",
    ];

    for path in paths_with_control {
        let result = parse_dataset_path(path);
        if let Ok((owner, dataset)) = result {
            assert!(!owner.is_empty() || !dataset.is_empty());
        }
    }
}

use gaggle::{gaggle_free, gaggle_get_cache_info, gaggle_set_credentials};
use std::ffi::CString;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_concurrent_credential_setting() {
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for i in 0..10 {
        let b = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            let username = CString::new(format!("user{}", i)).unwrap();
            let key = CString::new(format!("key{}", i)).unwrap();

            b.wait();
            unsafe {
                let result = gaggle_set_credentials(username.as_ptr(), key.as_ptr());
                assert!(result == 0 || result == -1);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_cache_info_access() {
    let mut handles = vec![];

    for _ in 0..20 {
        let handle = thread::spawn(|| unsafe {
            let info_ptr = gaggle_get_cache_info();
            assert!(!info_ptr.is_null());

            let info_cstr = std::ffi::CStr::from_ptr(info_ptr);
            let info_str = info_cstr.to_str().unwrap();
            assert!(!info_str.is_empty());

            gaggle_free(info_ptr);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_credential_setting_with_cache_access() {
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = vec![];

    for i in 0..2 {
        let b = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            let username = CString::new(format!("user{}", i)).unwrap();
            let key = CString::new(format!("key{}", i)).unwrap();

            b.wait();
            unsafe {
                gaggle_set_credentials(username.as_ptr(), key.as_ptr());
            }
        });
        handles.push(handle);
    }

    for _ in 0..2 {
        let b = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            b.wait();
            unsafe {
                let info_ptr = gaggle_get_cache_info();
                assert!(!info_ptr.is_null());
                gaggle_free(info_ptr);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_get_file_path_rejects_absolute_filename() {
    unsafe {
        let u = CString::new("user").unwrap();
        let k = CString::new("key").unwrap();
        assert_eq!(gaggle_set_credentials(u.as_ptr(), k.as_ptr()), 0);
        let ds = CString::new("owner/ds").unwrap();
        let filename = CString::new("/etc/passwd").unwrap();
        let ptr = gaggle::gaggle_get_file_path(ds.as_ptr(), filename.as_ptr());
        assert!(ptr.is_null());
        let err_ptr = gaggle::gaggle_last_error();
        assert!(!err_ptr.is_null());
        let msg = std::ffi::CStr::from_ptr(err_ptr).to_str().unwrap();
        assert!(msg.to_lowercase().contains("absolute"));
    }
}

#[test]
fn test_get_file_path_rejects_parent_components() {
    unsafe {
        let u = CString::new("user").unwrap();
        let k = CString::new("key").unwrap();
        assert_eq!(gaggle_set_credentials(u.as_ptr(), k.as_ptr()), 0);
        let ds = CString::new("owner/ds").unwrap();
        let filename = CString::new("../secrets.csv").unwrap();
        let ptr = gaggle::gaggle_get_file_path(ds.as_ptr(), filename.as_ptr());
        assert!(ptr.is_null());
        let err_ptr = gaggle::gaggle_last_error();
        assert!(!err_ptr.is_null());
        let msg = std::ffi::CStr::from_ptr(err_ptr).to_str().unwrap();
        assert!(msg.to_lowercase().contains("parent") || msg.to_lowercase().contains("root"));
    }
}
