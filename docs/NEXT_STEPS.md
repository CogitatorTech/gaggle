# Next Steps for Implementation

**Date:** November 2, 2025  
**Project:** Gaggle - DuckDB Extension for Kaggle Datasets

Based on the ROADMAP.md, here are the prioritized next items to implement:

## High Priority Items

### 1. Cache Size Limit ⭐⭐⭐
**Category:** Caching and Storage  
**Status:** Not Started  
**Complexity:** Medium  
**Impact:** High

**Description:**
Implement configurable cache size limits to prevent unbounded disk usage.

**Tasks:**
- [ ] Add `GAGGLE_CACHE_SIZE_LIMIT_GB` environment variable
- [ ] Track total cache size when downloading datasets
- [ ] Implement LRU (Least Recently Used) eviction policy
- [ ] Add warning when approaching cache limit
- [ ] Update `gaggle_cache_info()` to show limit and usage percentage
- [ ] Add tests for cache limit enforcement

**Suggested Implementation:**
```rust
// In config.rs
pub fn cache_size_limit_bytes() -> Option<u64> {
    env::var("GAGGLE_CACHE_SIZE_LIMIT_GB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|gb| gb * 1024 * 1024 * 1024)
}

// In download.rs
fn check_and_enforce_cache_limit() -> Result<(), GaggleError> {
    if let Some(limit) = cache_size_limit_bytes() {
        let current_size = calculate_cache_size()?;
        if current_size > limit {
            evict_oldest_datasets_until_under_limit(limit)?;
        }
    }
    Ok(())
}
```

**Files to Modify:**
- `gaggle/src/config.rs` - Add cache limit configuration
- `gaggle/src/kaggle/download.rs` - Implement size checking and eviction
- `gaggle/src/ffi.rs` - Update `gaggle_get_cache_info()` to include limit
- `gaggle/bindings/gaggle_extension.cpp` - Update C++ binding

**Estimated Time:** 2-3 days

---

### 2. Cache Expiration Policies ⭐⭐
**Category:** Caching and Storage  
**Status:** Not Started  
**Complexity:** Medium  
**Impact:** Medium

**Description:**
Add automatic cache expiration based on dataset age.

**Tasks:**
- [ ] Store download timestamp in marker file or metadata
- [ ] Add `GAGGLE_CACHE_EXPIRY_DAYS` environment variable
- [ ] Check dataset age before serving from cache
- [ ] Add function to clean expired datasets: `gaggle_clean_expired_cache()`
- [ ] Add tests for expiration logic

**Suggested Implementation:**
```rust
// Store timestamp in .downloaded file
#[derive(Serialize, Deserialize)]
struct CacheMetadata {
    downloaded_at: SystemTime,
    dataset_path: String,
}

fn is_cache_expired(cache_dir: &Path) -> Result<bool, GaggleError> {
    let marker_file = cache_dir.join(".downloaded");
    if !marker_file.exists() {
        return Ok(true);
    }

    let metadata: CacheMetadata = serde_json::from_str(
        &fs::read_to_string(marker_file)?
    )?;

    let expiry_days = cache_expiry_days();
    let age = SystemTime::now()
        .duration_since(metadata.downloaded_at)?
        .as_secs() / 86400; // Convert to days

    Ok(age > expiry_days)
}
```

**Files to Modify:**
- `gaggle/src/config.rs` - Add expiry configuration
- `gaggle/src/kaggle/download.rs` - Implement expiration checking
- `gaggle/src/ffi.rs` - Add `gaggle_clean_expired_cache()` function

**Estimated Time:** 2 days

---

### 3. Excel/XLSX File Support ⭐⭐
**Category:** Data Integration  
**Status:** Not Started  
**Complexity:** Medium  
**Impact:** Medium

**Description:**
Add support for reading Excel files in Kaggle datasets.

**Tasks:**
- [ ] Add `calamine` crate dependency for Excel parsing
- [ ] Detect .xlsx/.xls files in replacement scan
- [ ] Convert Excel files to DuckDB-compatible format
- [ ] Handle multiple sheets (default to first sheet or allow sheet selection)
- [ ] Add tests for Excel file reading

**Suggested Implementation:**
```toml
# In Cargo.toml
[dependencies]
calamine = "0.24"
```

```cpp
// In gaggle_extension.cpp - update KaggleReplacementScan
if (StringUtil::EndsWith(lower_name, ".xlsx") ||
    StringUtil::EndsWith(lower_name, ".xls")) {
    // Convert to CSV first or use a custom reader
    func_name = "read_csv_auto"; // After conversion
}
```

**Files to Modify:**
- `gaggle/Cargo.toml` - Add calamine dependency
- `gaggle/src/kaggle/download.rs` - Add Excel conversion helper
- `gaggle/bindings/gaggle_extension.cpp` - Update replacement scan

**Estimated Time:** 3-4 days

---

