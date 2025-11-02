# Implementation Guide: Version Pinning & Dataset Upload

**Date:** November 2, 2025  
**Features:**
1. Download specific dataset versions (version pinning)
2. Upload DuckDB tables to Kaggle as datasets

## Feature 1: Download Specific Dataset Versions (Version Pinning)

### Overview

Allow users to download and pin to specific versions of Kaggle datasets for reproducibility.

### Current Status

- ✅ Version tracking implemented (Phase 1)
- ✅ Version checking (`gaggle_is_current`)
- ✅ Force updates (`gaggle_update_dataset`)
- ❌ **Version pinning NOT implemented**

### Implementation Plan

#### 1.1 API Design

**Syntax Options:**

**Option A: Version in path (Recommended)**
```sql
-- Pin to specific version
SELECT gaggle_download('owner/dataset@v2');
SELECT gaggle_download('owner/dataset@v5');

-- Explicit latest
SELECT gaggle_download('owner/dataset@latest');

-- No version = latest (backward compatible)
SELECT gaggle_download('owner/dataset');
```

**Option B: Separate parameter**
```sql
SELECT gaggle_download_version('owner/dataset', 2);
```

**Recommendation:** Use Option A - cleaner, more intuitive, works with replacement scan.

#### 1.2 Path Parsing Updates

**File:** `gaggle/src/kaggle/mod.rs`

```rust
/// Parse dataset path with optional version
/// Formats:
///   "owner/dataset" -> (owner, dataset, None)
///   "owner/dataset@v2" -> (owner, dataset, Some("2"))
///   "owner/dataset@latest" -> (owner, dataset, None)
pub fn parse_dataset_path_with_version(path: &str) -> Result<(String, String, Option<String>), GaggleError> {
    // Split on @ to extract version
    let parts: Vec<&str> = path.split('@').collect();

    if parts.len() > 2 {
        return Err(GaggleError::InvalidDatasetPath(
            "Dataset path can only contain one @ for version".to_string()
        ));
    }

    let dataset_path = parts[0];
    let version = if parts.len() == 2 {
        let v = parts[1].trim();
        if v == "latest" || v.is_empty() {
            None
        } else {
            // Remove 'v' prefix if present
            let version_str = v.strip_prefix('v').unwrap_or(v);
            // Validate it's a number
            if version_str.parse::<u32>().is_err() {
                return Err(GaggleError::InvalidDatasetPath(
                    format!("Invalid version number: {}", v)
                ));
            }
            Some(version_str.to_string())
        }
    } else {
        None
    };

    // Parse owner/dataset
    let (owner, dataset) = parse_dataset_path(dataset_path)?;

    Ok((owner, dataset, version))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_with_version() {
        let (owner, dataset, version) = parse_dataset_path_with_version("owner/dataset@v2").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(dataset, "dataset");
        assert_eq!(version, Some("2".to_string()));
    }

    #[test]
    fn test_parse_with_version_no_v() {
        let (owner, dataset, version) = parse_dataset_path_with_version("owner/dataset@5").unwrap();
        assert_eq!(version, Some("5".to_string()));
    }

    #[test]
    fn test_parse_latest() {
        let (owner, dataset, version) = parse_dataset_path_with_version("owner/dataset@latest").unwrap();
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_no_version() {
        let (owner, dataset, version) = parse_dataset_path_with_version("owner/dataset").unwrap();
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_invalid_version() {
        let result = parse_dataset_path_with_version("owner/dataset@abc");
        assert!(result.is_err());
    }
}
```

#### 1.3 Download Function Updates

**File:** `gaggle/src/kaggle/download.rs`

