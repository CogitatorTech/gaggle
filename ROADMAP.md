## Feature Roadmap

This document includes the roadmap for the Gaggle DuckDB extension.
It outlines features to be implemented and their current status.

> [!IMPORTANT]
> This roadmap is a work in progress and is subject to change.

### 1. Kaggle API Integration

* **Authentication**
    * [x] Set Kaggle API credentials programmatically.
    * [x] Support environment variables for authentication (`KAGGLE_USERNAME` and `KAGGLE_KEY`).
    * [x] Support reading credentials from `~/.kaggle/kaggle.json file`.
* **Dataset Operations**
    * [x] Search for datasets on Kaggle.
    * [x] Download datasets from Kaggle.
    * [x] List files in a dataset.
    * [x] Get dataset metadata.
    * [x] Dataset version awareness and tracking.
    * [x] Download specific dataset versions (version pinning).
    * [x] Check for dataset updates.
    * [ ] Upload DuckDB tables to Kaggle.

### 2. Caching and Storage

* **Cache Management**
    * [x] Automatic caching of downloaded datasets.
    * [x] Clear cache functionality.
    * [x] Get cache information (size and storage location).
    * [x] Set cache size limit.
* **Storage**
    * [x] Store datasets in configurable directory.
    * [ ] Support for cloud storage backends (S3, GCS, and Azure).

### 3. Data Integration

* **File Format Support**
    * [x] CSV and TSV file reading.
    * [x] Parquet file reading.
    * [x] JSON file reading.
    * [ ] Excel and XLSX file reading.
* **Querying Datasets**
    * [x] Replacement scan for `kaggle:` URLs.
    * [ ] Virtual table support for lazy loading.

### 4. Performance and Concurrency

* **Concurrency Control**
    * [x] Thread-safe credential storage.
    * [x] Thread-safe cache access.
    * [x] Concurrent dataset downloads (with per-dataset serialization to prevent race conditions).
* **Network Optimization**
    * [x] Configurable HTTP timeouts.
    * [x] Retry logic with backoff for failed requests.
* **Caching Strategy**
    * [ ] Incremental cache updates.
    * [ ] Background cache synchronization.

### 5. Error Handling and Resilience

* **Error Messages**
    * [x] Clear error messages for invalid credentials.
    * [x] Clear error messages for missing datasets.
    * [x] Clear error messages for `NULL` inputs.
    * [ ] Detailed error codes for programmatic error handling.
* **Resilience**
    * [x] Automatic retry on network failures.
    * [ ] Graceful degradation when Kaggle API is unavailable.
    * [ ] Local-only mode for cached datasets.

### 6. Documentation and Distribution

* **Documentation**
    * [x] API reference in README.md.
    * [x] Usage examples (see `docs/examples/`).
    * [ ] Tutorial documentation.
    * [ ] FAQ section.
    * [ ] Troubleshooting guide.
* **Testing**
    * [x] Unit tests for core modules (Rust).
    * [x] SQL integration tests (DuckDB shell).
    * [ ] End-to-end integration tests with mocked HTTP.
    * [ ] Performance benchmarks.
* **Distribution**
    * [ ] Pre-compiled extension binaries for Linux, macOS, and Windows.
    * [ ] Submission to the DuckDB Community Extensions repository.
