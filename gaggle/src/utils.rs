use std::fs;
use std::path::Path;

/// Recursively calculates the size of a directory in bytes.
///
/// This function traverses the directory tree from the given path and sums the
/// sizes of all files. It follows the same semantics as the previous inline
/// helpers in `ffi.rs` and `download.rs`.
pub fn calculate_dir_size(path: &Path) -> Result<u64, std::io::Error> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                total = total.saturating_add(calculate_dir_size(&entry.path())?);
            } else {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

/// Selects the appropriate DuckDB reader function based on the file extension.
///
/// The selection is case-insensitive.
#[allow(dead_code)]
pub fn guess_reader_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".parquet") || lower.ends_with(".parq") {
        "read_parquet"
    } else if lower.ends_with(".json") || lower.ends_with(".jsonl") || lower.ends_with(".ndjson") {
        "read_json_auto"
    } else if lower.ends_with(".xlsx") {
        "read_excel"
    } else {
        "read_csv_auto"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dir_size_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let size = calculate_dir_size(temp.path()).unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_calculate_dir_size_nested() {
        let temp = tempfile::TempDir::new().unwrap();
        let sub = temp.path().join("a/b");
        fs::create_dir_all(&sub).unwrap();
        let f1 = temp.path().join("a.txt");
        let f2 = sub.join("b.txt");
        fs::write(&f1, b"hello").unwrap();
        fs::write(&f2, b"world").unwrap();
        let size = calculate_dir_size(temp.path()).unwrap();
        assert!(size >= 10);
    }

    #[test]
    fn test_guess_reader_for_path_mapping() {
        assert_eq!(guess_reader_for_path("file.parquet"), "read_parquet");
        assert_eq!(guess_reader_for_path("file.PARQ"), "read_parquet");
        assert_eq!(guess_reader_for_path("file.json"), "read_json_auto");
        assert_eq!(guess_reader_for_path("file.JSONL"), "read_json_auto");
        assert_eq!(guess_reader_for_path("file.ndjson"), "read_json_auto");
        assert_eq!(guess_reader_for_path("file.xlsx"), "read_excel");
        assert_eq!(guess_reader_for_path("file.csv"), "read_csv_auto");
        assert_eq!(guess_reader_for_path("file.txt"), "read_csv_auto");
    }
}