```rust
/// Download a specific version of a Kaggle dataset
pub fn download_dataset_version(
    dataset_path: &str,
    version: Option<String>
) -> Result<PathBuf, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    // Cache directory includes version if specified
    let cache_subdir = if let Some(ref v) = version {
        format!("{}/v{}", dataset, v)
    } else {
        dataset.clone()
    };

    let cache_dir = crate::config::cache_dir_runtime()
        .join("datasets")
        .join(&owner)
        .join(&cache_subdir);

    // Check if already downloaded
    let marker_file = cache_dir.join(".downloaded");
    if marker_file.exists() {
        return Ok(cache_dir);
    }

    // Acquire download lock
    let lock_key = format!("{}/{}/{:?}", owner, dataset, version);
    // ... existing lock logic ...

    fs::create_dir_all(&cache_dir)?;

    // Build URL with version if specified
    let url = if let Some(ref v) = version {
        format!(
            "{}/datasets/download/{}/{}/versions/{}",
            get_api_base(), owner, dataset, v
        )
    } else {
        format!(
            "{}/datasets/download/{}/{}",
            get_api_base(), owner, dataset
        )
    };

    // ... rest of download logic ...

    // Store version in metadata
    let mut metadata = CacheMetadata::new(dataset_path.to_string(), dataset_size_mb);
    metadata.version = version.or_else(|| {
        super::metadata::get_current_version(dataset_path).ok()
    });
    fs::write(&marker_file, serde_json::to_string(&metadata)?)?;

    Ok(cache_dir)
}

/// Updated main download function with version support
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError> {
    // Parse path to extract version
    let (owner, dataset, version) = super::parse_dataset_path_with_version(dataset_path)?;
    let reconstructed_path = format!("{}/{}", owner, dataset);

    download_dataset_version(&reconstructed_path, version)
}
```

#### 1.4 Cache Structure for Versioned Datasets

**Directory Structure:**
```
cache/datasets/
├── owner1/
│   ├── dataset1/           # Latest version (no @version in request)
│   │   ├── .downloaded
│   │   └── data.csv
│   ├── dataset1/
│   │   └── v2/             # Pinned version 2
│   │       ├── .downloaded
│   │       └── data.csv
│   └── dataset1/
│       └── v3/             # Pinned version 3
│           ├── .downloaded
│           └── data.csv
```

**Alternative (simpler):**
```
cache/datasets/
├── owner1/
│   ├── dataset1/           # Latest version
│   ├── dataset1-v2/        # Pinned version 2
│   └── dataset1-v3/        # Pinned version 3
```

#### 1.5 SQL Function Updates

**File:** `gaggle/bindings/gaggle_extension.cpp`

No changes needed - existing `DownloadDataset` function will automatically support new syntax!

```cpp
// This already works:
SELECT gaggle_download('owner/dataset@v2');
// Because we parse the version in Rust
```

#### 1.6 Replacement Scan Updates

**File:** `gaggle/bindings/gaggle_extension.cpp`

```cpp
// In KaggleReplacementScan function:
// Already supports versioned paths!
SELECT * FROM 'kaggle:owner/dataset@v2/file.csv';
```

#### 1.7 Testing

**File:** `gaggle/src/kaggle/download.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_download_specific_version() {
        // This would require mock HTTP server
        // For now, test path parsing
        let (owner, dataset, version) =
            super::parse_dataset_path_with_version("owner/dataset@v2").unwrap();
        assert_eq!(version, Some("2".to_string()));
    }

    #[test]
    fn test_version_in_cache_path() {
        // Test that versioned downloads create separate cache directories
        let path_v2 = "owner/dataset@v2";
        let path_v3 = "owner/dataset@v3";
        // Verify different cache directories
    }
}
```

### Implementation Steps

1. **Add version parsing** (30 min)
   - Implement `parse_dataset_path_with_version()`
   - Add comprehensive tests

2. **Update download function** (1 hour)
   - Add `download_dataset_version()`
   - Update URL building with version
   - Update cache directory logic

3. **Update cache structure** (30 min)
   - Handle versioned cache directories
   - Update LRU eviction to work across versions

4. **Test thoroughly** (1 hour)
   - Unit tests for parsing
   - Integration tests with mocked API
   - Test cache isolation between versions

5. **Update documentation** (30 min)
   - README examples
   - API documentation
   - Example SQL file

**Total Estimated Time:** 3-4 hours

---

## Feature 2: Upload DuckDB Tables to Kaggle

### Overview

Allow users to export DuckDB tables or query results as Kaggle datasets.

### Kaggle API Requirements

