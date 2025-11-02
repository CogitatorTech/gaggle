use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn duckdb_bin() -> Option<PathBuf> {
    let p = PathBuf::from("../../build/release/duckdb");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn duckdb_ext() -> Option<PathBuf> {
    let p = PathBuf::from("../../build/release/extension/gaggle/gaggle.duckdb_extension");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

#[test]
fn test_replacement_scan_csv_and_json() {
    let duckdb = match duckdb_bin() {
        Some(p) => p,
        None => {
            eprintln!("Skipping replacement scan test: duckdb binary not present");
            return;
        }
    };
    let ext = match duckdb_ext() {
        Some(p) => p,
        None => {
            eprintln!("Skipping replacement scan test: extension binary not present");
            return;
        }
    };

    // Prepare a fake cached dataset with marker file to avoid network/download
    let tmp = tempfile::TempDir::new().unwrap();
    std::env::set_var("GAGGLE_CACHE_DIR", tmp.path());

    let ds_dir = tmp.path().join("datasets").join("owner").join("ds");
    fs::create_dir_all(&ds_dir).unwrap();

    // Marker to short-circuit download
    fs::write(ds_dir.join(".downloaded"), b"{}").unwrap();

    // CSV file
    fs::write(ds_dir.join("data.csv"), b"a,b\n1,2\n").unwrap();

    // JSON Lines file
    fs::write(ds_dir.join("data.jsonl"), b"{\"a\":1}\n{\"a\":2}\n").unwrap();

    // Build SQL script
    let sql = format!(
        "load '{}';\nselect sum(a) as s from 'kaggle:owner/ds/data.csv';\nselect sum(a) as s from 'kaggle:owner/ds/data.jsonl';\n",
        ext.display()
    );

    // Run duckdb with SQL via stdin
    let output = Command::new(&duckdb)
        .env("GAGGLE_CACHE_DIR", tmp.path())
        .arg("-batch")
        .arg("-unsigned")
        .arg("-csv")
        .arg("-cmd")
        .arg(sql)
        .output()
        .expect("failed to run duckdb");

    assert!(
        output.status.success(),
        "DuckDB failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // In CSV output with -csv, results are printed one per line without headers
    // We expect two lines: 1 and 3
    let nums: Vec<i64> = stdout
        .lines()
        .filter_map(|l| l.trim().parse::<i64>().ok())
        .collect();

    assert!(nums.len() >= 2, "Unexpected output: {}", stdout);
    // First query: sum(a) from csv = 1
    assert_eq!(nums[0], 1);
    // Second query: sum(a) from jsonl = 3
    assert_eq!(nums[1], 3);

    // Cleanup env var
    std::env::remove_var("GAGGLE_CACHE_DIR");
}