### 4. Detailed Error Codes ⭐⭐
**Category:** Error Handling and Resilience  
**Status:** Not Started  
**Complexity:** Low  
**Impact:** Medium

**Description:**
Add numeric error codes for programmatic error handling.

**Tasks:**
- [ ] Define error code enum (1xxx for auth, 2xxx for network, etc.)
- [ ] Add error code to GaggleError variants
- [ ] Expose error code through FFI: `gaggle_last_error_code()`
- [ ] Document error codes in README
- [ ] Add tests for error code retrieval

**Suggested Implementation:**
```rust
#[derive(Debug, Clone, Copy)]
pub enum GaggleErrorCode {
    Success = 0,
    NullPointer = 1001,
    InvalidUtf8 = 1002,
    CredentialsNotFound = 2001,
    CredentialsInvalid = 2002,
    DatasetNotFound = 3001,
    InvalidDatasetPath = 3002,
    NetworkError = 4001,
    HttpError = 4002,
    // ... more codes
}

impl GaggleError {
    pub fn code(&self) -> GaggleErrorCode {
        match self {
            GaggleError::NullPointer => GaggleErrorCode::NullPointer,
            GaggleError::CredentialsError(_) => GaggleErrorCode::CredentialsNotFound,
            // ... more mappings
        }
    }
}

#[no_mangle]
pub extern "C" fn gaggle_last_error_code() -> i32 {
    // Return error code from last error
}
```

**Files to Modify:**
- `gaggle/src/error.rs` - Add error codes
- `gaggle/src/ffi.rs` - Add `gaggle_last_error_code()` function
- `gaggle/bindings/include/rust.h` - Add error code constants

**Estimated Time:** 1-2 days

---

## Medium Priority Items

### 5. Upload DuckDB Tables to Kaggle ⭐
**Category:** Kaggle API Integration  
**Status:** Not Started  
**Complexity:** High  
**Impact:** Medium

**Description:**
Allow users to export DuckDB tables/query results as Kaggle datasets.

**Tasks:**
- [ ] Research Kaggle dataset upload API
- [ ] Implement dataset creation endpoint
- [ ] Add file upload functionality
- [ ] Create `gaggle_upload_table()` SQL function
- [ ] Handle dataset metadata (title, description, tags)
- [ ] Add comprehensive tests

**Estimated Time:** 5-7 days

---

### 6. Virtual Table Support for Lazy Loading ⭐
**Category:** Data Integration  
**Status:** Not Started  
**Complexity:** High  
**Impact:** High

**Description:**
Implement virtual tables that load data on-demand without full download.

**Tasks:**
- [ ] Implement streaming download with range requests
- [ ] Create virtual table interface
- [ ] Add pagination/chunking for large datasets
- [ ] Optimize for columnar access patterns
- [ ] Add tests for lazy loading

**Estimated Time:** 7-10 days

---

### 7. Incremental Cache Updates ⭐
**Category:** Performance and Concurrency  
**Status:** Not Started  
**Complexity:** High  
**Impact:** Medium

**Description:**
Update cached datasets incrementally instead of full re-download.

**Tasks:**
- [ ] Implement differential sync with Kaggle API
- [ ] Track dataset versions/checksums
- [ ] Download only changed files
- [ ] Add tests for incremental updates

**Estimated Time:** 5-7 days

---

## Lower Priority / Future Items

### 8. Cloud Storage Backend Support
- S3 integration
- Google Cloud Storage integration
- Azure Blob Storage integration

### 9. Advanced Documentation
- Tutorial documentation
- FAQ section
- Troubleshooting guide

### 10. Distribution
- Pre-compiled binaries for all platforms
- Automated release pipeline
- Community Extensions submission

---

## Recommended Implementation Order

Based on impact, complexity, and dependencies:

1. **Cache Size Limit** (High impact, medium complexity)
2. **Detailed Error Codes** (Medium impact, low complexity)
3. **Cache Expiration Policies** (Medium impact, medium complexity)
4. **Excel/XLSX Support** (Medium impact, medium complexity)
5. **Upload DuckDB Tables** (Medium impact, high complexity)
6. **Virtual Table/Lazy Loading** (High impact, high complexity)
7. **Incremental Cache Updates** (Medium impact, high complexity)
8. **Cloud Storage Backends** (Lower impact, high complexity)

---

## Quick Wins (Can be done in parallel)

These items can be implemented independently:

- **Detailed Error Codes** - 1-2 days
- **Cache Size Limit** - 2-3 days
- **Cache Expiration Policies** - 2 days

Total quick wins: ~5-7 days of work

---

## Notes

- All implementations should include comprehensive unit tests
- Update documentation for each feature
- Consider backward compatibility
- Add configuration examples to README
- Update the ROADMAP.md as items are completed

## Current Project Status

✅ **Completed:** Core functionality, security hardening, comprehensive test coverage  
🚀 **Ready for:** Feature expansion and production hardening  
📊 **Test Coverage:** 159 tests passing (136 unit + 23 integration)
