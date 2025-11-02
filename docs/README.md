### API Reference

The table below includes the information about all SQL functions exposed by Gaggle.

| #  | Function                                                        | Return Type      | Description                                                                                                               |
|----|:----------------------------------------------------------------|:-----------------|:--------------------------------------------------------------------------------------------------------------------------|
| 1  | `gaggle_set_credentials(username VARCHAR, key VARCHAR)`         | `BOOLEAN`        | Sets Kaggle API credentials from SQL (alternatively use env vars or `~/.kaggle/kaggle.json`). Returns `true` on success.  |
| 2  | `gaggle_download(dataset_path VARCHAR)`                         | `VARCHAR`        | Downloads a Kaggle dataset to the local cache directory and returns the local dataset path. Idempotent.                   |
| 3  | `gaggle_search(query VARCHAR, page INTEGER, page_size INTEGER)` | `VARCHAR (JSON)` | Searches Kaggle datasets and returns a JSON array. Constraints: `page >= 1`, `1 <= page_size <= 100`.                     |
| 4  | `gaggle_info(dataset_path VARCHAR)`                             | `VARCHAR (JSON)` | Returns metadata for a dataset as JSON (for example: title, url, last_updated).                                           |
| 5  | `gaggle_version()`                                              | `VARCHAR`        | Returns the extension version string (for example: `"0.1.0"`).                                                            |
| 6  | `gaggle_clear_cache()`                                          | `BOOLEAN`        | Clears the dataset cache directory. Returns `true` on success.                                                            |
| 7  | `gaggle_cache_info()`                                           | `VARCHAR (JSON)` | Returns cache info JSON with `path`, `size_mb`, `limit_mb`, `usage_percent`, `is_soft_limit`, and `type` fields.          |
| 8  | `gaggle_enforce_cache_limit()`                                  | `BOOLEAN`        | Manually enforces cache size limit using LRU eviction. Returns `true` on success. (Automatic with soft limit by default). |
| 9  | `gaggle_is_current(dataset_path VARCHAR)`                       | `BOOLEAN`        | Checks if cached dataset is the latest version from Kaggle. Returns `false` if not cached or outdated.                    |
| 10 | `gaggle_update_dataset(dataset_path VARCHAR)`                   | `VARCHAR`        | Forces update to latest version (ignores cache). Returns local path to freshly downloaded dataset.                        |
| 11 | `gaggle_version_info(dataset_path VARCHAR)`                     | `VARCHAR (JSON)` | Returns version info: `cached_version`, `latest_version`, `is_current`, `is_cached`.                                      |
| 12 | `gaggle_json_each(json VARCHAR)`                                | `VARCHAR`        | Expands a JSON object/array into newline-delimited JSON rows with fields: `key`, `value`, `type`, `path`.                 |
| 13 | `gaggle_file_path(dataset_path VARCHAR, filename VARCHAR)`      | `VARCHAR`        | Resolves a specific file's local path inside a downloaded dataset.                                                        |

> [!NOTE]
> Dataset paths must be in the form `owner/dataset` where `owner` is the username and `dataset` is the dataset name on
> Kaggle.
> For example: `habedi/flickr-8k-dataset-clean`.
> You can also read files directly using the replacement scan with the `kaggle:` scheme.
> For example: `'kaggle:habedi/flickr-8k-dataset-clean/flickr8k.parquet`.

Table function:

| #  | Function                          | Return Type                                      | Description                                                                    |
|----|:----------------------------------|:-------------------------------------------------|:-------------------------------------------------------------------------------|
| 14 | `gaggle_ls(dataset_path VARCHAR)` | `TABLE(name VARCHAR, size BIGINT, path VARCHAR)` | Lists files (non-recursive) in the dataset's local directory; `size` is in MB. |

Replacement scan (transparent table read):

- Single file: `'kaggle:owner/dataset/file.ext'`
- Glob: `'kaggle:owner/dataset/*.ext'`
- Reader is chosen by extension:
    - `.parquet`/`.parq` -> `read_parquet`
    - `.json`/`.jsonl`/`.ndjson` -> `read_json_auto`
    - `.xlsx` -> `read_excel` (requires DuckDB to be built with the Excel reader)
    - everything else -> `read_csv_auto`

---

### Usage Examples

#### Dataset Management

```sql
-- Load the Gaggle extension
load
'build/release/extension/gaggle/gaggle.duckdb_extension';

-- Set Kaggle credentials (or rely on env vars / ~/.kaggle/kaggle.json)
select gaggle_set_credentials('your-username', 'your-api-key');

-- Check version
select gaggle_version();

-- Search datasets (JSON string)
select gaggle_search('iris', 1, 5);
-- Disabled in offline mode (GAGGLE_OFFLINE=1)

-- Download a dataset and get its local path
select gaggle_download('uciml/iris') as local_path;

-- Get dataset metadata (JSON)
select gaggle_info('uciml/iris') as dataset_metadata; -- Disabled in offline mode (GAGGLE_OFFLINE=1)
```

#### Reading Data

```sql
-- List files as a table
select *
from gaggle_ls('uciml/iris') limit 5;

-- List files as a JSON array
select to_json(list(struct_pack(name := name, size := size, path := path))) as files_json
from gaggle_ls('uciml/iris');

-- Resolve a file path and read it via a prepared statement
prepare rp as select * from read_parquet(?) limit 10;
execute rp(gaggle_file_path('owner/dataset', 'file.parquet'));
```

