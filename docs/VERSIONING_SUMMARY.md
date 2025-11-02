# Kaggle Dataset Versioning - Executive Summary

**Date:** November 2, 2025  
**Question:** How does Kaggle dataset versioning work with current Gaggle API?  
**Answer:** Currently not supported, but infrastructure is ready.

## Current Status: ⚠️ Limited Version Support

### What Works Now
- ✅ Downloads **latest version** of any dataset
- ✅ Caches datasets locally  
- ✅ Has `version: Option<String>` field in metadata (currently unused)

### What Doesn't Work
- ❌ No way to specify version to download
- ❌ No version stored in cache metadata
- ❌ No detection of dataset updates
- ❌ **Cache can become stale** - once downloaded, never checks for newer versions
- ❌ No reproducibility support - can't pin to specific version

## Critical Issue: Cache Staleness

**Problem:**
```sql
-- Day 1: Download dataset (version 1)
SELECT gaggle_download('owner/dataset');  -- Downloads v1, caches it

-- Day 30: Owner publishes version 2 with important fixes

-- User queries again
SELECT gaggle_download('owner/dataset');  -- Returns stale v1 from cache!
                                          -- No warning, no update check
```

**Impact:**
- Users unknowingly work with outdated data
- Breaks data pipelines that need latest data
- Makes reproducibility impossible (can't pin versions)

## How Kaggle Versioning Actually Works

**Kaggle API:**
```bash
# Latest version (what Gaggle uses now)
GET /datasets/download/owner/dataset

# Specific version (not supported by Gaggle)
GET /datasets/download/owner/dataset/versions/2

# Get version info (not used by Gaggle)
GET /datasets/view/owner/dataset  # Returns version metadata
```

**Version Model:**
- Datasets have integer versions: 1, 2, 3, ...
- Default = latest version
- Each version is immutable
- Version info available in metadata

## Recommended Solution

### Phase 1: Fix Cache Staleness (URGENT - 2-3 days)

**Implement:**
1. Store version number in cache metadata
2. Check for updates when accessing cached datasets
3. Add `gaggle_update_dataset()` to force refresh
4. Add version info to `gaggle_cache_info()`

**New SQL Functions:**
```sql
-- Force download latest version
SELECT gaggle_update_dataset('owner/dataset');

-- Check cache info (now includes version)
SELECT gaggle_cache_info();
-- Returns: {..., "version": "5", "is_current": true}
```

### Phase 2: Version Pinning (5-7 days)

**Implement:**
```sql
-- Download specific version
SELECT gaggle_download('owner/dataset@v2');

-- Check if current
SELECT gaggle_is_current('owner/dataset');

-- Get version info
SELECT gaggle_version_info('owner/dataset');
-- Returns: {"cached": "3", "latest": "5"}
```

## Impact Assessment

**Without Versioning Support:**
- 🔴 **HIGH RISK**: Users work with stale data unknowingly
- 🔴 **HIGH RISK**: No reproducibility for research/production
- 🔴 **MEDIUM RISK**: Debugging is difficult (which version caused issue?)

**With Phase 1 (Basic Support):**
- ✅ Cache staleness solved
- ✅ Users know what version they have
- ✅ Can force updates when needed
- ⚠️ Still can't pin to specific versions

**With Phase 2 (Full Support):**
- ✅ Complete version control
- ✅ Full reproducibility
- ✅ Can test against specific versions
- ✅ Production-ready

## Configuration (Proposed)

```bash
# Check for updates on cache hit (default: false for performance)
export GAGGLE_CACHE_REVALIDATE=true

# Revalidation interval in seconds (default: 86400 = 1 day)
export GAGGLE_CACHE_REVALIDATE_INTERVAL=3600

# Keep multiple versions (default: false to save space)
export GAGGLE_KEEP_ALL_VERSIONS=true
```

## Example Use Cases

### Use Case 1: Research Reproducibility
**Current Problem:**
```
Paper: "We used kaggle:owner/dataset for our analysis"
Reader: Downloads different version, gets different results
```

**With Versioning:**
```sql
-- Paper specifies exact version
SELECT * FROM 'kaggle:owner/dataset@v3';

-- Readers get identical data
```

### Use Case 2: Production Data Pipeline
**Current Problem:**
```sql
-- Daily pipeline
SELECT * FROM 'kaggle:owner/dataset';
-- Sometimes gets stale data, sometimes new data
-- Unpredictable results
```

**With Versioning:**
```sql
-- Option A: Always latest (with update check)
SELECT * FROM 'kaggle:owner/dataset@latest';

-- Option B: Pinned version (stable)
SELECT * FROM 'kaggle:owner/dataset@v5';

-- Option C: Check and update
SELECT gaggle_update_dataset('owner/dataset');
SELECT * FROM 'kaggle:owner/dataset';
```

### Use Case 3: Dataset Migration
**Current Problem:**
```
V1 has bug, V2 fixes it
Users with cached V1 don't know about V2
```

**With Versioning:**
```sql
-- Check version
SELECT gaggle_version_info('owner/dataset');
-- Shows: "cached v1, latest v2"

-- Update
SELECT gaggle_update_dataset('owner/dataset');
```

## Backward Compatibility

✅ **All changes are backward compatible:**

```sql
-- Existing code continues to work
SELECT gaggle_download('owner/dataset');  
-- Behavior: Downloads latest (same as now)
-- New: Stores version, checks for updates (configurable)
```

## Documentation Created

- ✅ **`docs/VERSIONING_ANALYSIS.md`** - Complete technical analysis
- ✅ **ROADMAP.md** - Added versioning features to roadmap

## Next Steps

**Immediate Priority:**
1. Review versioning analysis document
2. Decide on implementation approach
3. Implement Phase 1 (cache staleness fix)
4. Update documentation

**Timeline:**
- Phase 1: 2-3 days
- Phase 2: 5-7 days
- Total: ~1-2 weeks for complete versioning support

## Recommendation

**⚠️ IMPLEMENT PHASE 1 IMMEDIATELY**

Cache staleness is a critical issue that affects data correctness. Users have no way to know if their cached data is outdated. This should be addressed before releasing to production.

---

**For detailed technical analysis, see:** `docs/VERSIONING_ANALYSIS.md`
