// Rust integration-style tests using test-only overrides and mock HTTP server

use gaggle::{
    gaggle_free, gaggle_get_cache_info, gaggle_get_dataset_info, gaggle_search,
    gaggle_set_credentials,
};
use std::ffi::CString;

#[test]
fn integration_search_and_info_with_mock_server() {
    // Start a mock server
    let mut server = mockito::Server::new();

    // /datasets/list returns empty array
    let _m1 = server
        .mock("GET", "/datasets/list")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body("[]")
        .create();

    // /datasets/view/owner/dataset returns a tiny json
    let _m2 = server
        .mock("GET", "/datasets/view/owner/dataset")
        .with_status(200)
        .with_body(serde_json::json!({"ref":"owner/dataset"}).to_string())
        .create();

    // Configure credentials and API base
    unsafe {
        let u = CString::new("user").unwrap();
        let k = CString::new("key").unwrap();
        assert_eq!(gaggle_set_credentials(u.as_ptr(), k.as_ptr()), 0);
    }
    std::env::set_var("GAGGLE_API_BASE", server.url());

    // Call search
    unsafe {
        let q = CString::new("x").unwrap();
        let res = gaggle_search(q.as_ptr(), 1, 10);
        assert!(!res.is_null());
        gaggle_free(res);
    }

    // Call info
    unsafe {
        let ds = CString::new("owner/dataset").unwrap();
        let res = gaggle_get_dataset_info(ds.as_ptr());
        assert!(!res.is_null());
        gaggle_free(res);
    }

    // Call cache info (no server use)
    unsafe {
        let res = gaggle_get_cache_info();
        assert!(!res.is_null());
        gaggle_free(res);
    }

    std::env::remove_var("GAGGLE_API_BASE");
}
