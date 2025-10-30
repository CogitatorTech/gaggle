## Gaggle's Configuration Guide

Gaggle supports configuration via environment variables to customize its behavior without code changes.

### Environment Variables

#### Cache Configuration

##### GAGGLE_CACHE_DIR

- **Description**: Directory path for caching downloaded Kaggle datasets
- **Type**: String (path)
- **Default**: `$XDG_CACHE_HOME/gaggle_cache` (typically `~/.cache/gaggle_cache`)
- **Example**:
  ```bash
  export GAGGLE_CACHE_DIR="/var/cache/gaggle"
  ```

##### GAGGLE_CACHE_SIZE_LIMIT (planned)

- **Description**: Maximum cache size in bytes for downloaded datasets
- **Type**: Integer (bytes)
- **Default**: `10737418240` (10GB)
- **Status**: Planned, not implemented yet
- **Example**:
  ```bash
  ## Set to 50GB
  export GAGGLE_CACHE_SIZE_LIMIT=53687091200

  ## Set to 5GB
  export GAGGLE_CACHE_SIZE_LIMIT=5368709120
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

#### Logging Configuration

##### GAGGLE_VERBOSE

- **Description**: Enable verbose logging (boolean)
- **Type**: Boolean (accepts: 1, true, yes, on, 0, false, no, off)
- **Default**: `false`
- **Example**:
  ```bash
  export GAGGLE_VERBOSE=1
  ```

##### GAGGLE_LOG_LEVEL (planned)

- **Description**: Set logging level for detailed output
- **Type**: String (`ERROR`, `WARN`, `INFO`, `DEBUG`)
- **Default**: `WARN`
- **Status**: Planned, not implemented yet
- **Example**:
  ```bash
  ## Show all messages including debug
  export GAGGLE_LOG_LEVEL=DEBUG

  ## Show only errors
  export GAGGLE_LOG_LEVEL=ERROR

  ## Show informational messages and above
  export GAGGLE_LOG_LEVEL=INFO
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

#### Example 2: Larger Cache for Big Datasets (planned)

```bash
## Set cache to 50GB for large datasets (planned)
export GAGGLE_CACHE_SIZE_LIMIT=53687091200

## Download and query large Kaggle datasets
./build/release/duckdb
```

#### Example 3: Production Configuration

```bash
## Complete production configuration
export GAGGLE_CACHE_DIR="/var/lib/gaggle/cache"
export GAGGLE_CACHE_SIZE_LIMIT=53687091200  ## 50GB (planned)
export GAGGLE_HTTP_TIMEOUT=120              ## 2 minutes
export GAGGLE_HTTP_RETRY_ATTEMPTS=5         ## Retry up to 5 times (planned)
export GAGGLE_HTTP_RETRY_DELAY=2000         ## 2 second initial delay (planned)
export GAGGLE_LOG_LEVEL=WARN                ## Production logging (planned)

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
export GAGGLE_LOG_LEVEL=DEBUG               ## Detailed debug logs (planned)
export GAGGLE_HTTP_TIMEOUT=10               ## Shorter timeout for dev
export GAGGLE_HTTP_RETRY_ATTEMPTS=1         ## Fail fast in development (planned)

## Run DuckDB
./build/release/duckdb
```

#### Example 5: Slow Network Configuration (partially planned)

```bash
## Configuration for slow or unreliable networks
export GAGGLE_HTTP_TIMEOUT=300              ## 5 minute timeout
export GAGGLE_HTTP_RETRY_ATTEMPTS=10        ## Many retries (planned)
export GAGGLE_HTTP_RETRY_DELAY=5000         ## 5 second initial delay (planned)

./build/release/duckdb
```

### Configuration Verification

You can verify your configuration at runtime:

```sql
-- Search datasets (requires valid credentials)
SELECT gaggle_search('housing', 1, 10);

-- Get dataset metadata
SELECT gaggle_info('username/dataset-name');
```

### Retry Policy Details (planned)

Retry policy and exponential backoff are planned but not yet implemented. Current releases do not retry failed HTTP requests automatically.

### Logging Levels (planned)

Detailed logging control via `GAGGLE_LOG_LEVEL` is planned but not yet implemented.

### Notes

- Cache directory and HTTP timeout are checked at runtime. Changing `GAGGLE_CACHE_DIR` or `GAGGLE_HTTP_TIMEOUT` takes effect for subsequent operations in the same process.
- Kaggle credentials can be provided via environment variables, config file, or the `gaggle_set_credentials()` SQL function.
- Invalid values fall back to sensible defaults.
