.echo on

-- section 1: Set up
load 'build/release/extension/gaggle/gaggle.duckdb_extension';
select gaggle_set_credentials('your-username', 'your-api-key') as credentials_set;

-- section 2: Error handling - missing dataset
select '## Handle missing dataset error';
-- This will raise an error that can be observed in the shell
-- You can wrap it in a client-side try/catch depending on your environment
-- Example: attempt to fetch info for a non-existent dataset
-- select gaggle_info('nonexistent/dataset-name');

-- section 3: Query integration - filter dataset results (parse JSON)
select '## Filter search results';
with search_results as (
  select from_json(gaggle_search('titanic', 1, 10)) as j
)
select json_extract_string(value, '$.ref') as ref,
       json_extract_string(value, '$.title') as title
from json_each((select j from search_results))
limit 5;

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

-- section 5: Cleanup
select '## Cleanup';
drop table datasets;

.echo off
