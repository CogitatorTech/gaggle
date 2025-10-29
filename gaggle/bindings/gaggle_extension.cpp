#define DUCKDB_EXTENSION_MAIN

#include "include/gaggle_extension.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/common/types/data_chunk.hpp"
#include "duckdb/common/types/value.hpp"
#include "duckdb/common/types/vector.hpp"
#include "duckdb/function/pragma_function.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/function/table_function.hpp"
#include "duckdb/main/extension/extension_loader.hpp"
#include "duckdb/parser/parsed_data/create_table_function_info.hpp"
#include "duckdb/parser/parsed_data/create_pragma_function_info.hpp"
#include <algorithm>
#include <cstdint>
#include <iostream>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

#include "rust.h"

namespace duckdb {
using namespace gaggle;

/**
 * @brief Retrieves the last error message from the Gaggle Rust core.
 * @return A string containing the error message, or "unknown error" if not set.
 */
static std::string GetGaggleError() {
  const char *err = gaggle_last_error();
  return err ? std::string(err) : std::string("unknown error");
}

/**
 * @brief Implements the `gaggle_set_credentials(username, key)` SQL function.
 */
static void SetCredentials(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 2) {
    throw InvalidInputException("gaggle_set_credentials(username, key) expects exactly 2 arguments");
  }
  if (args.size() == 0) { return; }

  auto username_val = args.data[0].GetValue(0);
  auto key_val = args.data[1].GetValue(0);

  if (username_val.IsNull() || key_val.IsNull()) {
    throw InvalidInputException("Username and key cannot be NULL");
  }

  std::string username = username_val.ToString();
  std::string key = key_val.ToString();

  int rc = gaggle_set_credentials(username.c_str(), key.c_str());
  bool success = rc == 0;

