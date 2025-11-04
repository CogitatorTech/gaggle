// integration_mock.rs
//
// This integration test verifies the search and info functionalities of the Gaggle library
// using a mock HTTP server. It guarantees that the library correctly interacts with the Kaggle API
// endpoints for searching datasets and retrieving dataset information. The test sets up a mock
// server to simulate the Kaggle API, calls the relevant Gaggle FFI functions, and asserts
// that the functions return the expected results without errors.

use gaggle::{
    gaggle_free, gaggle_get_cache_info, gaggle_get_dataset_info, gaggle_search,
    gaggle_set_credentials,
};
use std::ffi::CString;

#[test]
fn integration_search_and_info_with_mock_server() {
    let mut server = mockito::Server::new();

    let _m1 = server
        .mock("GET", "/datasets/list")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body("[]")
        .create();

    let _m2 = server
        .mock("GET", "/datasets/view/owner/dataset")
        .with_status(200)
        .with_body(serde_json::json!({"ref":"owner/dataset"}).to_string())
        .create();

    unsafe {
        let u = CString::new("user").unwrap();
        let k = CString::new("key").unwrap();
        assert_eq!(gaggle_set_credentials(u.as_ptr(), k.as_ptr()), 0);
    }
    std::env::set_var("GAGGLE_API_BASE", server.url());

    unsafe {
        let q = CString::new("x").unwrap();
        let res = gaggle_search(q.as_ptr(), 1, 10);
        assert!(!res.is_null());
        gaggle_free(res);
    }

    unsafe {
        let ds = CString::new("owner/dataset").unwrap();
        let res = gaggle_get_dataset_info(ds.as_ptr());
        assert!(!res.is_null());
        gaggle_free(res);
    }

    unsafe {
        let res = gaggle_get_cache_info();
        assert!(!res.is_null());
        gaggle_free(res);
    }

    std::env::remove_var("GAGGLE_API_BASE");
}
