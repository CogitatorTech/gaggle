## Gaggle's Configuration Guide

Gaggle supports configuration via environment variables to customize its behavior without code changes.

### Environment Variables

#### Cache Configuration

##### GAGGLE_CACHE_DIR

- **Description**: Directory path for caching downloaded Kaggle datasets
- **Type**: String (path)
- **Default**: `$HOME/.gaggle_cache` (user home directory)
- **Example**:
  ```bash
  export GAGGLE_CACHE_DIR="/var/cache/gaggle"
  ```

##### GAGGLE_CACHE_SIZE_LIMIT

- **Description**: Maximum cache size in bytes for downloaded datasets
- **Type**: Integer (bytes)
- **Default**: `10737418240` (10GB)
- **Example**:
  ```bash
  ## Set to 50GB
  export GAGGLE_CACHE_SIZE_LIMIT=53687091200

  ## Set to 5GB
  export GAGGLE_CACHE_SIZE_LIMIT=5368709120
  ```

#### HTTP Configuration

##### GAGGLE_HTTP_TIMEOUT

- **Description**: HTTP request timeout in seconds for downloading datasets from Kaggle
- **Type**: Integer (seconds)
- **Default**: `30`
- **Example**:
  ```bash
  export GAGGLE_HTTP_TIMEOUT=120
  ```

##### GAGGLE_HTTP_RETRY_ATTEMPTS

- **Description**: Number of retry attempts for failed downloads
- **Type**: Integer
- **Default**: `3`
- **Example**:
  ```bash
  ## Retry up to 5 times on failure
  export GAGGLE_HTTP_RETRY_ATTEMPTS=5
  ```

##### GAGGLE_HTTP_RETRY_DELAY

- **Description**: Initial delay between retry attempts in milliseconds (uses exponential backoff)
- **Type**: Integer (milliseconds)
- **Default**: `1000` (1 second)
- **Example**:
  ```bash
  ## Wait 2 seconds between retries
  export GAGGLE_HTTP_RETRY_DELAY=2000
  ```

#### Logging Configuration

##### GAGGLE_VERBOSE

- **Description**: Enable verbose logging (deprecated, use GAGGLE_LOG_LEVEL instead)
- **Type**: Boolean (`1`, `true`, or `0`, `false`)
- **Default**: `false`
- **Example**:
  ```bash
  export GAGGLE_VERBOSE=1
  ```

##### GAGGLE_LOG_LEVEL

- **Description**: Set logging level for detailed output
- **Type**: String (`ERROR`, `WARN`, `INFO`, `DEBUG`)
- **Default**: `WARN`
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
SELECT gaggle_search_datasets('iris');
```

#### Example 2: Larger Cache for Big Datasets

```bash
## Set cache to 50GB for large datasets
export GAGGLE_CACHE_SIZE_LIMIT=53687091200

## Download and query large Kaggle datasets
./build/release/duckdb
```

#### Example 3: Production Configuration

```bash
## Complete production configuration
export GAGGLE_CACHE_DIR="/var/lib/gaggle/cache"
export GAGGLE_CACHE_SIZE_LIMIT=53687091200  ## 50GB
export GAGGLE_HTTP_TIMEOUT=120              ## 2 minutes
export GAGGLE_HTTP_RETRY_ATTEMPTS=5         ## Retry up to 5 times
export GAGGLE_HTTP_RETRY_DELAY=2000         ## 2 second initial delay
export GAGGLE_LOG_LEVEL=WARN                ## Production logging

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

## Run DuckDB
./build/release/duckdb
```

#### Example 5: Slow Network Configuration

```bash
## Configuration for slow or unreliable networks
export GAGGLE_HTTP_TIMEOUT=300              ## 5 minute timeout
export GAGGLE_HTTP_RETRY_ATTEMPTS=10        ## Many retries
export GAGGLE_HTTP_RETRY_DELAY=5000         ## 5 second initial delay
export GAGGLE_LOG_LEVEL=INFO                ## Track download progress

./build/release/duckdb
```

### Configuration Verification

You can verify your configuration at runtime:

```sql
-- Search datasets (requires valid credentials)
SELECT gaggle_search_datasets('housing');

-- Get dataset metadata
SELECT gaggle_get_dataset_metadata('username/dataset-name');
```

### Retry Policy Details

When downloading datasets from Kaggle, Gaggle automatically retries failed downloads with exponential backoff:

1. **Attempt 1**: Download immediately
2. **Attempt 2**: Wait `GAGGLE_HTTP_RETRY_DELAY` milliseconds (e.g., 1 second)
3. **Attempt 3**: Wait `GAGGLE_HTTP_RETRY_DELAY * 2` milliseconds (e.g., 2 seconds)
4. **Attempt N**: Wait `GAGGLE_HTTP_RETRY_DELAY * N` milliseconds

This helps handle temporary network issues, server rate limiting, and transient failures.

### Logging Levels

Logging levels control the verbosity of output to stderr:

- **ERROR**: Only critical errors that prevent operations
- **WARN**: Warnings about potential issues (default)
- **INFO**: Informational messages about operations (cache hits/misses, downloads)
- **DEBUG**: Detailed debugging information (retry attempts, file sizes, etc.)

Example log output with `GAGGLE_LOG_LEVEL=INFO`:

```
[INFO] Searching datasets for 'iris'...
[INFO] Cache miss for dataset 'uciml/iris', downloading...
[INFO] Successfully downloaded dataset
[INFO] Cache hit for dataset 'uciml/iris'
```

Example log output with `GAGGLE_LOG_LEVEL=DEBUG`:

```
[DEBUG] Download attempt 1/3 for uciml/iris
[INFO] Successfully downloaded dataset
[DEBUG] Downloaded file size: 15728640 bytes
```

### Notes

- Environment variables are read once when Gaggle initializes
- Changes to environment variables require restarting DuckDB
- Invalid values fall back to defaults (no errors thrown)
- Cache directory is created automatically if it doesn't exist
- Logging output goes to stderr and doesn't interfere with SQL query results
- Retry delays use exponential backoff to handle rate limiting gracefully
- Kaggle credentials must be set via environment variables, config file, or SQL function
