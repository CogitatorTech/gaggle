// offline_mode.rs
//
// This integration test verifies the behavior of the Gaggle library when operating in
// offline mode. The test verifies that dataset downloads fail as expected when the
// dataset is not already cached and that version information is handled correctly
// when the library cannot access the network. By setting the `GAGGLE_OFFLINE`
// environment variable, the test simulates a scenario with no internet connectivity
// and asserts that the library's FFI functions behave as designed in this context.

use std::ffi::CString;

#[test]
fn test_offline_download_fails_when_not_cached_and_version_unknown() {
    // Enable offline
    std::env::set_var("GAGGLE_OFFLINE", "1");

    // Point cache to an empty temp dir
    let temp = tempfile::TempDir::new().unwrap();
    std::env::set_var("GAGGLE_CACHE_DIR", temp.path());

    // Minimal credentials (won't be used in offline)
    let user = CString::new("user").unwrap();
    let key = CString::new("key").unwrap();
    unsafe {
        let _ = gaggle::gaggle_set_credentials(user.as_ptr(), key.as_ptr());
    }

    // Download should fail fast because not cached
    let ds = CString::new("owner/dataset").unwrap();
    let local_ptr = unsafe { gaggle::gaggle_download_dataset(ds.as_ptr()) };
    assert!(local_ptr.is_null());

    // Version should be "unknown" in offline mode without cache
    let version_info_ptr = unsafe { gaggle::gaggle_dataset_version_info(ds.as_ptr()) };
    assert!(!version_info_ptr.is_null());
    let info = unsafe {
        let s = std::ffi::CStr::from_ptr(version_info_ptr)
            .to_str()
            .unwrap()
            .to_string();
        gaggle::gaggle_free(version_info_ptr);
        s
    };
    let v: serde_json::Value = serde_json::from_str(&info).unwrap();
    // latest_version is "unknown" in offline mode when uncached
    assert_eq!(v["latest_version"], "unknown");
    assert!(!v["is_cached"].as_bool().unwrap());

    // Cleanup
    std::env::remove_var("GAGGLE_OFFLINE");
    std::env::remove_var("GAGGLE_CACHE_DIR");
}
