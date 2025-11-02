# Gaggle Error Codes Reference

**Version:** 0.1.0  
**Date:** November 2, 2025

## Overview

Gaggle uses standardized error codes to make error handling more predictable and debugging easier. Each error includes a
numeric code (E001-E010) that can be used programmatically.

## Error Code Format

All errors follow this format:

```
[Exxx] Error description: additional details
```

Example:

```
[E002] Dataset not found: owner/invalid-dataset
```

## Error Codes

### E001 - Invalid Credentials

**Category:** Authentication  
**Code:** `E001`  
**Type:** `CredentialsError`

**Description:**  
Kaggle API credentials are invalid, missing, or incorrectly formatted.

**Common Causes:**

- Wrong username or API key
- Missing credentials (no environment variables or kaggle.json)
- Expired API key
- Incorrectly formatted kaggle.json file

**Example:**

```
[E001] Invalid Kaggle credentials: Username or API key not found
```

**Solutions:**

1. **Set credentials via SQL:**
   ```sql
   SELECT gaggle_set_credentials('your-username', 'your-api-key');
   ```

2. **Set environment variables:**
   ```bash
   export KAGGLE_USERNAME=your-username
   export KAGGLE_KEY=your-api-key
   ```

3. **Create kaggle.json file:**
   ```bash
   mkdir -p ~/.kaggle
   cat > ~/.kaggle/kaggle.json << EOF
   {
     "username": "your-username",
     "key": "your-api-key"
   }
   EOF
   chmod 600 ~/.kaggle/kaggle.json
   ```

4. **Get your API key from Kaggle:**
    - Go to https://www.kaggle.com/settings/account
    - Click "Create New API Token"
    - Download kaggle.json

---

### E002 - Dataset Not Found

**Category:** Dataset  
**Code:** `E002`  
**Type:** `DatasetNotFound`

**Description:**  
The requested dataset does not exist on Kaggle or is not accessible.

**Common Causes:**

- Typo in dataset path
- Dataset was deleted or made private
- Wrong owner name
- Dataset requires special permissions

**Example:**

```
[E002] Dataset not found: owner/nonexistent-dataset
```

**Solutions:**

1. **Verify dataset path on Kaggle:**
    - Visit https://www.kaggle.com/datasets/owner/dataset-name
    - Check spelling and owner name

2. **Search for the dataset:**
   ```sql
   SELECT gaggle_search('dataset keywords', 1, 10);
   ```

3. **Check dataset availability:**
    - Ensure dataset is public
    - Verify you have access rights

---

### E003 - Network Error

**Category:** Network  
**Code:** `E003`  
**Type:** `HttpRequestError`

**Description:**  
Network error occurred during communication with Kaggle API.

**Common Causes:**

- No internet connection
- Kaggle API is down
- Firewall blocking requests
- Timeout
- Rate limiting

**Example:**

```
[E003] HTTP request failed: Connection timeout after 30s
```

**Solutions:**

1. **Check internet connection:**
   ```bash
   ping www.kaggle.com
   ```

2. **Increase timeout:**
   ```bash
   export GAGGLE_HTTP_TIMEOUT=120  # 2 minutes
   ```

3. **Check Kaggle API status:**
    - Visit https://www.kaggle.com
    - Check https://status.kaggle.com (if available)

4. **Retry with backoff:**
   ```bash
   export GAGGLE_HTTP_RETRY_ATTEMPTS=5
   export GAGGLE_HTTP_RETRY_DELAY=2
   export GAGGLE_HTTP_RETRY_MAX_DELAY=30
   ```

5. **Check firewall settings:**
    - Ensure outbound HTTPS (port 443) is allowed
    - Check corporate proxy settings

---

### E004 - Invalid Path

**Category:** Validation  
**Code:** `E004`  
**Type:** `InvalidDatasetPath`

**Description:**  
Dataset path format is invalid or contains forbidden characters.

**Common Causes:**

- Missing slash in path
- Path traversal attempts (../)
- Too many path components
- Control characters in path
- Path too long (>4096 characters)

**Example:**

```
[E004] Invalid dataset path: Must be in format 'owner/dataset-name'
```

**Valid Path Format:**

