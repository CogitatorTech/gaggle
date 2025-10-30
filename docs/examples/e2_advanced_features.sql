-- Gaggle Advanced Features Examples
-- Demonstrates dataset operations with file paths and integration with DuckDB functions

.echo on

-- section 1: Get file path and read CSV directly
select '## Read CSV from Kaggle dataset directly';
load 'build/release/extension/gaggle/gaggle.duckdb_extension';
select gaggle_set_credentials('your-username', 'your-api-key') as credentials_set;

-- Get path to specific file
select gaggle_get_file_path('owid/covid-latest-data', 'owid-covid-latest.csv') as file_path;

-- Use the file path with DuckDB's read_csv_auto
select * from read_csv_auto(
    (select gaggle_get_file_path('owid/covid-latest-data', 'owid-covid-latest.csv'))
) limit 10;

-- section 2: List and process multiple files
select '## List and process dataset files';
with files as (
  select gaggle_list_files('owid/covid-latest-data') as files_json
)
select files_json from files;

-- section 3: Download and verify cache
select '## Verify dataset is cached';
select gaggle_download('owid/covid-latest-data') as cached_path;
select gaggle_get_cache_info() as cache_status;

-- section 4: Clear cache if needed
select '## Clear cache (optional)';
select gaggle_clear_cache() as cache_cleared;

.echo off
