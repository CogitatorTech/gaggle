mod config;
mod error;
mod ffi;
mod kaggle;
mod utils;

pub use error::{gaggle_clear_last_error, gaggle_last_error};
pub use ffi::{
    gaggle_clear_cache, gaggle_dataset_version_info, gaggle_download_dataset,
    gaggle_enforce_cache_limit, gaggle_free, gaggle_get_cache_info, gaggle_get_dataset_info,
    gaggle_get_file_path, gaggle_get_version, gaggle_is_dataset_current, gaggle_json_each,
    gaggle_list_files, gaggle_search, gaggle_set_credentials, gaggle_update_dataset,
};
pub use kaggle::parse_dataset_path;
pub use kaggle::parse_dataset_path_with_version;

use once_cell::sync::OnceCell;
use std::io::IsTerminal;
use tracing_subscriber::{fmt, EnvFilter};

static LOG_INIT: OnceCell<()> = OnceCell::new();

/// Initialize global logging based on GAGGLE_LOG_LEVEL.
/// Safe to call multiple times; only the first call has an effect.
pub fn init_logging() {
    let _ = LOG_INIT.get_or_init(|| {
        let level = std::env::var("GAGGLE_LOG_LEVEL").unwrap_or_else(|_| "WARN".to_string());
        let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("WARN"));
        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_level(true)
            .with_ansi(std::io::stderr().is_terminal())
            .init();
    });
}
