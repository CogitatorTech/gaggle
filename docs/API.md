# Gaggle API Reference

## Overview

This document provides detailed information about all functions available in the Gaggle extension.

## Credential Management

### `gaggle_set_credentials`

Set Kaggle API credentials for the current session.

**Signature:**
```sql
gaggle_set_credentials(username VARCHAR, key VARCHAR) → BOOLEAN
```

**Parameters:**
- `username` (VARCHAR): Your Kaggle username
- `key` (VARCHAR): Your Kaggle API key

**Returns:**
- `true` on success
- Error on failure

**Example:**
```sql
SELECT gaggle_set_credentials('myusername', 'abc123xyz789');
-- Returns: true
```

**Notes:**
- Credentials are stored in memory for the session
- Alternative: Use `KAGGLE_USERNAME` and `KAGGLE_KEY` environment variables
- Alternative: Create `~/.kaggle/kaggle.json` file

---

## Dataset Operations

### `gaggle_download`

Download a Kaggle dataset and return the local cache path.

**Signature:**
```sql
gaggle_download(dataset_path VARCHAR) → VARCHAR
```

**Parameters:**
- `dataset_path` (VARCHAR): Dataset path in format `owner/dataset-name`

**Returns:**
- Local filesystem path to the downloaded dataset directory

**Example:**
```sql
SELECT gaggle_download('owid/covid-latest-data');
-- Returns: /home/user/.cache/gaggle_cache/datasets/owid/covid-latest-data
```

**Notes:**
- Downloads are cached locally
- Subsequent calls return cached path without re-downloading
- Downloads are extracted from ZIP format automatically

---

### `gaggle_list_files`

List all files in a downloaded dataset.

**Signature:**
```sql
gaggle_list_files(dataset_path VARCHAR) → VARCHAR
```

**Parameters:**
- `dataset_path` (VARCHAR): Dataset path in format `owner/dataset-name`

**Returns:**
- JSON array of file objects with `name` and `size` fields

**Example:**
```sql
SELECT * FROM json_each(gaggle_list_files('heptapod/titanic'));

-- Output:
-- {
--   "name": "train.csv",
--   "size": 61194
-- },
-- {
--   "name": "test.csv",
--   "size": 28629
-- }
```

**Usage Pattern:**
```sql
-- Extract file names
SELECT json_extract_string(value, '$.name') as filename,
       json_extract_string(value, '$.size') as size_bytes
FROM json_each(gaggle_list_files('heptapod/titanic'));
```

---

### `gaggle_info`

Get metadata and information about a Kaggle dataset.

**Signature:**
```sql
gaggle_info(dataset_path VARCHAR) → VARCHAR
```

**Parameters:**
- `dataset_path` (VARCHAR): Dataset path in format `owner/dataset-name`

**Returns:**
- JSON object with dataset metadata

**Example:**
```sql
SELECT * FROM json_each(gaggle_info('owid/covid-latest-data'));

-- Returns metadata including:
-- - title
-- - description
-- - size
-- - lastUpdated
-- - downloadCount
-- - etc.
```

---

### `gaggle_search`

Search for datasets on Kaggle.

**Signature:**
```sql
gaggle_search(query VARCHAR, page INTEGER, page_size INTEGER) → VARCHAR
```

**Parameters:**
- `query` (VARCHAR): Search query string
- `page` (INTEGER): Page number (1-indexed)
- `page_size` (INTEGER): Number of results per page (max 100)

**Returns:**
- JSON array of dataset objects

**Example:**
```sql
SELECT * FROM json_each(gaggle_search('covid-19', 1, 20));

-- Each result contains:
-- - ref: "owner/dataset-name"
-- - title: "Dataset Title"
-- - size: size in bytes
-- - lastUpdated: timestamp
-- - downloadCount: number
-- - etc.
```

**Usage Pattern:**
```sql
-- Find COVID datasets and show titles
SELECT 
    json_extract_string(value, '$.ref') as dataset_path,
    json_extract_string(value, '$.title') as title,
    json_extract_string(value, '$.size') as size
FROM json_each(gaggle_search('covid', 1, 10))
ORDER BY json_extract_string(value, '$.downloadCount') DESC;
```

---

## Cache Management

### `gaggle_clear_cache`

Clear the entire local dataset cache.

**Signature:**
```sql
gaggle_clear_cache() → BOOLEAN
```

**Parameters:**
- None

**Returns:**
- `true` on success
- Error on failure

**Example:**
```sql
SELECT gaggle_clear_cache();
-- Returns: true
```

**Notes:**
- Deletes all cached datasets
- Free up disk space
- Next download will re-fetch from Kaggle

---

### `gaggle_get_cache_info`