**Research Needed:**
- Kaggle API endpoint for dataset creation
- File upload mechanism (multipart form? direct upload?)
- Metadata requirements (title, description, license)
- File format requirements
- Authentication needs

### Implementation Plan

#### 2.1 Kaggle API Research

**Endpoint:** `POST /api/v1/datasets/create/new`

**Required Information:**
- Dataset slug (unique identifier)
- Title
- Description (optional)
- License (optional)
- Files to upload (CSV, Parquet, etc.)
- Dataset type (public/private)

**API Flow:**
1. Create dataset metadata
2. Upload files via multipart form
3. Receive dataset URL

#### 2.2 API Design

**Option A: Simple upload**
```sql
-- Export table to Kaggle
SELECT gaggle_upload_table(
    'my_table',                    -- Table name
    'myusername/my-dataset',       -- Target dataset path
    'My Dataset Title',            -- Title
    'Dataset description'          -- Description (optional)
);
```

**Option B: Export query results**
```sql
-- Export query results
SELECT gaggle_upload_query(
    'SELECT * FROM my_table WHERE value > 100',
    'myusername/my-dataset',
    'My Dataset Title',
    'Filtered results'
);
```

**Option C: Export to file, then upload**
```sql
-- Step 1: Export to file
COPY my_table TO '/tmp/data.csv' (FORMAT CSV, HEADER);

-- Step 2: Upload file
SELECT gaggle_upload_file(
    '/tmp/data.csv',
    'myusername/my-dataset',
    'My Dataset Title',
    'My description'
);
```

**Recommendation:** Start with Option C (most flexible), then add A and B as convenience wrappers.

#### 2.3 Rust Implementation

**File:** `gaggle/src/kaggle/upload.rs` (NEW FILE)

```rust
use crate::error::GaggleError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::api::{build_client, get_api_base, with_retries};
use super::credentials::get_credentials;

#[derive(Debug, Serialize)]
struct DatasetMetadata {
    title: String,
    slug: String,
    description: Option<String>,
    license: Option<String>,
    is_private: bool,
}

/// Upload a file as a new Kaggle dataset
pub fn upload_dataset(
    file_path: &str,
    dataset_path: &str,
    title: &str,
    description: Option<&str>,
) -> Result<String, GaggleError> {
    let creds = get_credentials()?;
    let (owner, dataset) = super::parse_dataset_path(dataset_path)?;

    // Verify file exists
    let file = Path::new(file_path);
    if !file.exists() {
        return Err(GaggleError::IoError(format!(
            "File not found: {}",
            file_path
        )));
    }

    // Prepare metadata
    let metadata = DatasetMetadata {
        title: title.to_string(),
        slug: dataset.clone(),
        description: description.map(|s| s.to_string()),
        license: Some("CC0-1.0".to_string()), // Default to public domain
        is_private: false,
    };

    // Build multipart form
    let client = build_client()?;
    let url = format!("{}/datasets/create/new", get_api_base());

    let form = reqwest::blocking::multipart::Form::new()
        .text("title", metadata.title)
        .text("slug", metadata.slug)
        .text("ownerSlug", owner);

    if let Some(desc) = metadata.description {
        form.text("description", desc);
    }

    // Add file
    let file_content = fs::read(file)?;
    let file_name = file.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| GaggleError::IoError("Invalid file name".to_string()))?;

    let form = form.part(
        "files",
        reqwest::blocking::multipart::Part::bytes(file_content)
            .file_name(file_name.to_string())
    );

    // Send request
    let response = with_retries(|| {
        client
            .post(&url)
            .basic_auth(&creds.username, Some(&creds.key))
            .multipart(form.try_clone().ok_or_else(||
                GaggleError::HttpRequestError("Failed to clone form".to_string())
            )?)
            .send()
            .map_err(|e| GaggleError::HttpRequestError(e.to_string()))
    })?;

    if !response.status().is_success() {
        let error_body = response.text().unwrap_or_else(|_| "Unknown error".to_string());
        return Err(GaggleError::HttpRequestError(format!(
            "Failed to upload dataset: HTTP {} - {}",
            response.status(),
            error_body
        )));
    }

    // Return dataset URL
    Ok(format!("https://www.kaggle.com/{}/{}", owner, dataset))
}

/// Upload a DuckDB table as a Kaggle dataset
pub fn upload_table(
    table_name: &str,
    dataset_path: &str,
    title: &str,
    description: Option<&str>,
) -> Result<String, GaggleError> {
    // This would require DuckDB C API integration
    // For now, return an error directing users to use COPY + upload_file
    Err(GaggleError::IoError(
        "Direct table upload not yet implemented. \
         Use: COPY table TO '/tmp/file.csv' then gaggle_upload_file()".to_string()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_file_validation() {
        std::env::set_var("KAGGLE_USERNAME", "test");
        std::env::set_var("KAGGLE_KEY", "test");

        // Should fail for non-existent file
        let result = upload_dataset(
            "/nonexistent/file.csv",
            "owner/dataset",
            "Test Dataset",
            None
        );
        assert!(result.is_err());

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }
}
```