```
owner/dataset-name
owner/dataset-name@v2  (with version)
```

**Invalid Paths:**

```
ownerdataset         # Missing slash
owner/dataset/extra  # Too many components
../dataset           # Path traversal
owner/.              # Dot component
```

**Solutions:**

1. **Use correct format:**
   ```sql
   SELECT gaggle_download('owner/dataset-name');
   ```

2. **Check for special characters:**
    - Avoid: `..`, `.`, control characters
    - Allowed: letters, numbers, hyphens, underscores

---

### E005 - File System Error

**Category:** I/O  
**Code:** `E005`  
**Type:** `IoError`

**Description:**  
Error reading from or writing to the file system.

**Common Causes:**

- Insufficient disk space
- Permission denied
- File not found
- Directory not writable
- Disk full

**Example:**

```
[E005] IO error: Permission denied (os error 13)
```

**Solutions:**

1. **Check disk space:**
   ```bash
   df -h
   ```

2. **Check permissions:**
   ```bash
   ls -la ~/.cache/gaggle_cache
   chmod -R u+rw ~/.cache/gaggle_cache
   ```

3. **Verify cache directory:**
   ```sql
   SELECT gaggle_cache_info();
   ```

4. **Change cache directory:**
   ```bash
   export GAGGLE_CACHE_DIR=/path/with/space
   ```

5. **Clean up cache:**
   ```sql
   SELECT gaggle_clear_cache();
   ```

---

### E006 - JSON Error

**Category:** Serialization  
**Code:** `E006`  
**Type:** `JsonError`

**Description:**  
Error parsing or serializing JSON data.

**Common Causes:**

- Corrupted cache metadata
- Invalid JSON response from Kaggle API
- Encoding issues
- Malformed JSON

**Example:**

```
[E006] JSON serialization error: expected `,` or `}` at line 5 column 10
```

**Solutions:**

1. **Clear cache:**
   ```sql
   SELECT gaggle_clear_cache();
   ```

2. **Re-download dataset:**
   ```sql
   SELECT gaggle_update_dataset('owner/dataset');
   ```

3. **Check Kaggle API response manually:**
   ```bash
   curl -u username:key https://www.kaggle.com/api/v1/datasets/view/owner/dataset
   ```

---

### E007 - ZIP Extraction Error

**Category:** Archive  
**Code:** `E007`  
**Type:** `ZipError`

**Description:**  
Error extracting downloaded ZIP file.

**Common Causes:**

- Corrupted download
- ZIP bomb protection triggered (>10GB uncompressed)
- Path traversal in ZIP
- Symlinks in ZIP
- Invalid ZIP format

**Example:**

```
[E007] ZIP extraction failed: ZIP file too large (exceeds 10GB)
```

**Solutions:**

1. **Re-download dataset:**
   ```sql
   SELECT gaggle_update_dataset('owner/dataset');
   ```

2. **Check dataset size:**
   ```sql
   SELECT gaggle_info('owner/dataset');
   ```

3. **For large datasets:**
    - Note: 10GB uncompressed limit is a security feature
    - Consider using a different dataset or smaller subset

4. **Check ZIP integrity:**
   ```bash
   unzip -t /path/to/dataset.zip
   ```

---

### E008 - CSV Parsing Error

**Category:** Parsing  
**Code:** `E008`  
**Type:** `CsvError`

**Description:**  
Error parsing CSV file format.

**Common Causes:**

- Malformed CSV
- Inconsistent column count
- Invalid quotes or delimiters
- Encoding issues

**Example:**

```
[E008] CSV parsing error: record 145 has different field count
```

**Solutions:**

1. **Check CSV format:**
   ```bash
   head -20 /path/to/file.csv
   ```

2. **Use DuckDB's flexible CSV reader:**
   ```sql
   SELECT * FROM read_csv_auto('kaggle:owner/dataset/file.csv',
                                ignore_errors := true);
   ```

3. **Try different parser options:**
   ```sql
   SELECT * FROM read_csv('kaggle:owner/dataset/file.csv',
                          delim := ';',
                          quote := '"',
                          escape := '\\');
   ```

---

### E009 - UTF-8 Encoding Error

