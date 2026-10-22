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
 * Initializes logging for the Gaggle Rust core.
 *
 * This function sets up the logging framework based on the `GAGGLE_LOG_LEVEL`
 * environment variable. It should be called once at the beginning of the
 * application's lifecycle.
 */
 void gaggle_init_logging(void);

/**
 * Sets the Kaggle API credentials.
 *
 * # Arguments
 *
 * * `username` - A non-null pointer to a NUL-terminated C string representing the Kaggle username.
 * * `key` - A non-null pointer to a NUL-terminated C string representing the Kaggle API key.
 *
 * # Returns
 *
 * Returns `0` on success, or `-1` on failure. If the operation fails,
 * a detailed error message can be retrieved using `gaggle_last_error`.
 *
 * # Safety
 *
 * - The pointers must be valid and remain accessible for the duration of this call.
 * - The provided strings must be valid UTF-8, and interior NUL characters are not permitted.
 */
 int32_t gaggle_set_credentials(const char *username, const char *key);

/**
 * Downloads a Kaggle dataset and returns its local cache path.
 *
 * # Arguments
 *
 * * `dataset_path` - A non-null pointer to a NUL-terminated C string in the format
 *   "owner/dataset[[@vN|@latest]]".
 *
 * # Returns
 *
 * Returns a pointer to a heap-allocated C string containing the local path.
 * This string must be freed with `gaggle_free()`. On error, returns `NULL`
 * and sets a detailed error message retrievable with `gaggle_last_error`.
 *
 * # Safety
 *
 * - The pointer must be valid and the string must be valid UTF-8.
 * - Interior NUL characters are not allowed in the string.
 */
 char *gaggle_download_dataset(const char *dataset_path);

/**
 * Retrieves the local path to a specific file within a downloaded dataset.
 *
 * # Arguments
 *
 * * `dataset_path` - A non-null pointer to a NUL-terminated C string representing the owner and dataset.
 * * `filename` - A non-null pointer to a NUL-terminated C string for the relative filename inside the dataset.
 *
 * # Safety
 *
 * - Both pointers must be valid and point to valid NUL-terminated C strings.
 * - The strings must be valid UTF-8, and interior NUL characters are not allowed.
 */

char *gaggle_get_file_path(const char *dataset_path,
                           const char *filename);

/**
 * Lists the files available in a Kaggle dataset.
 *
 * # Safety
 *
 * - The pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 char *gaggle_list_files(const char *dataset_path);

/**
 * Searches for Kaggle datasets.
 *
 * # Safety
 *
 * - The `query` pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 char *gaggle_search(const char *query, int32_t page, int32_t page_size);

/**
 * Retrieves metadata for a specific Kaggle dataset.
 *
 * # Safety
 *
 * - The pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 char *gaggle_get_dataset_info(const char *dataset_path);

/**
 * Retrieves the version of the Gaggle library.
 */
 char *gaggle_get_version(void);

/**
 * Frees a heap-allocated C string.
 *
 * # Safety
 *
 * `ptr` must be a pointer previously returned by a Gaggle FFI function that transfers ownership,
 * such as `gaggle_get_version` or `gaggle_list_files`. Passing the same pointer more than once,
 * or providing a pointer not allocated by Gaggle, will result in undefined behavior.
 */
 void gaggle_free(char *ptr);

/**
 * Clears the dataset cache.
 */
 int32_t gaggle_clear_cache(void);

/**
 * Enforces the cache size limit by evicting the oldest datasets.
 */
 int32_t gaggle_enforce_cache_limit(void);

/**
 * Checks if the cached dataset is the current version.
 *
 * # Safety
 *
 * - The pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 int32_t gaggle_is_dataset_current(const char *dataset_path);

/**
 * Forces an update of the dataset to the latest version, ignoring the cache.
 *
 * # Safety
 *
 * - The pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 char *gaggle_update_dataset(const char *dataset_path);

/**
 * Retrieves version information for a dataset.
 *
 * # Safety
 *
 * - The pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 char *gaggle_dataset_version_info(const char *dataset_path);

/**
 * Retrieves information about the cache.
 */
 char *gaggle_get_cache_info(void);

/**
 * Parses JSON and expands objects/arrays, similar to `json_each`.
 *
 * # Safety
 *
 * - The pointer must be valid and point to a valid NUL-terminated C string.
 * - The string must be valid UTF-8, and interior NUL characters are not allowed.
 */
 char *gaggle_json_each(const char *json_str);

/**
 * Prefetches multiple files in a dataset without downloading the entire archive.
 *
 * # Safety
 *
 * - Both pointers must be valid and point to valid NUL-terminated C strings.
 * - The strings must be valid UTF-8, and interior NUL characters are not allowed.
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