#### 2.4 FFI Bindings

**File:** `gaggle/src/ffi.rs`

```rust
/// Upload a file as a Kaggle dataset
///
/// # Arguments
///
/// * `file_path` - Local file path to upload
/// * `dataset_path` - Target dataset path (owner/dataset)
/// * `title` - Dataset title
/// * `description` - Dataset description (can be NULL)
///
/// # Returns
///
/// Pointer to dataset URL string, or NULL on failure
#[no_mangle]
pub unsafe extern "C" fn gaggle_upload_file(
    file_path: *const c_char,
    dataset_path: *const c_char,
    title: *const c_char,
    description: *const c_char,
) -> *mut c_char {
    error::clear_last_error_internal();

    let result = (|| -> Result<String, error::GaggleError> {
        if file_path.is_null() || dataset_path.is_null() || title.is_null() {
            return Err(error::GaggleError::NullPointer);
        }

        let file_str = CStr::from_ptr(file_path).to_str()?;
        let dataset_str = CStr::from_ptr(dataset_path).to_str()?;
        let title_str = CStr::from_ptr(title).to_str()?;

        let desc_str = if description.is_null() {
            None
        } else {
            Some(CStr::from_ptr(description).to_str()?)
        };

        kaggle::upload_dataset(file_str, dataset_str, title_str, desc_str)
    })();

    match result {
        Ok(url) => string_to_c_string(url),
        Err(e) => {
            error::set_last_error(&e);
            std::ptr::null_mut()
        }
    }
}
```

#### 2.5 C++ SQL Function

**File:** `gaggle/bindings/gaggle_extension.cpp`

```cpp
static void UploadFile(DataChunk &args, ExpressionState &state, Vector &result) {
    if (args.ColumnCount() < 3 || args.ColumnCount() > 4) {
        throw InvalidInputException(
            "gaggle_upload_file(file_path, dataset_path, title, [description])");
    }
    if (args.size() == 0) {
        return;
    }

    auto file_path = args.data[0].GetValue(0).ToString();
    auto dataset_path = args.data[1].GetValue(0).ToString();
    auto title = args.data[2].GetValue(0).ToString();

    const char* description = nullptr;
    if (args.ColumnCount() == 4) {
        auto desc_val = args.data[3].GetValue(0);
        if (!desc_val.IsNull()) {
            auto desc_str = desc_val.ToString();
            description = desc_str.c_str();
        }
    }

    char* dataset_url = gaggle_upload_file(
        file_path.c_str(),
        dataset_path.c_str(),
        title.c_str(),
        description
    );

    if (dataset_url == nullptr) {
        throw InvalidInputException("Failed to upload dataset: " + GetGaggleError());
    }

    result.SetVectorType(VectorType::CONSTANT_VECTOR);
    ConstantVector::GetData<string_t>(result)[0] =
        StringVector::AddString(result, dataset_url);
    ConstantVector::SetNull(result, false);
    gaggle_free(dataset_url);
}

// Register in LoadInternal:
loader.RegisterFunction(ScalarFunction(
    "gaggle_upload_file",
    {LogicalType::VARCHAR, LogicalType::VARCHAR, LogicalType::VARCHAR, LogicalType::VARCHAR},
    LogicalType::VARCHAR,
    UploadFile
));
```

