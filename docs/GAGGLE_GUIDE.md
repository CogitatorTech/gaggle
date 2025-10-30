# Gaggle Configuration Guide

## Overview

Gaggle is a DuckDB extension that provides seamless integration with Kaggle datasets. This guide covers installation,
configuration, and usage.

## Installation

### Prerequisites

- DuckDB (matching the version bundled in `external/duckdb` if building from source)
- Kaggle API credentials
- Rust toolchain (for building the Rust core)

### Building from Source

```bash
# Clone the repository (including submodules)
git clone --recursive https://github.com/CogitatorTech/gaggle.git
cd gaggle

# Build (DuckDB + extension + Rust core)
make release

# Start the local DuckDB shell with the extension available
./build/release/duckdb
```

## Configuration

### Kaggle API Credentials

Gaggle needs your Kaggle credentials to access datasets. There are three ways to provide them:

#### Option 1: Environment Variables (Recommended for CI/CD)

```bash
export KAGGLE_USERNAME="your-username"
export KAGGLE_KEY="your-api-key"
```

#### Option 2: Kaggle Configuration File (Recommended for local development)

Create `~/.kaggle/kaggle.json`:

```json
{
    "username": "your-username",
    "key": "your-api-key"
}
```

Make sure to set proper permissions:

```bash
chmod 600 ~/.kaggle/kaggle.json
```

#### Option 3: SQL Function (Recommended for temporary sessions)

```sql
SELECT gaggle_set_credentials('your-username', 'your-api-key');
```

### Getting Kaggle API Credentials

1. Go to https://www.kaggle.com/
2. Sign in to your account
3. Go to Account Settings (click on your profile picture → Settings)
4. Scroll down to the "API" section
5. Click "Create New API Token"
6. A `kaggle.json` file will be downloaded with your credentials

### Environment Variables

| Variable              | Description                               | Default                                  |
|-----------------------|-------------------------------------------|------------------------------------------|
| `KAGGLE_USERNAME`     | Your Kaggle username                      | None (required)                          |
| `KAGGLE_KEY`          | Your Kaggle API key                       | None (required)                          |
| `GAGGLE_CACHE_DIR`    | Directory for caching downloaded datasets | System cache directory + `/gaggle_cache` |
| `GAGGLE_VERBOSE`      | Enable verbose logging                    | `false`                                  |
| `GAGGLE_HTTP_TIMEOUT` | HTTP request timeout in seconds           | `30`                                     |

## Usage

### Loading the Extension

```sql
LOAD 'build/release/extension/gaggle/gaggle.duckdb_extension';
```

Or if installed:

```sql
LOAD gaggle;
```

### Core Functions

#### 1. Set Credentials

```sql
SELECT gaggle_set_credentials('username', 'api-key');
-- Returns: true on success
```

#### 2. Search Datasets

```sql
-- Search for COVID-19 datasets
SELECT gaggle_search('covid-19', 1, 20) AS results_json;
```

#### 3. Download Dataset

```sql
-- Download and get local cache path
SELECT gaggle_download('owid/covid-latest-data') AS local_dir;
```

#### 4. List Files in Dataset (Table)

```sql
-- Returns rows: name, size (MB), path
SELECT * FROM gaggle_ls('owid/covid-latest-data') LIMIT 5;
```

#### 5. Get Dataset Metadata

```sql
SELECT gaggle_info('owid/covid-latest-data') AS metadata_json;
```

#### 6. Read Dataset Files

```sql
-- Preferred: Parquet via prepared statement
PREPARE rp AS SELECT * FROM read_parquet(?) LIMIT 100;
EXECUTE rp(gaggle_file_paths('owid/covid-latest-data', 'owid-covid-latest.parquet'));

-- Replacement scan via kaggle: URL
SELECT COUNT(*)
FROM 'kaggle:owid/covid-latest-data/*.parquet';
```

#### 7. Cache Management

```sql
-- Cache information (JSON with path, size [MB], type)
SELECT gaggle_cache_info();

-- Purge cache (clears local dataset cache)
SELECT gaggle_purge_cache();
```

#### 8. Get Version

```sql
SELECT gaggle_version();
-- Returns: plain version string (e.g., '0.1.0')
```

## Examples

### Example 1: Explore COVID-19 Data

```sql
LOAD gaggle;

-- Search for COVID datasets
SELECT gaggle_search('covid-19', 1, 10) AS results_json;

-- Download and query (Parquet if available)
PREPARE rp AS SELECT * FROM read_parquet(?) LIMIT 10;
EXECUTE rp(gaggle_file_paths('owid/covid-latest-data', 'owid-covid-latest.parquet'));
```

### Example 2: Analyze Titanic Dataset

```sql
-- Download Titanic dataset
SELECT gaggle_download('heptapod/titanic');

-- List available files (table)
SELECT * FROM gaggle_ls('heptapod/titanic');

-- Query the data (CSV)
SELECT Pclass,
       Sex,
       AVG(Age)                         as avg_age,
       AVG(Fare)                        as avg_fare,
       SUM(Survived) * 100.0 / COUNT(*) as survival_rate
FROM read_csv_auto(
        (SELECT gaggle_download('heptapod/titanic') || '/train.csv')
     )
GROUP BY Pclass, Sex
ORDER BY Pclass, Sex;
```

### Example 3: Join Multiple Datasets

```sql
-- Download multiple datasets
SELECT gaggle_download('dataset1/name');
SELECT gaggle_download('dataset2/name');

-- Create views for easier access
CREATE VIEW data1 AS
SELECT *
FROM read_csv_auto(
        (SELECT gaggle_download('dataset1/name') || '/data.csv')
     );

CREATE VIEW data2 AS
SELECT *
FROM read_csv_auto(
        (SELECT gaggle_download('dataset2/name') || '/data.csv')
     );

-- Perform joins
SELECT *
FROM data1 d1
         JOIN data2 d2 ON d1.id = d2.id;
```

## Troubleshooting

### Error: "No Kaggle credentials found"

**Solution:** Set your credentials using one of the three methods described above.

### Error: "Failed to download dataset: HTTP 403"

**Possible causes:**

- Invalid credentials
- Dataset requires acceptance of competition rules
- Rate limit exceeded

**Solution:**

- Verify your credentials
- Accept dataset terms on Kaggle website
- Wait a few minutes before retrying

### Error: "Dataset not found"

**Solution:**

- Verify the dataset path format: `owner/dataset-name`
- Check if the dataset exists on Kaggle
- Ensure you have access rights to the dataset

### Cache Issues

If you experience cache corruption:

```sql
-- Purge the entire cache
SELECT gaggle_purge_cache();

-- Re-download the dataset
SELECT gaggle_download('owner/dataset-name');
```

## Performance Tips

1. **Use caching**: Downloaded datasets are cached locally for fast subsequent access
2. **Filter early**: Use WHERE clauses to limit data read
3. **Prefer Parquet**: Use Parquet files for better performance when available
4. **Prepared statements**: Use PREPARE/EXECUTE to pass dynamic file paths to table functions

## Security Notes

- Keep your `kaggle.json` file secure with proper permissions (chmod 600)
- Never commit credentials to version control
- Use environment variables in CI/CD pipelines
- Rotate API keys periodically

## API Reference

See the [API documentation](API.md) for detailed function signatures and return types.

## Support

- GitHub Issues: https://github.com/CogitatorTech/gaggle/issues
- Documentation: https://github.com/CogitatorTech/gaggle/tree/main/docs
