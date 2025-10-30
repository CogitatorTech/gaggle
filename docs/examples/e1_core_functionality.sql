.echo on

-- section 1: Load extension and get version
select '## Load extension and get version';
load 'build/release/extension/gaggle/gaggle.duckdb_extension';
select gaggle_version() as version;

-- section 2: Set Kaggle credentials
select '## Set Kaggle credentials';
-- Method 1: Set directly (or use KAGGLE_USERNAME/KAGGLE_KEY env vars or ~/.kaggle/kaggle.json)
select gaggle_set_credentials('your-username', 'your-api-key') as credentials_set;

-- section 3: Search for datasets
select '## Search for datasets';
select gaggle_search('covid', 1, 5) as search_results;

-- section 4: Download a dataset
select '## Download a dataset';
select gaggle_download('owid/covid-latest-data') as download_path;

-- section 5: List files in a dataset (JSON)
select '## List files (JSON)';
select gaggle_list_files('owid/covid-latest-data') as files_json;

-- section 5b: List files in a dataset (table)
select '## List files (table)';
select * from gaggle_ls('owid/covid-latest-data') limit 5;

-- section 6: Get dataset metadata
select '## Get dataset metadata';
select gaggle_info('owid/covid-latest-data') as dataset_metadata;

-- section 7: Get cache information
select '## Get cache information';
select gaggle_cache_info() as cache_info;

.echo off
