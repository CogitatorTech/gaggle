### API Reference

The table below includes the information about all SQL functions exposed by Gaggle.

| # | Function                                                | Return Type      | Description                                                                         |
|---|:--------------------------------------------------------|:-----------------|:------------------------------------------------------------------------------------|
| 1 | `gaggle_set_credentials(username VARCHAR, key VARCHAR)` | `BOOLEAN`        | Sets Kaggle API credentials for the session. Returns `true` on success.             |
| 2 | `gaggle_search(query VARCHAR, page INTEGER, page_size INTEGER)` | `VARCHAR (JSON)` | Searches Kaggle for datasets matching the query and returns results as JSON.        |
| 3 | `gaggle_list_files(dataset_path VARCHAR)`        | `VARCHAR (JSON)` | Lists all files in a Kaggle dataset (format: 'owner/dataset-name').                 |
| 4 | `gaggle_download(dataset_path VARCHAR)`         | `VARCHAR`        | Downloads a Kaggle dataset and returns the local cache directory path.              |
| 5 | `gaggle_info(dataset_path VARCHAR)`     | `VARCHAR (JSON)` | Returns metadata for a Kaggle dataset including size, description, and update info. |

> [!NOTE]
> Kaggle credentials can be provided via environment variables (`KAGGLE_USERNAME`, `KAGGLE_KEY`),
> a `~/.kaggle/kaggle.json` file, or using the `gaggle_set_credentials()` function.

---

### Usage Examples

This section includes some examples of how to use the Gaggle functions.

#### Credential Management

```sql
-- Set credentials programmatically
select gaggle_set_credentials('your-username', 'your-api-key');

-- Or use environment variables: KAGGLE_USERNAME and KAGGLE_KEY
-- Or create ~/.kaggle/kaggle.json with credentials
```

#### Dataset Discovery

```sql
-- Search for datasets matching a query
select gaggle_search('housing', 1, 10);
-- Returns JSON with matching datasets

-- Get metadata about a specific dataset
select gaggle_info('username/dataset-name');
-- Returns JSON with size, description, update date, etc.
```

#### Dataset Access

```sql
-- List files in a dataset
select gaggle_list_files('username/dataset-name');
-- Returns JSON array of files in the dataset

-- Download a dataset (cached locally)
select gaggle_download('username/dataset-name');
-- Returns local directory path

-- Read a CSV file from a Kaggle dataset
select *
from read_csv('~/.gaggle_cache/datasets/username/dataset-name/file.csv');
```

#### Integration with DuckDB

```sql
-- Load the extension
LOAD
'build/release/extension/gaggle/gaggle.duckdb_extension';

-- Search for a dataset
select gaggle_search('iris', 1, 10);

-- Download and read the dataset
select *
from read_csv((select gaggle_download('uciml/iris') || '/iris.csv'));
```

---

### Building Gaggle from Source

To build Gaggle from source, you need to have GNU Make, CMake, and a C++ compiler (like GCC or Clang) installed.
You also need to have Rust (nightly version) and Cargo installed.

1. **Clone the repository:**

   ```bash
   git clone --recursive https://github.com/CogitatorTech/gaggle.git
   cd gaggle
   ```

> [!NOTE]
> The `--recursive` flag is important to clone the required submodules (like DuckDB).

2. **Install dependencies:**

   The project includes a [`Makefile`](../Makefile) target to help set up the development environment. For Debian-based
   systems, you can run:
   ```bash
   make install-deps
   ```
   This will install necessary system packages, Rust tools, and Python dependencies. For other operating systems, please
   check the `Makefile` to see the list of dependencies and install them manually.

3. **Build the extension:**

   Run the following command to build the DuckDB shell with the Gaggle extension included:
   ```bash
   make release
   ```
   This will create a `duckdb` executable inside the `build/release/` directory.

4. **Run the custom DuckDB shell:**

   You can now run the custom-built DuckDB shell:
   ```bash
   ./build/release/duckdb
   ```
   The Gaggle extension will be automatically available, and you can start using the `gaggle_*` functions right away
   without needing to run the `load` command.

> [!NOTE]
> After a successful build, you will find the following files in the `build/release/` directory:
> - `./build/release/duckdb`: this is a DuckDB binary with the Gaggle extension already statically linked to it.
> - `./build/release/test/unittest`: this is the test runner for running the SQL tests in the `test/sql/` directory.
> - `./build/release/extension/gaggle/gaggle.duckdb_extension`: this is the loadable extension file for Gaggle.

---

### Configuration

See [CONFIGURATION.md](CONFIGURATION.md) for more information about how to configure various settings for Gaggle.

### Architecture

Gaggle is made up of two main components:

1. **Rust Core (`gaggle/src/`)**: The core logic is implemented in Rust. This component is responsible for:
    * Authenticating with Kaggle API using credentials.
    * Searching for and discovering datasets on Kaggle.
    * Downloading datasets and managing local cache.
    * Listing files in datasets.
    * Retrieving dataset metadata.
    * Exposing a C-compatible Foreign Function Interface (FFI) so that it can be called from other languages.

2. **C++ DuckDB Bindings (`gaggle/bindings/`)**: A C++ layer that connects the Rust core and DuckDB. Its
   responsibilities include:
    * Defining the custom SQL functions (like `gaggle_set_credentials` and `gaggle_search`).
    * Translating data from DuckDB's internal vector-based format into the raw data pointers expected by the Rust FFI.
    * Calling the Rust functions and handling the returned results and errors.
    * Integrating with DuckDB's extension loading mechanism.