#### 2.6 Usage Examples

```sql
-- Example 1: Export table and upload
COPY my_table TO '/tmp/my_data.csv' (FORMAT CSV, HEADER);

SELECT gaggle_upload_file(
    '/tmp/my_data.csv',
    'myusername/my-dataset',
    'My Awesome Dataset',
    'This dataset contains interesting data'
) as dataset_url;

-- Example 2: Export query results
COPY (
    SELECT * FROM sales
    WHERE year = 2024
    ORDER BY revenue DESC
) TO '/tmp/sales_2024.csv' (FORMAT CSV, HEADER);

SELECT gaggle_upload_file(
    '/tmp/sales_2024.csv',
    'myusername/sales-2024',
    'Sales Data 2024',
    'Top revenue generating sales from 2024'
);

-- Example 3: Export Parquet
COPY my_table TO '/tmp/data.parquet' (FORMAT PARQUET);

SELECT gaggle_upload_file(
    '/tmp/data.parquet',
    'myusername/my-parquet-dataset',
    'My Parquet Dataset',
    NULL  -- No description
);
```

### Implementation Steps

1. **Research Kaggle API** (2 hours)
   - Study Kaggle's dataset creation API
   - Test with curl/Python to understand requirements
   - Document authentication and file upload process

2. **Implement upload module** (3 hours)
   - Create `kaggle/upload.rs`
   - Implement `upload_dataset()` function
   - Handle multipart form upload
   - Error handling

3. **Add FFI bindings** (1 hour)
   - Add `gaggle_upload_file()` C function
   - Add C++ wrapper
   - Register SQL function

4. **Testing** (2 hours)
   - Unit tests for validation
   - Integration tests with mock server
   - Test with actual Kaggle API (manual)

5. **Documentation** (1 hour)
   - Add upload examples
   - Update API reference
   - Create tutorial

**Total Estimated Time:** 9-10 hours

### Challenges

1. **Kaggle API Access**
   - Need to verify exact API endpoints
   - May require API beta access
   - Rate limiting considerations

2. **File Format Support**
   - Kaggle may have restrictions on file types
   - Size limits per file/dataset
   - Metadata requirements

3. **DuckDB Integration**
   - Direct table access from Rust requires C API
   - May need to go through file export first
   - Alternative: Use DuckDB extension capabilities

### Alternative Approach

If Kaggle API doesn't support direct uploads, use Kaggle CLI approach:

```bash
# User workflow:
# 1. Export from DuckDB
COPY my_table TO '/tmp/dataset.csv';

# 2. Create kaggle metadata file
# 3. Use kaggle CLI
kaggle datasets create -p /tmp/dataset
```

We could wrap this in Rust:
```rust
// Execute kaggle CLI as subprocess
std::process::Command::new("kaggle")
    .args(&["datasets", "create", "-p", path])
    .output()?;
```

---

## Summary

### Version Pinning (Easier)
- **Complexity:** Medium
- **Time:** 3-4 hours
- **Dependencies:** None (builds on existing code)
- **Risk:** Low
- **Impact:** High (enables reproducibility)

**Recommendation:** **Implement immediately** - straightforward extension of existing versioning work.

### Dataset Upload (Harder)
- **Complexity:** High
- **Time:** 9-10 hours
- **Dependencies:** Kaggle API research, possible beta access
- **Risk:** Medium-High (API may change, restrictions)
- **Impact:** Medium (nice-to-have, not critical)

**Recommendation:** **Defer to Phase 3** - requires more research and may have external dependencies.

---

## Recommended Implementation Order

1. **✅ NOW: Version Pinning** (3-4 hours)
   - Clean extension of Phase 1 versioning
   - High user value for reproducibility
   - Low risk

2. **⏭️ LATER: Dataset Upload** (9-10 hours)
   - Requires Kaggle API research
   - Less critical than other features
   - Can use workarounds (export + kaggle CLI)

3. **🎯 PRIORITIZE INSTEAD:**
   - Detailed error codes (2 days)
   - Excel/XLSX support (3-4 days)
   - These provide more immediate value

Would you like me to implement **Version Pinning** now? It's ready to go and will complete the versioning feature set!