**Category:** Encoding  
**Code:** `E009`  
**Type:** `Utf8Error`

**Description:**  
String is not valid UTF-8.

**Common Causes:**

- Binary data in string field
- Wrong character encoding
- Corrupted data
- FFI boundary issues

**Example:**

```
[E009] Invalid UTF-8 string
```

**Solutions:**

1. **Check file encoding:**
   ```bash
   file -i /path/to/file.csv
   ```

2. **Convert to UTF-8:**
   ```bash
   iconv -f ISO-8859-1 -t UTF-8 input.csv > output.csv
   ```

3. **Use DuckDB encoding options:**
   ```sql
   SELECT * FROM read_csv('file.csv', encoding := 'ISO-8859-1');
   ```

---

### E010 - Null Pointer Error

**Category:** FFI  
**Code:** `E010`  
**Type:** `NullPointer`

**Description:**  
NULL pointer passed to FFI function.

**Common Causes:**

- Internal programming error
- Invalid function call
- Memory corruption

**Example:**

```
[E010] Null pointer passed
```

**Solutions:**

- This is typically an internal error
- Report as a bug if you encounter this
- Include reproduction steps

---

## Programmatic Error Handling

### In Rust

```rust
use gaggle::error::{GaggleError, ErrorCode};

match gaggle::download_dataset("owner/dataset") {
Ok(path) => println ! ("Downloaded to: {:?}", path),
Err(e) => {
match e.code() {
ErrorCode::E001_InvalidCredentials => {
// Handle authentication error
eprintln ! ("Please set Kaggle credentials");
}
ErrorCode::E002_DatasetNotFound => {
// Handle missing dataset
eprintln ! ("Dataset not found, trying alternative...");
}
ErrorCode::E003_NetworkError => {
// Handle network error
eprintln ! ("Network error, retrying...");
}
_ => {
// Handle other errors
eprintln ! ("Error: {}", e);
}
}
}
}
```

### In SQL

```sql
-- Check last error after failure
SELECT gaggle_download('owner/invalid'); -- This fails
SELECT gaggle_last_error();
-- Get error message with code

-- Example output:
-- "[E002] Dataset not found: owner/invalid"
```

### Parsing Error Codes in Application

```python
# Python example
error_msg = execute_sql("SELECT gaggle_last_error()")

if "[E001]" in error_msg:
    # Handle credentials error
    setup_credentials()
elif "[E002]" in error_msg:
    # Handle dataset not found
    search_alternative_dataset()
elif "[E003]" in error_msg:
    # Handle network error
    retry_with_backoff()
```

## Error Recovery Strategies

### Transient Errors (Retry)

- **E003** - Network errors (automatic retry with backoff)
- **E006** - JSON errors (may be temporary API issue)

### Configuration Errors (User Action Required)

- **E001** - Invalid credentials
- **E004** - Invalid path format

### Resource Errors (Check System)

- **E005** - I/O errors (disk space, permissions)
- **E007** - ZIP errors (space, corruption)

### Data Errors (Check Dataset)

- **E002** - Dataset not found
- **E008** - CSV parsing errors

## Best Practices

1. **Always check error codes in production:**
   ```sql
   SELECT CASE
       WHEN gaggle_is_current('owner/dataset') THEN 'OK'
       ELSE gaggle_last_error()
   END;
   ```

2. **Log errors with codes:**
    - Include error code in logs
    - Helps with debugging and monitoring

3. **Implement retry logic for transient errors:**
    - E003 (Network) - retry with exponential backoff
    - E006 (JSON) - retry once or twice

4. **Alert on specific error codes:**
    - E001 (Credentials) - immediate alert
    - E002 (Not Found) - dataset issue alert

5. **Document error codes in your application:**
    - Link to this reference
    - Provide context-specific solutions

## Changelog

### Version 0.1.0 (November 2, 2025)

- Initial error code implementation
- 10 error codes defined (E001-E010)
- All error messages updated with codes

## See Also

- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Troubleshooting guide
- [FAQ.md](FAQ.md) - Frequently asked questions
- [CONFIGURATION.md](CONFIGURATION.md) - Configuration options
- [README.md](../README.md) - Main documentation
