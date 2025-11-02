/* Generated with cbindgen */
/* DO NOT EDIT */


#ifndef GAGGLE_H
#define GAGGLE_H

#pragma once

/* Generated with cbindgen:0.29.0 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
namespace gaggle {
#endif  // __cplusplus

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Retrieves the last error message set in the current thread.
 *
 * After an FFI function returns an error code, this function can be called
 * to get a more descriptive, human-readable error message.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing the last error message.
 * Returns a null pointer if no error has occurred since the last call.
 * The caller **must not** free this pointer, as it is managed by a thread-local static variable.
 */
 const char *gaggle_last_error(void);

/**
 * Clears the last error for the current thread.
 *
 * This is useful for ensuring that old error messages don't persist
 * and get confused with new errors.
 */
 void gaggle_clear_last_error(void);

/**
 * Initialize logging for the Rust core based on GAGGLE_LOG_LEVEL
 */
 void gaggle_init_logging(void);

/**
 * Set Kaggle API credentials
 *
 * # Arguments
 *
 * * `username` - A pointer to a null-terminated C string representing the Kaggle username.
 * * `key` - A pointer to a null-terminated C string representing the Kaggle API key.
 *
 * # Returns
 *
 * * `0` on success.
 * * `-1` on failure. Call `gaggle_last_error()` to get a descriptive error message.
 *
 * # Safety
 *
 * * The `username` and `key` pointers must not be null.
 * * The memory pointed to by `username` and `key` must be valid, null-terminated C strings.
 */
 int32_t gaggle_set_credentials(const char *username, const char *key);

/**
 * Download a Kaggle dataset and return its local cache path
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path (e.g., "owner/dataset-name").
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing the local path, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The `dataset_path` pointer must not be null.
 * * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
 */

char *gaggle_download_dataset(const char *dataset_path);

/**
 * Get the local path to a specific file in a downloaded dataset
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
 * * `filename` - A pointer to a null-terminated C string representing the filename.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing the file path, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The pointers must not be null.
 * * The memory pointed to must be valid, null-terminated C strings.
 */
 char *gaggle_get_file_path(const char *dataset_path, const char *filename);

/**
 * List files in a Kaggle dataset
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing JSON array of files, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The `dataset_path` pointer must not be null.
 * * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
 */
 char *gaggle_list_files(const char *dataset_path);

/**
 * Search for Kaggle datasets
 *
 * # Arguments
 *
 * * `query` - A pointer to a null-terminated C string representing the search query.
 * * `page` - Page number (1-indexed).
 * * `page_size` - Number of results per page.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing JSON search results, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The `query` pointer must not be null.
 * * The memory pointed to by `query` must be a valid, null-terminated C string.
 */
 char *gaggle_search(const char *query, int32_t page, int32_t page_size);

/**
 * Get metadata for a specific Kaggle dataset
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing JSON metadata, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The `dataset_path` pointer must not be null.
 * * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
 */
 char *gaggle_get_dataset_info(const char *dataset_path);

/**
 * Get version information
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing the version string (e.g., "0.1.0").
 * The caller must free this pointer using `gaggle_free()`.
 */
 char *gaggle_get_version(void);

/**
 * Frees a heap-allocated C string
 *
 * # Safety
 *
 * The `ptr` must be a non-null pointer to a C string that was previously allocated
 * by a Gaggle function.
 */
 void gaggle_free(char *ptr);

/**
 * Clear the dataset cache
 *
 * # Returns
 *
 * * `0` on success.
 * * `-1` on failure.
 */
 int32_t gaggle_clear_cache(void);

/**
 * Enforce cache size limit by evicting oldest datasets
 *
 * # Returns
 *
 * * `0` on success.
 * * `-1` on failure.
 */
 int32_t gaggle_enforce_cache_limit(void);

/**
 * Check if cached dataset is the current version
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
 *
 * # Returns
 *
 * * `1` if cached version is current.
 * * `0` if cached version is outdated or not cached.
 * * `-1` on error.
 *
 * # Safety
 *
 * * The `dataset_path` pointer must not be null.
 * * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
 */
 int32_t gaggle_is_dataset_current(const char *dataset_path);

/**
 * Force update dataset to latest version (ignores cache)
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing the local path, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The `dataset_path` pointer must not be null.
 * * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
 */
 char *gaggle_update_dataset(const char *dataset_path);

/**
 * Get version information for a dataset
 *
 * # Arguments
 *
 * * `dataset_path` - A pointer to a null-terminated C string representing the dataset path.
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing JSON version info, or NULL on failure.
 * The caller must free this pointer using `gaggle_free()`.
 *
 * # Safety
 *
 * * The `dataset_path` pointer must not be null.
 * * The memory pointed to by `dataset_path` must be a valid, null-terminated C string.
 */
 char *gaggle_dataset_version_info(const char *dataset_path);

/**
 * Get cache information
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing JSON cache info.
 * The caller must free this pointer using `gaggle_free()`.
 */
 char *gaggle_get_cache_info(void);

/**
 * Parse JSON and expand objects/arrays similar to json_each
 *
 * # Arguments
 *
 * * `json_str` - A pointer to a null-terminated C string containing JSON data
 *
 * # Returns
 *
 * A pointer to a null-terminated C string containing newline-delimited JSON objects
 *
 * # Safety
 *
 * * The `json_str` pointer must not be null.
 * * The memory pointed to by `json_str` must be a valid, null-terminated C string.
 */
 char *gaggle_json_each(const char *json_str);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#ifdef __cplusplus
}  // namespace gaggle
#endif  // __cplusplus

#endif  /* GAGGLE_H */

/* End of generated bindings */