  if (!success) {
    throw InvalidInputException("Failed to set credentials: " + GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<bool>(result)[0] = success;
  ConstantVector::SetNull(result, false);
}

/**
 * @brief Implements the `gaggle_download(dataset_path)` SQL function.
 */
static void DownloadDataset(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException("gaggle_download(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) { return; }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *local_path = gaggle_download_dataset(path_str.c_str());

  if (local_path == nullptr) {
    throw InvalidInputException("Failed to download dataset: " + GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] = StringVector::AddString(result, local_path);
  ConstantVector::SetNull(result, false);
  gaggle_free(local_path);
}

/**
 * @brief Implements the `gaggle_list_files(dataset_path)` SQL function.
 */
static void ListFiles(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException("gaggle_list_files(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) { return; }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *files_json = gaggle_list_files(path_str.c_str());

  if (files_json == nullptr) {
    throw InvalidInputException("Failed to list files: " + GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] = StringVector::AddString(result, files_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(files_json);
}

/**
 * @brief Implements the `gaggle_search(query, page, page_size)` SQL function.
 */
static void SearchDatasets(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 3) {
    throw InvalidInputException("gaggle_search(query, page, page_size) expects exactly 3 arguments");
  }
  if (args.size() == 0) { return; }

  auto query_val = args.data[0].GetValue(0);
  auto page_val = args.data[1].GetValue(0);
  auto page_size_val = args.data[2].GetValue(0);

  if (query_val.IsNull()) {
    throw InvalidInputException("Query cannot be NULL");
  }

  std::string query_str = query_val.ToString();
  int32_t page = page_val.IsNull() ? 1 : page_val.GetValue<int32_t>();
  int32_t page_size = page_size_val.IsNull() ? 20 : page_size_val.GetValue<int32_t>();

  char *results_json = gaggle_search(query_str.c_str(), page, page_size);

  if (results_json == nullptr) {
    throw InvalidInputException("Failed to search datasets: " + GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] = StringVector::AddString(result, results_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(results_json);
}

/**
 * @brief Implements the `gaggle_info(dataset_path)` SQL function.
 */
static void GetDatasetInfo(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException("gaggle_info(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) { return; }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *info_json = gaggle_get_dataset_info(path_str.c_str());

  if (info_json == nullptr) {
    throw InvalidInputException("Failed to get dataset info: " + GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] = StringVector::AddString(result, info_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(info_json);
}

/**
 * @brief Implements the `gaggle_get_version()` SQL function.
 */
static void GetVersion(DataChunk &args, ExpressionState &state, Vector &result) {
  char *info_json_c = gaggle_get_version();
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] = StringVector::AddString(result, info_json_c);
  ConstantVector::SetNull(result, false);
  gaggle_free(info_json_c);
}

/**
 * @brief Implements the `gaggle_clear_cache()` SQL function.
 */
static void ClearCache(DataChunk &args, ExpressionState &state, Vector &result) {
  int rc = gaggle_clear_cache();
  bool success = rc == 0;
  if (!success) {
    throw InvalidInputException("Failed to clear cache: " + GetGaggleError());
  }
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<bool>(result)[0] = success;
  ConstantVector::SetNull(result, false);
}

/**
 * @brief Implements the `gaggle_get_cache_info()` SQL function.
 */
static void GetCacheInfo(DataChunk &args, ExpressionState &state, Vector &result) {
  char *cache_info_json = gaggle_get_cache_info();
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] = StringVector::AddString(result, cache_info_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(cache_info_json);
}

/**
 * @brief Table function to read a Kaggle dataset file as a table
 */
struct KaggleReadBindData : public TableFunctionData {
  string dataset_path;
  string filename;
  string local_path;
};

static unique_ptr<FunctionData> KaggleReadBind(ClientContext &context, TableFunctionBindInput &input,
                                                vector<LogicalType> &return_types, vector<string> &names) {
  auto result = make_uniq<KaggleReadBindData>();

  result->dataset_path = input.inputs[0].ToString();
  result->filename = input.inputs[1].ToString();

  // Get the local file path
  char *file_path = gaggle_get_file_path(result->dataset_path.c_str(), result->filename.c_str());
  if (file_path == nullptr) {
    throw InvalidInputException("Failed to get file path: " + GetGaggleError());
  }
  result->local_path = std::string(file_path);
  gaggle_free(file_path);

  return std::move(result);
}

static void KaggleReadFunction(ClientContext &context, TableFunctionInput &data_p, DataChunk &output) {
  // The actual data reading is delegated to DuckDB's CSV reader
  // This is handled by the replacement scan
}

static unique_ptr<TableRef> KaggleReplacementScan(ClientContext &context, const string &table_name,
                                                    ReplacementScanData *data) {
  // Check if table_name starts with "kaggle:"
  if (!StringUtil::StartsWith(table_name, "kaggle:")) {
    return nullptr;
  }

  // Parse kaggle:owner/dataset/file.csv
  string kaggle_ref = table_name.substr(7); // Remove "kaggle:" prefix
  auto last_slash = kaggle_ref.find_last_of('/');
  if (last_slash == string::npos) {
    return nullptr;
  }

  string dataset_path = kaggle_ref.substr(0, last_slash);
  string filename = kaggle_ref.substr(last_slash + 1);

  // Get the local file path
  char *file_path = gaggle_get_file_path(dataset_path.c_str(), filename.c_str());
  if (file_path == nullptr) {
    return nullptr;
  }

  string local_path = std::string(file_path);
  gaggle_free(file_path);

  // Return nullptr for now - replacement scan not fully implemented
  return nullptr;
}

/**
 * @brief Registers all the Gaggle functions with DuckDB.
 */
static void LoadInternal(ExtensionLoader &loader) {
  // Scalar functions
  loader.RegisterFunction(ScalarFunction("gaggle_set_credentials",
    {LogicalType::VARCHAR, LogicalType::VARCHAR}, LogicalType::BOOLEAN, SetCredentials));
  loader.RegisterFunction(ScalarFunction("gaggle_download",
    {LogicalType::VARCHAR}, LogicalType::VARCHAR, DownloadDataset));
  loader.RegisterFunction(ScalarFunction("gaggle_list_files",
    {LogicalType::VARCHAR}, LogicalType::VARCHAR, ListFiles));
  loader.RegisterFunction(ScalarFunction("gaggle_search",
    {LogicalType::VARCHAR, LogicalType::INTEGER, LogicalType::INTEGER}, LogicalType::VARCHAR, SearchDatasets));
  loader.RegisterFunction(ScalarFunction("gaggle_info",
    {LogicalType::VARCHAR}, LogicalType::VARCHAR, GetDatasetInfo));
  loader.RegisterFunction(ScalarFunction("gaggle_get_version",
    {}, LogicalType::VARCHAR, GetVersion));
  loader.RegisterFunction(ScalarFunction("gaggle_clear_cache",
    {}, LogicalType::BOOLEAN, ClearCache));
  loader.RegisterFunction(ScalarFunction("gaggle_get_cache_info",
    {}, LogicalType::VARCHAR, GetCacheInfo));

  // Register replacement scan for "kaggle:" prefix
  // This allows: SELECT * FROM 'kaggle:owner/dataset/file.csv'
  // TODO: Implement replacement scan properly
  // loader.config.replacement_scans.emplace_back(KaggleReplacementScan);
}

void GaggleExtension::Load(ExtensionLoader &loader) { LoadInternal(loader); }
std::string GaggleExtension::Name() { return "gaggle"; }
std::string GaggleExtension::Version() const { return "v0.3.0"; }

} // namespace duckdb

extern "C" {
DUCKDB_EXTENSION_API void gaggle_duckdb_cpp_init(duckdb::ExtensionLoader &loader) {
  duckdb::LoadInternal(loader);
}

DUCKDB_EXTENSION_API void gaggle_init(duckdb::DatabaseInstance &db) {
  duckdb::ExtensionLoader loader(db, "gaggle");
  duckdb::LoadInternal(loader);
}
}