```sql
-- Replacement scan: read a single Parquet file via Kaggle: URL
select count(*)
from 'kaggle:owner/dataset/file.parquet';

-- Replacement scan: glob Parquet files in a dataset directory
select count(*)
from 'kaggle:owner/dataset/*.parquet';
```

#### Dataset Versioning

```sql
-- Check if cached dataset is the latest version
select gaggle_is_current('owner/dataset') as is_current;

-- Get detailed version information
select gaggle_version_info('owner/dataset') as version_info;
-- Returns: {"cached_version": "3", "latest_version": "5", "is_current": false, "is_cached": true}

-- Force update to latest version (ignores cache)
select gaggle_update_dataset('owner/dataset') as updated_path;

-- Download specific version (version pinning)
select gaggle_download('owner/dataset@v2'); -- Version 2
select gaggle_download('owner/dataset@5'); -- Version 5 (without 'v' prefix)
select gaggle_download('owner/dataset@latest');
-- Explicit latest

-- Use versioned datasets in queries
select *
from 'kaggle:owner/dataset@v2/file.csv';
select *
from 'kaggle:owner/dataset@v5/*.parquet';

-- Smart download: update only if outdated
select CASE
           WHEN gaggle_is_current('owner/dataset') THEN gaggle_download('owner/dataset')
           ELSE gaggle_update_dataset('owner/dataset')
           END as path;
```

#### Utility Functions

```sql
-- Purge cache and inspect info
select gaggle_clear_cache();
select gaggle_cache_info();

-- Manually enforce cache size limit (LRU eviction of oldest datasets)
select gaggle_enforce_cache_limit();

-- Expand JSON into newline-delimited rows
select gaggle_json_each('{"a":1,"b":[true,{"c":"x"}]}') as rows;

-- Parse search results as JSON and extract a couple of fields
with s as (select from_json(gaggle_search('iris', 1, 10)) as j)
select json_extract_string(value, '$.ref')   as ref,
       json_extract_string(value, '$.title') as title
from json_each((select j from s)) limit 5;
```

> [!IMPORTANT]
> When you use a dataset, you read and process its files locally. Only use datasets from trusted sources.

---

### Building Gaggle from Source

To build Gaggle from source, you need GNU Make, CMake, a C++ compiler (GCC/Clang), Rust and Cargo.

1. **Clone the repository:**
   ```bash
   git clone --recursive https://github.com/CogitatorTech/gaggle.git
   cd gaggle
   ```
   > [!NOTE]
   > The `--recursive` flag ensures required submodules (like DuckDB) are cloned.

2. **Build the extension:**
   ```bash
   make release
   ```
   This will create a `duckdb` executable inside `build/release/` and a loadable extension at
   `build/release/extension/gaggle/gaggle.duckdb_extension`.

3. **Run the custom DuckDB shell:**
   ```bash
   ./build/release/duckdb
   ```
   You can load the extension with:
   ```sql
   load 'build/release/extension/gaggle/gaggle.duckdb_extension';
   ```

---

### Configuration

See [CONFIGURATION.md](CONFIGURATION.md) for full details. Key environment variables:

- `GAGGLE_CACHE_DIR` — cache directory path (default: `~/.cache/gaggle`)
- `GAGGLE_HTTP_TIMEOUT` — HTTP timeout (in seconds)
- `GAGGLE_HTTP_RETRY_ATTEMPTS` — retry attempts after the initial try
- `GAGGLE_HTTP_RETRY_DELAY` — initial backoff delay (in seconds)
- `GAGGLE_HTTP_RETRY_MAX_DELAY` — maximum backoff delay cap (in seconds)
- `GAGGLE_LOG_LEVEL` — structured log level for the Rust core (e.g., `INFO`, `DEBUG`)
- `GAGGLE_OFFLINE` — disable network; only use cached data (downloads fail fast if not cached)
- `KAGGLE_USERNAME` and `KAGGLE_KEY` — Kaggle credentials (alternative to the SQL call)

> [!NOTE]
> Environment variables are case-sensitive on Unix-like systems. Changes take effect for subsequent operations in the
> same process.

#### Units

- Storage sizes are reported in megabytes (MB) across SQL/API (for example: `gaggle_cache_info()` returns `size_mb`).
- Timeouts and retry delays are configured in seconds (via clean environment variables without unit suffixes).

### Replacement Scan Readers

Gaggle selects the DuckDB reader based on file extension:

- `.parquet`/`.parq` -> `read_parquet`
- `.json`/`.jsonl`/`.ndjson` -> `read_json_auto`
- `.xlsx` -> `read_excel` (requires DuckDB to be built with the Excel reader)
- everything else -> `read_csv_auto`

---

### Architecture

Gaggle is made up of two main components:

1. **Rust Core (`gaggle/src/`)** that handles:
    - Credentials management
    - HTTP client with timeout and exponential backoff
    - Dataset download with safe ZIP extraction and file resolution
    - Search and metadata requests
    - C-compatible FFI surface

2. **C++ DuckDB Bindings (`gaggle/bindings/`)** that:
    - Defines the custom SQL functions (for example: `gaggle_ls`, `gaggle_file_path`, `gaggle_search`)
    - Integrates with DuckDB’s extension system and replacement scans (`'kaggle:...'`)
    - Marshals values between DuckDB vectors and the Rust FFI
