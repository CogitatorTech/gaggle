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
 * Arguments:
 * - `username`: non-null pointer to a NUL-terminated C string
 * - `key`: non-null pointer to a NUL-terminated C string
 *
 * Returns 0 on success, -1 on failure (call gaggle_last_error).
 *
 * Safety:
 * - The pointers must be valid and remain alive for the duration of this call.
 * - Strings must be valid UTF-8; interior NULs are not allowed.
 */
 int32_t gaggle_set_credentials(const char *username, const char *key);

/**
 * Download a Kaggle dataset and return its local cache path
 *
 * Arguments:
 * - `dataset_path`: non-null pointer to a NUL-terminated C string "owner/dataset[[@vN|@latest]]".
 *
 * Returns pointer to a heap-allocated C string. Free with gaggle_free(). On error, returns NULL and sets gaggle_last_error.
 *
 * Safety:
 * - The pointer must be valid and the string valid UTF-8; interior NULs are not allowed.
 */

char *gaggle_download_dataset(const char *dataset_path);

/**
 * Get the local path to a specific file in a downloaded dataset
 *
 * Arguments:
 * - `dataset_path`: non-null pointer to owner/dataset
 * - `filename`: non-null pointer to relative filename within the dataset
 */
 char *gaggle_get_file_path(const char *dataset_path, const char *filename);

/**
 * List files in a Kaggle dataset
 */
 char *gaggle_list_files(const char *dataset_path);

/**
 * Search for Kaggle datasets
 */
 char *gaggle_search(const char *query, int32_t page, int32_t page_size);

/**
 * Get metadata for a specific Kaggle dataset
 */
 char *gaggle_get_dataset_info(const char *dataset_path);

/**
 * Get version information
 */
 char *gaggle_get_version(void);

/**
 * Frees a heap-allocated C string
 *
 * Safety:
 * - `ptr` must be a pointer previously returned by a Gaggle FFI function that transfers ownership
 *   (e.g., gaggle_get_version, gaggle_list_files, etc.).
 * - Passing the same pointer twice, or a pointer not allocated by Gaggle, results in undefined behavior.
 */

void gaggle_free(char *ptr);

/**
 * Clear the dataset cache
 */
 int32_t gaggle_clear_cache(void);

/**
 * Enforce cache size limit by evicting oldest datasets
 */
 int32_t gaggle_enforce_cache_limit(void);

/**
 * Check if cached dataset is the current version
 */
 int32_t gaggle_is_dataset_current(const char *dataset_path);

/**
 * Force update dataset to latest version (ignores cache)
 */
 char *gaggle_update_dataset(const char *dataset_path);

/**
 * Get version information for a dataset
 */
 char *gaggle_dataset_version_info(const char *dataset_path);

/**
 * Get cache information
 */
 char *gaggle_get_cache_info(void);

/**
 * Parse JSON and expand objects/arrays similar to json_each
 */
 char *gaggle_json_each(const char *json_str);

/**
 * Prefetch multiple files in a dataset without downloading the entire archive
 */
 char *gaggle_prefetch_files(const char *dataset_path, const char *file_list);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#ifdef __cplusplus
}  // namespace gaggle
#endif  // __cplusplus

#endif  /* GAGGLE_H */

/* End of generated bindings */
