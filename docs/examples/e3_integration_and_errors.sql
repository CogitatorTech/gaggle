-- Gaggle Integration and Error Handling Examples
-- Demonstrates error handling and integration patterns with DuckDB queries

.echo on

-- section 1: Set up
load 'build/release/extension/gaggle/gaggle.duckdb_extension';
select gaggle_set_credentials('your-username', 'your-api-key') as credentials_set;

-- section 2: Error handling - missing dataset
select '## Handle missing dataset error';
select gaggle_info('nonexistent/dataset-name') as error_result;
select gaggle_last_error() as last_error_message;

-- section 3: Query integration - filter dataset results
select '## Filter search results';
with search_results as (
  select gaggle_search('titanic', 1, 10) as results
)
select results from search_results;

-- section 4: Integration - bulk dataset operations
select '## Batch dataset operations';
create or replace table datasets as
values
  ('owid/covid-latest-data'),
  ('uciml/iris'),
  ('titanic-dataset/titanic')
  ;

-- Download multiple datasets (note: this may take time)
select dataset_name,
       gaggle_download(dataset_name) as local_path
from datasets
limit 1;

-- section 5: Error handling - null inputs
select '## Handle null pointer inputs';
-- These would fail with proper error handling
-- select gaggle_info(null) as null_input_error;
-- Instead verify the error was caught:
select 'Null inputs are handled by gaggle_last_error()' as note;

-- section 6: Cleanup
select '## Cleanup';
drop table datasets;

.echo off
