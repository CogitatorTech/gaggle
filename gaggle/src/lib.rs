mod config;
mod error;
mod ffi;
mod kaggle;

// Re-export error some of the functions to use them internally
pub use error::{gaggle_clear_last_error, gaggle_last_error};
pub use ffi::{
    gaggle_clear_cache, gaggle_dataset_version_info, gaggle_download_dataset,
    gaggle_enforce_cache_limit, gaggle_free, gaggle_get_cache_info, gaggle_get_dataset_info,
    gaggle_get_file_path, gaggle_get_version, gaggle_is_dataset_current, gaggle_json_each,
    gaggle_list_files, gaggle_search, gaggle_set_credentials, gaggle_update_dataset,
};
pub use kaggle::parse_dataset_path;
pub use kaggle::parse_dataset_path_with_version;
