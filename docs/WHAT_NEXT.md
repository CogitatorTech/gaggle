# What's Next? - Feature Prioritization

**Date:** November 2, 2025  
**Current Status:** Version Pinning Complete ✅  
**Total Tests:** 165 passing ✅

## Recently Completed

✅ **Cache Size Limit** (100GB default, LRU eviction)  
✅ **Dataset Versioning** (version awareness, updates, checking)  
✅ **Version Pinning** (`@vN` syntax for reproducibility)

## Roadmap Analysis

### High Priority Features (Implement Next)

#### 1. **Excel/XLSX Support** ⭐⭐⭐⭐⭐
**Why High Priority:**
- Many Kaggle datasets include Excel files
- Currently unsupported, limiting dataset accessibility
- Relatively straightforward to implement
- High user value

**Implementation Complexity:** Medium (3-4 days)

**Requirements:**
- Add `calamine` or `xlsx` crate for Excel parsing
- Integrate with replacement scan
- Support both `.xls` and `.xlsx` formats
- Handle multiple sheets

**Files to Create/Modify:**
- `gaggle/Cargo.toml` - Add Excel library
- `gaggle/src/kaggle/excel.rs` - Excel parsing logic
- `gaggle/bindings/gaggle_extension.cpp` - Update replacement scan
- Tests and documentation

**Estimated Effort:** 3-4 days

---

#### 2. **Detailed Error Codes** ⭐⭐⭐⭐
**Why High Priority:**
- Better programmatic error handling
- Easier debugging for users
- Professional quality enhancement
- Relatively quick to implement

**Implementation Complexity:** Low-Medium (1-2 days)

**Requirements:**
- Define error code enum (e.g., `E001`, `E002`)
- Update `GaggleError` to include error codes
- Update all error messages with codes
- Document error codes

**Example:**
```rust
pub enum ErrorCode {
    E001_InvalidCredentials,
    E002_DatasetNotFound,
    E003_NetworkError,
    // ...
}
```

**Estimated Effort:** 1-2 days

---

#### 3. **Tutorial Documentation** ⭐⭐⭐⭐
**Why High Priority:**
- Improves user onboarding
- Reduces support burden
- Professional documentation quality
- Quick to implement

**Implementation Complexity:** Low (1-2 days)

**Content Needed:**
- Getting started tutorial
- Common use cases (5-10 examples)
- Troubleshooting guide
- FAQ section
- Best practices guide

**Estimated Effort:** 1-2 days

---

### Medium Priority Features

#### 4. **FAQ Section** ⭐⭐⭐
**Implementation:** 1 day
- Common questions and answers
- Performance tips
- Troubleshooting common issues

#### 5. **Troubleshooting Guide** ⭐⭐⭐
**Implementation:** 1 day
- Network issues
- Authentication problems
- Cache issues
- Performance problems

#### 6. **End-to-End Integration Tests** ⭐⭐⭐
**Implementation:** 2-3 days
- Comprehensive test suite with HTTP mocking
- Test all SQL functions
- Test edge cases and error conditions

---

### Lower Priority Features

#### 7. **Upload DuckDB Tables to Kaggle** ⭐⭐
**Why Lower Priority:**
- Complex (requires Kaggle API research)
- Less commonly needed
- Workaround exists (export + kaggle CLI)

**Implementation:** 7-10 days

#### 8. **Cloud Storage Backends** ⭐⭐
**Why Lower Priority:**
- Complex integration
- Not critical for core functionality
- Can be added later as enhancement

**Implementation:** 10-15 days

#### 9. **Virtual Table Support** ⭐⭐
**Why Lower Priority:**
- Complex DuckDB integration
- Replacement scan already works well
- Advanced feature

**Implementation:** 5-7 days

---

## Recommended Next Steps

### Option A: High-Value Quick Wins (Recommended)
**Timeline: 1 week**

1. **Detailed Error Codes** (1-2 days)
   - Quick to implement
   - High professional value
   - Better UX

2. **Tutorial Documentation** (1-2 days)
   - Quick to create
   - Huge user benefit
   - Professional polish

3. **Excel/XLSX Support** (3-4 days)
   - High user value
   - Opens up many datasets
   - Medium complexity

**Total: ~7 days, 3 major features**

---

### Option B: Focus on One Big Feature
**Timeline: 3-4 days**

