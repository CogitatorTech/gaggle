## Gaggle's Configuration Guide

Gaggle supports configuration via environment variables to customize its behavior without code changes.

### Environment Variables

#### Cache Configuration

##### GAGGLE_CACHE_DIR

- **Description**: Directory path for caching downloaded Kaggle datasets
- **Type**: String (path)
- **Default**: `$XDG_CACHE_HOME/gaggle` (typically `~/.cache/gaggle`)
- **Example**:
  ```bash
  export GAGGLE_CACHE_DIR="/var/cache/gaggle"
  ```

##### GAGGLE_CACHE_SIZE_LIMIT_MB

- **Description**: Maximum cache size in megabytes for downloaded datasets
- **Type**: Integer (megabytes) or "unlimited"
- **Default**: `102400` (100GB)
- **Status**: ✅ Implemented
- **Behavior**: Uses soft limit by default - downloads complete even if they exceed the limit, then oldest datasets are
  automatically evicted using LRU (Least Recently Used) policy
- **Example**:
  ```bash
  # Set to 50GB
  export GAGGLE_CACHE_SIZE_LIMIT_MB=51200

  # Set to 5GB
  export GAGGLE_CACHE_SIZE_LIMIT_MB=5120

  # Set unlimited cache
  export GAGGLE_CACHE_SIZE_LIMIT_MB=unlimited
  ```

##### GAGGLE_CACHE_HARD_LIMIT

- **Description**: Enable hard limit mode (prevents downloads when cache limit would be exceeded)
- **Type**: Boolean (accepts: true, yes, 1 for hard limit; false, no, 0 for soft limit)
- **Default**: `false` (soft limit)
- **Status**: ✅ Implemented
- **Example**:
  ```bash
  # Enable hard limit (prevents downloads when cache is full)
  export GAGGLE_CACHE_HARD_LIMIT=true
  ```

#### HTTP Configuration

##### GAGGLE_HTTP_TIMEOUT

- **Description**: HTTP request timeout in seconds for Kaggle API requests
- **Type**: Integer (seconds)
- **Default**: `30`
- **Example**:
  ```bash
  export GAGGLE_HTTP_TIMEOUT=120
  ```

##### GAGGLE_API_BASE

- **Description**: Override the Kaggle API base URL (primarily for testing/mocking)
- **Type**: String (URL)
- **Default**: `https://www.kaggle.com/api/v1`
- **Example**:
  ```bash
  # Point requests to a local mock server
  export GAGGLE_API_BASE=http://127.0.0.1:12345
  ```

##### HTTP Retry Controls

- **GAGGLE_HTTP_RETRY_ATTEMPTS**
    - **Description**: Number of retry attempts after the initial try
    - **Type**: Integer
    - **Default**: `3`
- **GAGGLE_HTTP_RETRY_DELAY**
    - **Description**: Initial backoff delay in seconds
    - **Type**: Float or integer (seconds)
    - **Default**: `1`
- **GAGGLE_HTTP_RETRY_MAX_DELAY**
    - **Description**: Maximum backoff delay cap in seconds
    - **Type**: Float or integer (seconds)
    - **Default**: `30`

These controls enable exponential backoff with cap across metadata/search/download requests.

#### Download Coordination

When multiple queries attempt to download the same dataset concurrently, Gaggle coordinates using an in-process lock.
These settings control the wait behavior when a download is already in progress.

- **GAGGLE_DOWNLOAD_WAIT_TIMEOUT**
    - **Description**: Maximum time a waiting request will block (seconds)
    - **Type**: Float or integer (seconds)
    - **Default**: `30`
    - **Example**:
      ```bash
      export GAGGLE_DOWNLOAD_WAIT_TIMEOUT=600 # 10 minutes
      ```
- **GAGGLE_DOWNLOAD_WAIT_POLL**
    - **Description**: Polling interval while waiting (seconds)
    - **Type**: Float or integer (seconds)
    - **Default**: `0.1`

#### Logging Configuration

##### GAGGLE_VERBOSE

- **Description**: Enable verbose logging (boolean)
- **Type**: Boolean (accepts: 1, true, yes, on, 0, false, no, off)
- **Default**: `false`
- **Example**:
  ```bash
  export GAGGLE_VERBOSE=1
  ```

##### GAGGLE_LOG_LEVEL

- **Description**: Set logging level for structured logs emitted by the Rust core (via `tracing`)
- **Type**: String (`ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`); case-insensitive
- **Default**: `WARN`
- **Status**: ✅ Implemented
- **Example**:
  ```bash
  export GAGGLE_LOG_LEVEL=INFO
  ```

  Notes:
    - Logging is initialized lazily on first use (when the crate is loaded in-process or when `gaggle::init_logging()`
      is called). The environment variable is read once per process.
    - Logs include a level prefix and optional ANSI colors if stderr is a terminal.

#### Offline Mode

