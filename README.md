<div align="center">
  <picture>
    <img alt="Gaggle Logo" src="logo.svg" height="25%" width="25%">
  </picture>
<br>

<h2>Gaggle</h2>

[![Tests](https://img.shields.io/github/actions/workflow/status/CogitatorTech/gaggle/tests.yml?label=tests&style=flat&labelColor=282c34&logo=github)](https://github.com/CogitatorTech/gaggle/actions/workflows/tests.yml)
[![Code Quality](https://img.shields.io/codefactor/grade/github/CogitatorTech/gaggle?label=quality&style=flat&labelColor=282c34&logo=codefactor)](https://www.codefactor.io/repository/github/CogitatorTech/gaggle)
[![Examples](https://img.shields.io/badge/examples-view-green?style=flat&labelColor=282c34&logo=github)](https://github.com/CogitatorTech/gaggle/tree/main/docs/examples)
[![Docs](https://img.shields.io/badge/docs-view-blue?style=flat&labelColor=282c34&logo=read-the-docs)](https://github.com/CogitatorTech/gaggle/tree/main/docs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-007ec6?style=flat&labelColor=282c34&logo=open-source-initiative)](https://github.com/CogitatorTech/gaggle)

Kaggle Datasets for DuckDB

</div>

---

Gaggle is a DuckDB extension that allows you to read and write Kaggle datasets directly in SQL queries, as if
they were DuckDB tables.
It is written in Rust and uses the official Kaggle API to search, download, and manage datasets.

Kaggle hosts a large collection of very useful datasets for data science and machine learning work.
Accessing these datasets typically involves multiple steps including manually downloading a dataset (as a ZIP file),
extracting it, loading the files in the dataset into your data science environment, and managing storage and dataset
updates, etc.
This workflow can be simplified and optimized by bringing the datasets directly into DuckDB.

### Features

- Has a simple API (just a few SQL functions)
- Allows you search, download, update, and delete Kaggle datasets directly from DuckDB
- Supports datasets made of CSV, JSON, and Parquet files
- Configurable and has built-in caching support
- Thread-safe and memory-efficient

See the [ROADMAP.md](ROADMAP.md) for planned features and the [docs](docs) folder for detailed documentation.

> [!IMPORTANT]
> Gaggle is in early development, so bugs and breaking changes are expected.
> Please use the [issues page](https://github.com/CogitatorTech/gaggle/issues) to report bugs or request features.

---

### Quickstart

#### Install from Community Extensions Repository

You can install and load Gaggle from
the [DuckDB community extensions](https://duckdb.org/community_extensions/extensions/gaggle) repository by running the
following SQL commands in the DuckDB shell:

```sql
install gaggle from community;
load gaggle;
```

#### Build from Source

Alternatively, you can build Gaggle from source and use it by following these steps:

1. Clone the repository and build the Gaggle extension from source:

```bash
git clone --recursive https://github.com/CogitatorTech/gaggle.git
cd gaggle

# This might take a while to run
make release
```

2. Start DuckDB shell (with Gaggle statically linked to it):

```bash
./build/release/duckdb
```

> [!NOTE]
> After building from source, the Gaggle binary will be `build/release/extension/gaggle/gaggle.duckdb_extension`.
> You can load it using the `load 'build/release/extension/gaggle/gaggle.duckdb_extension';` in the DuckDB shell.
> Note that the extension binary will only work with the DuckDB version that it was built against.
> You can download the pre-built binaries from the [releases page](https://github.com/CogitatorTech/gaggle/releases) for
> your platform.

#### Trying Gaggle

```sql
-- Load the Gaggle extension
load 'build/release/extension/gaggle/gaggle.duckdb_extension';

-- Set your Kaggle credentials (or use `~/.kaggle/kaggle.json`)
select gaggle_set_credentials('your-username', 'your-api-key');

-- Get extension version
select gaggle_version();

-- Prime cache by downloading the dataset locally (optional, but improves first-time performance)
select gaggle_download('habedi/flickr-8k-dataset-clean');

-- List files in the downloaded dataset
select *
from gaggle_ls('habedi/flickr-8k-dataset-clean') limit 5;

-- Read a Parquet file from local cache using a prepared statement (no subquery in function args)
prepare rp as select * from read_parquet(?) limit 10;
execute rp(gaggle_file_paths('habedi/flickr-8k-dataset-clean', 'flickr8k.parquet'));

-- Use the replacement scan to read directly via kaggle: URL
select count(*)
from 'kaggle:habedi/flickr-8k-dataset-clean/flickr8k.parquet';

-- Or glob Parquet files in a dataset directory
select count(*)
from 'kaggle:habedi/flickr-8k-dataset-clean/*.parquet';

-- Optionally, check cache info
select gaggle_cache_info();
```

[![Simple Demo 1](https://asciinema.org/a/745806.svg)](https://asciinema.org/a/745806)

---

### Documentation

Check out the [docs](docs/README.md) directory for the API documentation, how to build Gaggle from source, and more.

#### Examples

Check out the [examples](docs/examples) directory for SQL scripts that show how to use Gaggle.

---

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to make a contribution.

### License

Gaggle is available under either of the following licenses:

* MIT License ([LICENSE-MIT](LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

### Acknowledgements

* The logo is from [here](https://www.svgrepo.com/svg/322445/goose) with some modifications.
