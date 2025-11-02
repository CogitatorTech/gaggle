# Kaggle Dataset Versioning Analysis

**Date:** November 2, 2025  
**Topic:** How Kaggle Dataset Versioning Works with Gaggle API  
**Status:** Analysis and Recommendations

## Overview

Kaggle datasets support versioning - dataset owners can publish multiple versions of their datasets. This analysis explores how the current Gaggle implementation handles (or doesn't handle) dataset versions.

## How Kaggle Dataset Versioning Works

### Kaggle's Versioning Model

1. **Version Numbers**: Datasets have integer version numbers (1, 2, 3, etc.)
2. **Default Behavior**: When you request a dataset without specifying a version, Kaggle returns the **latest version**
3. **Version Pinning**: You can request a specific version by appending `/versions/{version_number}` to the API endpoint
4. **Version Metadata**: Each version has its own metadata (creation time, size, description, files)

### Kaggle API Endpoints

```
# Latest version (default)
GET /datasets/download/{owner}/{dataset}

# Specific version
GET /datasets/download/{owner}/{dataset}/versions/{version}

# Get dataset metadata (includes version info)
GET /datasets/view/{owner}/{dataset}

# List all versions
GET /datasets/view/{owner}/{dataset}/versions
```

## Current Gaggle Implementation

### What Gaggle Does Now

**✅ Implements:**
1. Downloads the **latest version** of a dataset by default
2. Has infrastructure for version tracking:
   - `CacheMetadata` struct includes `version: Option<String>` field
   - Currently always set to `None`
3. Caches datasets locally with no version awareness
4. Uses path format: `cache_dir/datasets/{owner}/{dataset}/`

**❌ Missing:**
1. No way to specify which version to download
2. No version checking on subsequent downloads
3. No detection of dataset updates
4. No version information stored in cache metadata
5. No version information exposed to users

### Current API Signature

```rust
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError>
```

**Parameters:**
- `dataset_path`: "owner/dataset" format only

**Behavior:**
- Always downloads latest version
- Reuses cached version if marker file exists
- No version checking or comparison

### SQL API

```sql
-- Current API
SELECT gaggle_download('owner/dataset');  -- Always gets latest, or uses cache

-- What users might expect (but doesn't work)
SELECT gaggle_download('owner/dataset/versions/2');  -- Not supported
```

## Issues with Current Implementation

### 1. Cache Staleness
**Problem:** Once a dataset is cached, Gaggle never checks for updates.

**Scenario:**
```sql
-- Day 1: Download dataset (version 1)
SELECT gaggle_download('owner/dataset');  -- Downloads v1

-- Day 30: Owner publishes version 2
-- User tries again
SELECT gaggle_download('owner/dataset');  -- Still returns v1 (cached)
```

**Impact:**
- Users unknowingly use outdated data
- No warning or indication that newer version exists

### 2. No Version Pinning
**Problem:** Can't request specific versions.

**Scenario:**
```
A research paper uses "owner/dataset" version 3 for reproducibility.
Readers trying to reproduce results get version 5 (latest).
Results don't match due to data changes.
```

**Impact:**
- Breaks reproducibility
- Makes it hard to debug data-dependent issues

### 3. No Version Awareness
**Problem:** Users don't know what version they have.

**Current:**
```sql
SELECT gaggle_cache_info();
-- Returns: {"path": "...", "size_mb": 1024, ...}
-- No version information
```

**Impact:**
- Can't verify which version is cached
- Can't determine if cache is stale

### 4. No Update Detection
**Problem:** No mechanism to detect when datasets are updated.

**Missing Features:**
- Check if cached version is outdated
- Force download of latest version
- Compare cached vs latest version

## Proposed Solutions

### Solution 1: Add Version Support to Download API

#### Option A: Extend Dataset Path Format

```rust
// Support version in path
pub fn download_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError>

// Examples:
// "owner/dataset"           -> latest version (current behavior)
// "owner/dataset@v2"        -> specific version 2
// "owner/dataset@latest"    -> explicitly latest (same as no version)
```

**SQL API:**
```sql
-- Latest version (default, backward compatible)
SELECT gaggle_download('owner/dataset');

-- Specific version
SELECT gaggle_download('owner/dataset@v2');

-- Explicit latest
SELECT gaggle_download('owner/dataset@latest');
```

**Pros:**
- ✅ Backward compatible (no version = latest)
- ✅ Simple syntax
- ✅ Works with existing path parsing

**Cons:**
- ⚠️ Need to validate version format
- ⚠️ '@' character might conflict with other uses

#### Option B: Add Separate Version Parameter

```rust
pub fn download_dataset_version(
    dataset_path: &str,
    version: Option<u32>
) -> Result<PathBuf, GaggleError>
```

**SQL API:**
```sql
-- New function with version parameter
SELECT gaggle_download_versioned('owner/dataset', 2);

-- Keep existing function for backward compatibility
SELECT gaggle_download('owner/dataset');  -- Latest
```

**Pros:**
- ✅ Explicit and clear
- ✅ Type-safe version number

**Cons:**
- ❌ Requires new function
- ❌ More complex API

### Solution 2: Store Version in Cache Metadata

```rust
struct CacheMetadata {
    downloaded_at_secs: u64,
    dataset_path: String,
    size_mb: u64,
    version: Option<String>,  // Already exists, just needs to be populated
}
```

**Implementation:**
1. Extract version from API response headers or metadata
2. Store in `.downloaded` marker file
3. Expose via `gaggle_cache_info()`

**Enhanced Cache Info:**
```json
{
  "path": "/cache/datasets/owner/dataset",
  "size_mb": 1024,
  "limit_mb": 102400,
  "usage_percent": 1,
  "is_soft_limit": true,
  "type": "local",
  "version": "5",           // NEW
  "downloaded_at": "2025-11-01T10:00:00Z"
}
```

### Solution 3: Add Version Check and Update Functions

```rust
// Check if cached version is latest
pub fn is_dataset_current(dataset_path: &str) -> Result<bool, GaggleError>

// Force download of latest version (ignore cache)
pub fn update_dataset(dataset_path: &str) -> Result<PathBuf, GaggleError>

// Get version info without downloading
pub fn get_dataset_version_info(dataset_path: &str) -> Result<VersionInfo, GaggleError>
```

**SQL API:**
```sql
-- Check if cached version is current
SELECT gaggle_is_current('owner/dataset');  -- Returns boolean

-- Force update to latest
SELECT gaggle_update_dataset('owner/dataset');

-- Get version information
SELECT gaggle_version_info('owner/dataset');
-- Returns: {"cached": "3", "latest": "5", "is_current": false}
```

### Solution 4: Cache Directory Structure with Versions

**Current Structure:**
```
cache/datasets/owner/dataset/
  ├── .downloaded      (metadata)
  ├── file1.csv
  └── file2.json
```

**Proposed Structure (Option A - One version at a time):**
```
cache/datasets/owner/dataset/
  ├── .downloaded      (includes version metadata)
  ├── file1.csv
  └── file2.json
```

**Proposed Structure (Option B - Multiple versions):**
```
cache/datasets/owner/dataset/
  ├── v1/
  │   ├── .downloaded
  │   ├── file1.csv
  │   └── file2.json
  ├── v2/
  │   ├── .downloaded
  │   ├── file1.csv
  │   └── file2.json
  └── latest -> v2/    (symlink)
```

**Option A Pros:**
- ✅ Simpler
- ✅ Less disk space
- ✅ Consistent with current structure

**Option B Pros:**
- ✅ Supports multiple versions simultaneously
- ✅ Easy rollback
- ✅ Useful for reproducibility

**Option B Cons:**
- ❌ More disk space
- ❌ More complex cache management
- ❌ Need symlink support (not available on all systems)

## Recommended Implementation Plan

### Phase 1: Immediate (Required for Production)

**Priority: HIGH**

1. **Store Version in Metadata**
   - Extract version from Kaggle API response
   - Populate `version` field in `CacheMetadata`
   - Display in `gaggle_cache_info()`

2. **Add Version Check on Cache Hit**
   - When marker file exists, check if it's still the latest version
   - Optional: Add `GAGGLE_CACHE_REVALIDATE` env var to control behavior

3. **Add Update Function**
   - `gaggle_update_dataset()` to force refresh
   - Ignores cache and downloads latest

**Effort:** 2-3 days  
**Impact:** Solves cache staleness issue

### Phase 2: Short-term (Within 1 month)

**Priority: MEDIUM**

1. **Version Pinning Support**
   - Add `@vN` syntax to dataset paths
   - Update path parsing to extract version
   - Modify download URL to include version

2. **Version Information Functions**
   - `gaggle_version_info()` to show version details
   - `gaggle_is_current()` to check if cached version is latest

3. **Cache Directory Updates**
   - Option: Store version in directory name for multi-version support
   - Ensure LRU eviction works across versions

**Effort:** 5-7 days  
**Impact:** Enables reproducibility

### Phase 3: Long-term (Future)

**Priority: LOW**

1. **Automatic Update Detection**
   - Background checking for updates
   - Notifications when new versions available

2. **Version History**
   - Store multiple versions
   - Easy switching between versions

3. **Version Metadata API**
   - List all available versions
   - Show changelog/differences between versions

**Effort:** 10-15 days  
**Impact:** Advanced version management

## Backward Compatibility

All proposed changes should maintain backward compatibility:

```sql
-- Current behavior (no version = latest)
SELECT gaggle_download('owner/dataset');

-- Still works after changes, but now:
-- 1. Stores version metadata
-- 2. Checks for updates on subsequent calls (configurable)
-- 3. Warns if cached version is outdated
```

## Configuration Options

```bash
# Check for updates on every download (default: false for performance)
export GAGGLE_CACHE_REVALIDATE=true

# Cache revalidation interval in seconds (default: 86400 = 1 day)
export GAGGLE_CACHE_REVALIDATE_INTERVAL=3600

# Keep multiple versions (default: false)
export GAGGLE_KEEP_ALL_VERSIONS=true

# Maximum versions to keep per dataset (default: 1)
export GAGGLE_MAX_VERSIONS_PER_DATASET=3
```

## API Changes Summary

### New SQL Functions (Phase 1 & 2)

```sql
-- Force update to latest version (Phase 1)
gaggle_update_dataset(dataset_path VARCHAR) -> VARCHAR

-- Check if cached is current (Phase 1)
gaggle_is_current(dataset_path VARCHAR) -> BOOLEAN

-- Get version information (Phase 2)
gaggle_version_info(dataset_path VARCHAR) -> VARCHAR (JSON)

-- Download specific version (Phase 2)
gaggle_download(dataset_path VARCHAR)  -- Now supports 'owner/dataset@v2'
```

### Enhanced Functions

```sql
-- Enhanced cache info (Phase 1)
gaggle_cache_info() -> VARCHAR (JSON)
-- Now includes: "version", "downloaded_at", "is_current"

-- Enhanced dataset info (Phase 1)
gaggle_info(dataset_path VARCHAR) -> VARCHAR (JSON)
-- Now includes: "current_version", "total_versions"
```

## Testing Considerations

**New Tests Needed:**
1. Version parsing from dataset paths
2. Version extraction from API responses
3. Version comparison logic
4. Cache revalidation
5. Multi-version storage
6. Version pinning in SQL queries

**All tests must remain offline** (no real Kaggle API calls)

## Documentation Updates Needed

1. **README.md** - Add versioning section
2. **docs/README.md** - Document new functions
3. **docs/CONFIGURATION.md** - Add version-related env vars
4. **ROADMAP.md** - Add versioning features
5. **New: docs/VERSIONING.md** - Complete versioning guide

## Conclusion

**Current State:**
- ❌ No version awareness
- ❌ Cache can become stale
- ❌ No reproducibility support
- ✅ Infrastructure ready (version field exists)

**Recommended Action:**
Implement **Phase 1** immediately to address cache staleness and provide basic version awareness. This is critical for production use.

**Timeline:**
- Phase 1: 2-3 days
- Phase 2: 5-7 days  
- Phase 3: Future enhancement

**Impact:**
- Fixes major usability issue (stale cache)
- Enables reproducible research
- Maintains backward compatibility
- Aligns with Kaggle's versioning model