**Excel/XLSX Support Only**
- Deep dive into Excel integration
- Comprehensive testing
- Full documentation
- Multiple sheet support
- Format detection

---

### Option C: Documentation Sprint
**Timeline: 3-4 days**

1. Tutorial documentation (1 day)
2. FAQ section (1 day)
3. Troubleshooting guide (1 day)
4. Best practices guide (1 day)

---

## My Recommendation: Option A

**Implement in this order:**

### Week 1: Quick Wins
1. **Day 1-2: Detailed Error Codes**
   - Define error code enum
   - Update all error messages
   - Document error codes
   - Add tests

2. **Day 3-4: Tutorial Documentation**
   - Getting started tutorial
   - Common use cases
   - FAQ section
   - Troubleshooting guide

3. **Day 5-7: Excel/XLSX Support**
   - Add Excel parsing library
   - Implement Excel reader
   - Update replacement scan
   - Add tests and docs

### Why This Order?

1. **Error codes first** - Quick win, improves everything else
2. **Documentation second** - Makes Excel feature easier to document
3. **Excel third** - Most complex, benefits from improved errors and docs

---

## Feature Comparison Matrix

| Feature | User Value | Complexity | Time | Priority |
|---------|------------|------------|------|----------|
| Error Codes | ⭐⭐⭐⭐ | Low | 1-2 days | High |
| Tutorial Docs | ⭐⭐⭐⭐⭐ | Low | 1-2 days | High |
| Excel Support | ⭐⭐⭐⭐⭐ | Medium | 3-4 days | High |
| FAQ Section | ⭐⭐⭐ | Low | 1 day | Medium |
| Troubleshooting | ⭐⭐⭐ | Low | 1 day | Medium |
| Integration Tests | ⭐⭐⭐ | Medium | 2-3 days | Medium |
| Upload Tables | ⭐⭐ | High | 7-10 days | Low |
| Cloud Storage | ⭐⭐ | High | 10-15 days | Low |
| Virtual Tables | ⭐⭐ | High | 5-7 days | Low |

---

## Implementation Details

### 1. Detailed Error Codes (Day 1-2)

**Step 1: Define Error Codes**
```rust
// gaggle/src/error.rs
pub enum ErrorCode {
    E001, // Invalid credentials
    E002, // Dataset not found
    E003, // Network error
    E004, // Invalid dataset path
    E005, // Cache error
    E006, // ZIP extraction error
    E007, // Version not found
    // ...
}
```

**Step 2: Update Error Messages**
```rust
GaggleError::InvalidCredentials => {
    "[E001] Invalid Kaggle credentials. Check username and API key."
}
```

**Step 3: Documentation**
Create `docs/ERROR_CODES.md` with descriptions and solutions.

---

### 2. Tutorial Documentation (Day 3-4)

**Create:**
- `docs/TUTORIAL.md` - Step-by-step getting started
- `docs/FAQ.md` - Common questions
- `docs/TROUBLESHOOTING.md` - Problem solving
- `docs/BEST_PRACTICES.md` - Performance and usage tips

---

### 3. Excel/XLSX Support (Day 5-7)

**Step 1: Add Dependencies**
```toml
[dependencies]
calamine = "0.24"  # Excel file parsing
```

**Step 2: Implement Excel Reader**
```rust
// gaggle/src/kaggle/excel.rs
pub fn read_excel_to_csv(path: &Path) -> Result<Vec<u8>, GaggleError>
```

**Step 3: Update Replacement Scan**
Detect `.xlsx` and `.xls` extensions, convert to CSV on-the-fly.

---

## Summary

**Recommendation: Implement Option A**

**Week 1 Deliverables:**
1. ✅ Detailed error codes with documentation
2. ✅ Complete tutorial documentation
3. ✅ Excel/XLSX file support

**Benefits:**
- 3 high-value features
- Improved user experience
- Better error handling
- Opens up more datasets
- Professional documentation

**After Week 1:**
- Integration tests
- Upload functionality (if needed)
- Cloud storage (if needed)
- Performance optimizations

---

## Decision Time

**What do you want to implement next?**

**A.** Quick wins (Error codes + Docs + Excel) - 1 week  
**B.** Focus on Excel support only - 3-4 days  
**C.** Documentation sprint - 3-4 days  
**D.** Something else from the roadmap

Let me know and I'll start implementing immediately!