Get information about the current cache.

**Signature:**
```sql
gaggle_get_cache_info() → VARCHAR
```

**Parameters:**
- None

**Returns:**
- JSON object with cache statistics

**Example:**
```sql
SELECT * FROM json_each(gaggle_get_cache_info());

-- Returns:
-- {
--   "cache_dir": "/home/user/.cache/gaggle_cache",
--   "size_bytes": 1048576,
--   "size_mb": 1
-- }
```

---

## Utility Functions

### `gaggle_get_version`

Get version and build information for the extension.

**Signature:**
```sql
gaggle_get_version() → VARCHAR
```

**Parameters:**
- None

**Returns:**
- JSON object with version information

**Example:**
```sql
SELECT * FROM json_each(gaggle_get_version());

-- Returns:
-- {
--   "version": "0.1.0",
--   "name": "Gaggle - Kaggle Dataset DuckDB Extension"
-- }
```

---

## Complete Usage Examples

### Example 1: Download and Query CSV

```sql
-- Load extension
LOAD gaggle;

-- Set credentials (if not using env vars or config file)
SELECT gaggle_set_credentials('username', 'api-key');

-- Download dataset
SELECT gaggle_download('heptapod/titanic');

-- List available files
SELECT * FROM json_each(gaggle_list_files('heptapod/titanic'));

-- Read CSV file
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('heptapod/titanic') || '/train.csv')
) LIMIT 10;
```

### Example 2: Search and Explore

```sql
-- Search for datasets
SELECT 
    json_extract_string(value, '$.ref') as dataset,
    json_extract_string(value, '$.title') as title,
    CAST(json_extract_string(value, '$.downloadCount') AS INTEGER) as downloads
FROM json_each(gaggle_search('machine learning', 1, 20))
ORDER BY downloads DESC
LIMIT 5;

-- Get info about a specific dataset
SELECT * FROM json_each(
    gaggle_info('userid/dataset-name')
);
```

### Example 3: Create Reusable Views

```sql
-- Download once
SELECT gaggle_download('owid/covid-latest-data');

-- Create view for easy access
CREATE VIEW covid AS
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('owid/covid-latest-data') || '/owid-covid-latest.csv')
);

-- Query like a regular table
SELECT location, MAX(total_cases) as max_cases
FROM covid
WHERE continent = 'North America'
GROUP BY location
ORDER BY max_cases DESC;
```

### Example 4: Join Multiple Datasets

```sql
-- Download multiple datasets
SELECT gaggle_download('dataset1/name');
SELECT gaggle_download('dataset2/name');

-- Create views
CREATE VIEW data1 AS SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('dataset1/name') || '/file1.csv')
);

CREATE VIEW data2 AS SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('dataset2/name') || '/file2.csv')
);

-- Join them
SELECT d1.*, d2.extra_column
FROM data1 d1
INNER JOIN data2 d2 ON d1.id = d2.id;
```

## Error Handling

All functions may raise errors in the following scenarios:

### Common Errors

**"No Kaggle credentials found"**
- Cause: Credentials not set
- Solution: Use `gaggle_set_credentials()`, set env vars, or create `~/.kaggle/kaggle.json`

**"Dataset not found"**
- Cause: Invalid dataset path or no access
- Solution: Verify dataset exists on Kaggle and you have access

**"HTTP request failed: 403"**
- Cause: Invalid credentials or rate limit
- Solution: Check credentials, wait if rate limited

**"Failed to download dataset"**
- Cause: Network issues or Kaggle API problems
- Solution: Check internet connection, try again later

**"Invalid dataset path"**
- Cause: Incorrect path format
- Solution: Use format `owner/dataset-name`

## Performance Considerations

1. **Caching**: First download is slow, subsequent access is fast
2. **Large Datasets**: Downloads may take time, consider network speed
3. **CSV vs Parquet**: Prefer Parquet files when available for better performance
4. **Filters**: Use WHERE clauses to read only needed data from CSV files
5. **Views**: Create views for frequently accessed datasets

## Security Notes

- Never hardcode credentials in SQL files
- Use environment variables or config files
- Set proper permissions on `kaggle.json` (chmod 600)
- Rotate API keys regularly
- Be mindful of dataset licenses and terms of use

## API Rate Limits

Kaggle API has rate limits:
- Requests per hour: varies by account type
- Large dataset downloads may count multiple times
- Monitor usage through Kaggle account settings

## Additional Resources

- Kaggle API Documentation: https://www.kaggle.com/docs/api
- Gaggle User Guide: [GAGGLE_GUIDE.md](GAGGLE_GUIDE.md)
- Implementation Details: [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)
- GitHub Repository: https://github.com/CogitatorTech/gaggle

