use mockito::{Matcher, Server};
use std::env;
use std::ffi::{CStr, CString};
use std::io::Write;

fn make_zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, content) in files.iter() {
            if name.ends_with('/') {
                zip.add_directory(name.to_string(), options).unwrap();
            } else {
                zip.start_file(name.to_string(), options).unwrap();
                zip.write_all(content).unwrap();
            }
        }
        zip.finish().unwrap();
    }
    buf
}

#[test]
#[serial_test::serial]
fn test_search_datasets_with_mock() {
    gaggle::init_logging();
    // Arrange mock server and environment
    let mut server = Server::new();
    let server_url = server.url();
    env::set_var("GAGGLE_API_BASE", &server_url);

    // Set any non-empty credentials (code only checks presence)
    let user = CString::new("user").unwrap();
    let key = CString::new("key").unwrap();
    unsafe {
        let _ = gaggle::gaggle_set_credentials(user.as_ptr(), key.as_ptr());
    }

    let _m = server
        .mock("GET", "/datasets/list")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[{\"ref\":\"owner/dataset\",\"title\":\"T\"}]")
        .create();

    // Act via FFI
    let query = CString::new("hello world").unwrap();
    let ptr = unsafe { gaggle::gaggle_search(query.as_ptr(), 1, 10) };
    assert!(!ptr.is_null());
    unsafe {
        let s = CStr::from_ptr(ptr).to_str().unwrap().to_string();
        gaggle::gaggle_free(ptr);
        assert!(s.starts_with("["));
        assert!(s.contains("owner/dataset"));
    }

    // Cleanup
    env::remove_var("GAGGLE_API_BASE");
}

#[test]
#[serial_test::serial]
fn test_download_and_version_with_mock() {
    gaggle::init_logging();
    let temp = tempfile::TempDir::new().unwrap();
    env::set_var("GAGGLE_CACHE_DIR", temp.path());

    let mut server = Server::new();
    let server_url = server.url();
    env::set_var("GAGGLE_API_BASE", &server_url);

    // Set any non-empty credentials
    let user = CString::new("user").unwrap();
    let key = CString::new("key").unwrap();
    unsafe {
        let _ = gaggle::gaggle_set_credentials(user.as_ptr(), key.as_ptr());
    }

    // Mock metadata: current version number 7
    let _meta = server
        .mock("GET", "/datasets/view/owner/dataset")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{\"currentVersionNumber\":7}")
        .create();

    // Mock download: simple zip with a file "data.csv"
    let zip_bytes = make_zip_bytes(&[("data.csv", b"a,b\n1,2\n")]);
    let _dl = server
        .mock("GET", "/datasets/download/owner/dataset")
        .with_status(200)
        .with_header("content-type", "application/zip")
        .with_body(zip_bytes)
        .create();

    // Note: no direct GET here to avoid consuming the mock before the actual download.

    // Act: download
    let ds = CString::new("owner/dataset").unwrap();
    let local_ptr = unsafe { gaggle::gaggle_download_dataset(ds.as_ptr()) };
    if local_ptr.is_null() {
        // fetch and display last error for debugging
        let err_ptr = gaggle::gaggle_last_error();
        if !err_ptr.is_null() {
            let err = unsafe { CStr::from_ptr(err_ptr) };
            panic!("download failed: {}", err.to_str().unwrap());
        } else {
            panic!("download failed with null pointer and no error set");
        }
    }
    let local = unsafe {
        let s = CStr::from_ptr(local_ptr).to_str().unwrap().to_string();
        gaggle::gaggle_free(local_ptr);
        std::path::PathBuf::from(s)
    };

    // Assert: file exists
    let csv = local.join("data.csv");
    assert!(csv.exists());

    // Version info should see cached and current
    let info_ptr = unsafe { gaggle::gaggle_dataset_version_info(ds.as_ptr()) };
    assert!(!info_ptr.is_null());
    let info_json = unsafe {
        let s = CStr::from_ptr(info_ptr).to_str().unwrap().to_string();
        gaggle::gaggle_free(info_ptr);
        s
    };
    let v: serde_json::Value = serde_json::from_str(&info_json).unwrap();
    assert!(v["is_cached"].as_bool().unwrap());
    assert_eq!(v["latest_version"].as_str().unwrap(), "7");

    // Cleanup
    env::remove_var("GAGGLE_CACHE_DIR");
    env::remove_var("GAGGLE_API_BASE");
}