- **GAGGLE_OFFLINE**
    - **Description**: Disable network access. When enabled, operations that require network will fail fast unless data
      is already cached.
    - **Type**: Boolean (`1`, `true`, `yes`, `on` to enable)
    - **Default**: `false`
    - **Effects**:
        - `gaggle_download(...)` fails if the dataset isn’t cached.
        - `gaggle_version_info` reports `latest_version` as "unknown" if no cache metadata exists.
        - `gaggle_is_current` and other version checks use cached `.downloaded` metadata when available.
        - `gaggle_search` and `gaggle_info` also fail fast in offline mode (no network attempts).
    - **Example**:
      ```bash
      export GAGGLE_OFFLINE=1
      ```

### Usage Examples

#### Example 1: Custom Cache Directory

```bash
## Set custom cache directory
export GAGGLE_CACHE_DIR="/mnt/fast-ssd/kaggle-cache"

## Start DuckDB
./build/release/duckdb

## Check configuration
SELECT gaggle_search('iris', 1, 10);
```

#### Example 2: Larger Cache for Big Datasets

```bash
# Set cache to 50GB for large datasets
export GAGGLE_CACHE_SIZE_LIMIT_MB=51200

# Download and query large Kaggle datasets
./build/release/duckdb
```

#### Example 3: Production Configuration

```bash
# Complete production configuration
export GAGGLE_CACHE_DIR="/var/lib/gaggle/cache"
export GAGGLE_CACHE_SIZE_LIMIT_MB=51200     # 50GB
export GAGGLE_HTTP_TIMEOUT=120              # 2 minutes
export GAGGLE_HTTP_RETRY_ATTEMPTS=5         # Retry up to 5 times
export GAGGLE_HTTP_RETRY_DELAY=2            # 2 second initial delay
export GAGGLE_HTTP_RETRY_MAX_DELAY=30       # Cap backoff at 30s
export GAGGLE_LOG_LEVEL=WARN                # Production logging

## Set Kaggle credentials
export KAGGLE_USERNAME="your-username"
export KAGGLE_KEY="your-api-key"

## Run DuckDB with Gaggle
./build/release/duckdb
```

#### Example 4: Development/Debug Configuration

```bash
## Development setup with verbose logging
export GAGGLE_CACHE_DIR="./dev-cache"
export GAGGLE_LOG_LEVEL=DEBUG               ## Detailed debug logs
export GAGGLE_HTTP_TIMEOUT=10               ## Shorter timeout for dev
export GAGGLE_HTTP_RETRY_ATTEMPTS=1         ## Fail fast in development
export GAGGLE_HTTP_RETRY_DELAY=0.25         ## Quick retry (250ms)

## Run DuckDB
./build/release/duckdb
```

#### Example 5: Slow Network Configuration

```bash
## Configuration for slow or unreliable networks
export GAGGLE_HTTP_TIMEOUT=300              ## 5 minute timeout
export GAGGLE_HTTP_RETRY_ATTEMPTS=10        ## Many retries
export GAGGLE_HTTP_RETRY_DELAY=5            ## 5 second initial delay
export GAGGLE_HTTP_RETRY_MAX_DELAY=60       ## Cap at 60s

./build/release/duckdb
```

#### Example 6: Offline Mode

```bash
# Enable offline mode
export GAGGLE_OFFLINE=1

# Attempt to download a dataset (will fail if not cached)
SELECT gaggle_download('username/dataset-name');

# Querying metadata or searching will fail fast in offline mode
SELECT gaggle_info('username/dataset-name');
SELECT gaggle_search('keyword', 1, 10);
```

### Configuration Verification

You can verify your configuration at runtime:

```sql
-- Check cache info (includes limit and usage)
SELECT gaggle_cache_info();
-- Returns: {"path": "...", "size_mb": 1024, "limit_mb": 102400, "usage_percent": 1, "is_soft_limit": true, "type": "local"}

-- Manually enforce cache limit (LRU eviction)
SELECT gaggle_enforce_cache_limit();

-- Search datasets (requires valid credentials)
SELECT gaggle_search('housing', 1, 10);

-- Get dataset metadata
SELECT gaggle_info('username/dataset-name');

-- Retrieve last error string (or NULL if none)
SELECT gaggle_last_error();
```

### Retry Policy Details

Gaggle implements retries with exponential backoff for HTTP requests. The number of attempts, initial delay, and
maximum delay can be tuned with the environment variables above.

### Logging Levels

Detailed logging control via `GAGGLE_LOG_LEVEL` is implemented.

### Units

- Storage sizes are reported in megabytes (MB) throughout the API and SQL functions.
- Timeouts and retry delays are configured in seconds via environment variables with clean names (no unit suffixes). For
  example: `GAGGLE_HTTP_RETRY_DELAY=1.5`.

```sql
-- Example cache info (note size is in MB only)
SELECT gaggle_cache_info();
-- {"path":"...","size_mb":42,"limit_mb":102400,"usage_percent":0,"is_soft_limit":true,"type":"local"}
```
