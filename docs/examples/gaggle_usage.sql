# Gaggle - Kaggle Dataset Extension for DuckDB
# Example Usage

# Load the extension
LOAD 'build/release/extension/gaggle/gaggle.duckdb_extension';

# Set Kaggle credentials (or use KAGGLE_USERNAME and KAGGLE_KEY env vars, or ~/.kaggle/kaggle.json)
SELECT gaggle_set_credentials('your-username', 'your-api-key');

# Get extension version
SELECT gaggle_get_version();

# Search for datasets
SELECT * FROM json_each(gaggle_search('covid-19', 1, 10));

# Download a dataset
SELECT gaggle_download('owid/covid-latest-data');

# List files in a dataset
SELECT * FROM json_each(gaggle_list_files('owid/covid-latest-data'));

# Get dataset metadata
SELECT * FROM json_each(gaggle_info('owid/covid-latest-data'));

# Read a CSV file from Kaggle dataset directly
-- Option 1: Using read_csv with the file path
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('owid/covid-latest-data') || '/owid-covid-latest.csv')
) LIMIT 10;

# Clear cache
SELECT gaggle_clear_cache();

# Get cache information
SELECT * FROM json_each(gaggle_get_cache_info());

