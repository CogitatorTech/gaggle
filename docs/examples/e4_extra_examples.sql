.echo on

-- Optional: configure retry/backoff via environment
--   export GAGGLE_HTTP_RETRY_ATTEMPTS=3
--   export GAGGLE_HTTP_RETRY_DELAY=250

-- Load the extension and set credentials
load 'build/release/extension/gaggle/gaggle.duckdb_extension';
select gaggle_set_credentials('your-username', 'your-api-key') as credentials_set;

-- Get version
select gaggle_version() as version;

-- Download a dataset and read a file via local path
select gaggle_download('habedi/flickr-8k-dataset-clean') as flickr_path;
prepare rp as select * from read_parquet(?) limit 5;
execute rp(gaggle_file_paths('habedi/flickr-8k-dataset-clean', 'flickr8k.parquet'));

-- Read directly via kaggle: URL using replacement scan
select count(*) as cnt from 'kaggle:habedi/flickr-8k-dataset-clean/flickr8k.parquet';
-- Glob Parquet files in a dataset directory
select count(*) as cnt from 'kaggle:habedi/flickr-8k-dataset-clean/*.parquet';

-- Search and parse JSON
with s as (
  select from_json(gaggle_search('flickr', 1, 10)) as j
)
select json_extract_string(value, '$.ref') as ref,
       json_extract_string(value, '$.title') as title
from json_each((select j from s))
limit 5;

-- List files (JSON and table)
select gaggle_list_files('habedi/flickr-8k-dataset-clean') as files_json;
select * from gaggle_ls('habedi/flickr-8k-dataset-clean') limit 5;

-- Get dataset metadata
select gaggle_info('habedi/flickr-8k-dataset-clean') as metadata;

-- Purge cache and get cache info
select gaggle_purge_cache() as cache_purged;
select gaggle_cache_info() as cache_info;

.echo off
