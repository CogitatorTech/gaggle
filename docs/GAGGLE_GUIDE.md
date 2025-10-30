# Gaggle Configuration Guide

## Overview

Gaggle is a DuckDB extension that provides seamless integration with Kaggle datasets. This guide covers installation, configuration, and usage.

## Installation

### Prerequisites

- DuckDB (latest version recommended)
- Kaggle API credentials

### Building from Source

```bash
# Clone the repository
git clone https://github.com/CogitatorTech/gaggle.git
cd gaggle

# Build the Rust library
cd gaggle
cargo build --release --features duckdb_extension
cd ..

# Build the DuckDB extension
make release

# Install (optional)
make install
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

| Variable | Description | Default |
|----------|-------------|---------|
| `KAGGLE_USERNAME` | Your Kaggle username | None (required) |
| `KAGGLE_KEY` | Your Kaggle API key | None (required) |
| `GAGGLE_CACHE_DIR` | Directory for caching downloaded datasets | System cache directory + `/gaggle_cache` |
| `GAGGLE_VERBOSE` | Enable verbose logging | `false` |
| `GAGGLE_HTTP_TIMEOUT` | HTTP request timeout in seconds | `30` |

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
SELECT * FROM json_each(
    gaggle_search('covid-19', 1, 20)
);

-- Parameters: (query, page, page_size)
```

#### 3. Download Dataset

```sql
-- Download and get local cache path
SELECT gaggle_download('owid/covid-latest-data');
-- Returns: /path/to/cache/datasets/owid/covid-latest-data
```

#### 4. List Files in Dataset

```sql
SELECT * FROM json_each(
    gaggle_list_files('owid/covid-latest-data')
);
-- Returns: JSON array of files with name and size
```

#### 5. Get Dataset Metadata

```sql
SELECT * FROM json_each(
    gaggle_info('owid/covid-latest-data')
);
-- Returns: JSON with title, description, size, etc.
```

#### 6. Read Dataset Files

```sql
-- Get the local path to a specific file
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('owid/covid-latest-data') || '/owid-covid-latest.csv')
) LIMIT 100;

-- Or use Parquet files
SELECT * FROM parquet_scan(
    (SELECT gaggle_download('username/dataset') || '/data.parquet')
);
```

#### 7. Cache Management

```sql
-- Get cache information
SELECT * FROM json_each(gaggle_get_cache_info());

-- Clear cache
SELECT gaggle_clear_cache();
-- Returns: true on success
```

#### 8. Get Version

```sql
SELECT gaggle_get_version();
-- Returns: JSON with version info
```

## Examples

### Example 1: Explore COVID-19 Data

```sql
LOAD gaggle;

-- Search for COVID datasets
SELECT * FROM json_each(gaggle_search('covid-19', 1, 10));

-- Download and query
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('owid/covid-latest-data') || '/owid-covid-latest.csv')
)
WHERE location = 'United States'
LIMIT 10;
```

### Example 2: Analyze Titanic Dataset

```sql
-- Download Titanic dataset
SELECT gaggle_download('heptapod/titanic');

-- List available files
SELECT * FROM json_each(gaggle_list_files('heptapod/titanic'));

-- Query the data
SELECT
    Pclass,
    Sex,
    AVG(Age) as avg_age,
    AVG(Fare) as avg_fare,
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
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('dataset1/name') || '/data.csv')
);

CREATE VIEW data2 AS
SELECT * FROM read_csv_auto(
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
-- Clear the entire cache
SELECT gaggle_clear_cache();

-- Re-download the dataset
SELECT gaggle_download('owner/dataset-name');
```

## Performance Tips

1. **Use caching**: Downloaded datasets are cached locally for fast subsequent access
2. **Filter early**: Use WHERE clauses to limit data read from CSV files
3. **Create views**: For frequently accessed datasets, create views
4. **Parquet over CSV**: When available, use Parquet files for better performance

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
