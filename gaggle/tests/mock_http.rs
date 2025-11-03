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

#[test]
#[serial_test::serial]
fn test_single_file_fetch_on_demand() {
    gaggle::init_logging();
    let temp = tempfile::TempDir::new().unwrap();
    env::set_var("GAGGLE_CACHE_DIR", temp.path());

    let mut server = Server::new();
    let server_url = server.url();
    env::set_var("GAGGLE_API_BASE", &server_url);

    // Set credentials
    let user = CString::new("user").unwrap();
    let key = CString::new("key").unwrap();
    unsafe {
        let _ = gaggle::gaggle_set_credentials(user.as_ptr(), key.as_ptr());
    }

    // Mock single-file endpoint
    let _file = server
        .mock("GET", "/datasets/download/owner/dataset")
        .match_query(Matcher::UrlEncoded("fileName".into(), "data.csv".into()))
        .with_status(200)
        .with_header("content-type", "text/csv")
        .with_body("a,b\n1,2\n")
        .create();

    // Act: request file path; should trigger on-demand fetch
    let ds = CString::new("owner/dataset").unwrap();
    let fnm = CString::new("data.csv").unwrap();
    let ptr = unsafe { gaggle::gaggle_get_file_path(ds.as_ptr(), fnm.as_ptr()) };
    assert!(!ptr.is_null());
    let path = unsafe {
        let s = CStr::from_ptr(ptr).to_str().unwrap().to_string();
        gaggle::gaggle_free(ptr);
        std::path::PathBuf::from(s)
    };
    assert!(path.exists());

    // Ensure that full dataset extraction marker is not required for single-file presence
    let ds_dir = temp.path().join("datasets/owner/dataset");
    assert!(ds_dir.join("data.csv").exists());
    // .downloaded marker may not exist yet (partial cache is allowed)

    env::remove_var("GAGGLE_CACHE_DIR");
    env::remove_var("GAGGLE_API_BASE");
}

#[test]
#[serial_test::serial]
fn test_strict_on_demand_no_fallback() {
    gaggle::init_logging();
    let temp = tempfile::TempDir::new().unwrap();
    env::set_var("GAGGLE_CACHE_DIR", temp.path());
    env::set_var("GAGGLE_STRICT_ONDEMAND", "1");

    let mut server = Server::new();
    let server_url = server.url();
    env::set_var("GAGGLE_API_BASE", &server_url);

    // Set credentials
    let user = CString::new("user").unwrap();
    let key = CString::new("key").unwrap();
    unsafe {
        let _ = gaggle::gaggle_set_credentials(user.as_ptr(), key.as_ptr());
    }

    // Mock single-file endpoint to return 404 (force failure)
    let _file = server
        .mock("GET", "/datasets/download/owner/dataset")
        .match_query(Matcher::UrlEncoded("fileName".into(), "missing.csv".into()))
        .with_status(404)
        .with_header("content-type", "text/plain")
        .with_body("not found")
        .create();

    // Act: request file path; should fail and not fall back to full download
    let ds = CString::new("owner/dataset").unwrap();
    let fnm = CString::new("missing.csv").unwrap();
    let ptr = unsafe { gaggle::gaggle_get_file_path(ds.as_ptr(), fnm.as_ptr()) };
    assert!(ptr.is_null());
    let err_ptr = gaggle::gaggle_last_error();
    assert!(!err_ptr.is_null());
    let err = unsafe { CStr::from_ptr(err_ptr) }
        .to_str()
        .unwrap()
        .to_lowercase();
    assert!(err.contains("http"));

    env::remove_var("GAGGLE_CACHE_DIR");
    env::remove_var("GAGGLE_STRICT_ONDEMAND");
    env::remove_var("GAGGLE_API_BASE");
}

#[test]
#[serial_test::serial]
fn test_prefetch_files_mixed_results() {
    gaggle::init_logging();
    let temp = tempfile::TempDir::new().unwrap();
    env::set_var("GAGGLE_CACHE_DIR", temp.path());
    env::set_var("GAGGLE_STRICT_ONDEMAND", "1");

    let mut server = Server::new();
    let server_url = server.url();
    env::set_var("GAGGLE_API_BASE", &server_url);

    // Set credentials
    let user = CString::new("user").unwrap();
    let key = CString::new("key").unwrap();
    unsafe {
        let _ = gaggle::gaggle_set_credentials(user.as_ptr(), key.as_ptr());
    }

    // Mock good file
    let _good = server
        .mock("GET", "/datasets/download/owner/dataset")
        .match_query(Matcher::UrlEncoded("fileName".into(), "good.csv".into()))
        .with_status(200)
        .with_header("content-type", "text/csv")
        .with_body("x\n1\n")
        .create();

    // Mock missing file
    let _bad = server
        .mock("GET", "/datasets/download/owner/dataset")
        .match_query(Matcher::UrlEncoded("fileName".into(), "bad.csv".into()))
        .with_status(404)
        .with_body("not found")
        .create();

    // Call prefetch
    let ds = CString::new("owner/dataset").unwrap();
    let list = CString::new("good.csv\nbad.csv").unwrap();
    let ptr = unsafe { gaggle::gaggle_prefetch_files(ds.as_ptr(), list.as_ptr()) };
    assert!(!ptr.is_null());
    let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
    unsafe { gaggle::gaggle_free(ptr) };

    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["dataset"].as_str().unwrap(), "owner/dataset");
    let files = v["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);

    // Find statuses
    let mut ok_seen = false;
    let mut err_seen = false;
    for f in files {
        let name = f["name"].as_str().unwrap();
        let status = f["status"].as_str().unwrap();
        if name == "good.csv" {
            assert_eq!(status, "ok");
            assert!(f["path"].as_str().unwrap().ends_with("good.csv"));
            ok_seen = true;
        }
        if name == "bad.csv" {
            assert_eq!(status, "error");
            assert!(!f["error"].as_str().unwrap().is_empty());
            err_seen = true;
        }
    }
    assert!(ok_seen && err_seen);

    env::remove_var("GAGGLE_CACHE_DIR");
    env::remove_var("GAGGLE_STRICT_ONDEMAND");
    env::remove_var("GAGGLE_API_BASE");
}
