# Gaggle Quick Start Guide

Get started with Gaggle in 5 minutes!

## Prerequisites

- DuckDB installed
- Kaggle account with API credentials

## Step 1: Get Kaggle API Credentials

1. Visit https://www.kaggle.com/
2. Sign in to your account
3. Go to Settings (click profile picture → Settings)
4. Scroll to "API" section
5. Click "Create New API Token"
6. Save the downloaded `kaggle.json` file

## Step 2: Set Up Credentials

**Option A: Using the config file (recommended)**

```bash
mkdir -p ~/.kaggle
mv ~/Downloads/kaggle.json ~/.kaggle/
chmod 600 ~/.kaggle/kaggle.json
```

**Option B: Using environment variables**

```bash
export KAGGLE_USERNAME="your-username"
export KAGGLE_KEY="your-api-key"
```

## Step 3: Build Gaggle

```bash
# Clone repository
git clone https://github.com/CogitatorTech/gaggle.git
cd gaggle

# Build Rust library
cd gaggle
cargo build --release --features duckdb_extension
cd ..

# Build DuckDB extension
make release
```

## Step 4: Try It Out!

Start DuckDB and run:

```sql
-- Load the extension
LOAD 'build/release/extension/gaggle/gaggle.duckdb_extension';

-- If you didn't use a config file, set credentials
SELECT gaggle_set_credentials('your-username', 'your-api-key');

-- Search for datasets
SELECT json_extract_string(value, '$.ref') as dataset,
       json_extract_string(value, '$.title') as title
FROM json_each(gaggle_search('titanic', 1, 5));

-- Download Titanic dataset
SELECT gaggle_download('heptapod/titanic');

-- List files in the dataset
SELECT json_extract_string(value, '$.name') as filename
FROM json_each(gaggle_list_files('heptapod/titanic'));

-- Read and analyze data
SELECT Pclass,
       Sex,
       COUNT(*) as passengers,
       AVG(Age) as avg_age,
       SUM(Survived) * 100.0 / COUNT(*) as survival_rate
FROM read_csv_auto(
    (SELECT gaggle_download('heptapod/titanic') || '/train.csv')
)
GROUP BY Pclass, Sex
ORDER BY Pclass, Sex;
```

## Common Use Cases

### Use Case 1: Explore COVID-19 Data

```sql
-- Search for COVID datasets
SELECT * FROM json_each(gaggle_search('covid-19', 1, 10));

-- Download and query
SELECT location, date, new_cases
FROM read_csv_auto(
    (SELECT gaggle_download('owid/covid-latest-data') || '/owid-covid-latest.csv')
)
WHERE location = 'United States'
ORDER BY date DESC
LIMIT 10;
```

### Use Case 2: Create a Persistent View

```sql
-- Download dataset
SELECT gaggle_download('userid/dataset');

-- Create view for easy access
CREATE VIEW my_data AS
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('userid/dataset') || '/data.csv')
);

-- Query like a regular table
SELECT * FROM my_data WHERE category = 'A' LIMIT 100;
```

### Use Case 3: Join Multiple Datasets

```sql
-- Download datasets
SELECT gaggle_download('dataset1/name');
SELECT gaggle_download('dataset2/name');

-- Create views
CREATE VIEW users AS SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('dataset1/name') || '/users.csv')
);

CREATE VIEW transactions AS SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('dataset2/name') || '/transactions.csv')
);

-- Join them
SELECT u.name, COUNT(t.id) as transaction_count, SUM(t.amount) as total
FROM users u
LEFT JOIN transactions t ON u.id = t.user_id
GROUP BY u.name
ORDER BY total DESC;
```

## Tips & Tricks

### 1. Cache Management

```sql
-- Check cache size
SELECT * FROM json_each(gaggle_get_cache_info());

-- Clear cache if needed
SELECT gaggle_clear_cache();
```

### 2. Efficient Queries

```sql
-- Filter early when reading CSVs
SELECT * FROM read_csv_auto(
    (SELECT gaggle_download('dataset/name') || '/large_file.csv')
)
WHERE date >= '2023-01-01'  -- Filter applied during read
LIMIT 1000;
```

### 3. Working with Parquet

```sql
-- If dataset contains Parquet files, use them for better performance
SELECT * FROM parquet_scan(
    (SELECT gaggle_download('dataset/name') || '/data.parquet')
);
```

### 4. Batch Operations

```sql
-- Download multiple datasets at once
SELECT gaggle_download('dataset1/name') as d1,
       gaggle_download('dataset2/name') as d2,
       gaggle_download('dataset3/name') as d3;
```

## Troubleshooting

### Problem: "No Kaggle credentials found"

**Solution:**
```sql
-- Set credentials in SQL
SELECT gaggle_set_credentials('username', 'api-key');
```

### Problem: Dataset download is slow

**Reason:** First download fetches from Kaggle (internet speed dependent)

**Solution:** Be patient! Subsequent access uses cached version (very fast)

### Problem: "Dataset not found"

**Check:**
1. Dataset exists on Kaggle
2. You have access (some datasets require joining competitions)
3. Path format is correct: `owner/dataset-name`

## Next Steps

- Read the [Complete Guide](GAGGLE_GUIDE.md)
- Check the [API Reference](API.md)
- View [More Examples](examples/)
- Report issues on [GitHub](https://github.com/CogitatorTech/gaggle/issues)

## Getting Help

- Documentation: `docs/` folder
- Examples: `docs/examples/`
- Issues: GitHub Issues
- Kaggle API: https://www.kaggle.com/docs/api

Happy data exploring with Gaggle! 🎉
