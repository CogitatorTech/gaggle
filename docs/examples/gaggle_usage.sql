-- Gaggle - Kaggle Dataset Extension for DuckDB
-- Example Usage

-- load the extension
load 'build/release/extension/gaggle/gaggle.duckdb_extension';

-- set kaggle credentials (or use kaggle_username and kaggle_key env vars, or ~/.kaggle/kaggle.json)
select gaggle_set_credentials('your-username', 'your-api-key');

-- get extension version
select gaggle_get_version();

-- search for datasets
select * from json_each(gaggle_search('covid-19', 1, 10));

-- download a dataset
select gaggle_download('owid/covid-latest-data');

-- list files in a dataset
select * from json_each(gaggle_list_files('owid/covid-latest-data'));

-- get dataset metadata
select * from json_each(gaggle_info('owid/covid-latest-data'));

-- read a csv file from kaggle dataset directly
-- option 1: using read_csv with the file path
select * from read_csv_auto(
    (select gaggle_download('owid/covid-latest-data') || '/owid-covid-latest.csv')
) limit 10;

-- clear cache
select gaggle_clear_cache();

-- get cache information
select * from json_each(gaggle_get_cache_info());
