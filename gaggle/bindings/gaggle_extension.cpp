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
#include "duckdb/main/config.hpp"
#include "duckdb/main/extension/extension_loader.hpp"
#include "duckdb/parser/expression/constant_expression.hpp"
#include "duckdb/parser/expression/function_expression.hpp"
#include "duckdb/parser/parsed_data/create_pragma_function_info.hpp"
#include "duckdb/parser/parsed_data/create_table_function_info.hpp"
#include "duckdb/parser/tableref/table_function_ref.hpp"
#include <algorithm>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

#include "duckdb.h"
#include "rust.h"

namespace duckdb {
using namespace gaggle;
namespace fs = std::filesystem;

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
static void SetCredentials(DataChunk &args, ExpressionState &state,
                           Vector &result) {
  if (args.ColumnCount() != 2) {
    throw InvalidInputException(
        "gaggle_set_credentials(username, key) expects exactly 2 arguments");
  }
  if (args.size() == 0) {
    return;
  }

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
    throw InvalidInputException("Failed to set credentials: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<bool>(result)[0] = success;
  ConstantVector::SetNull(result, false);
}

/**
 * @brief Implements the `gaggle_download(dataset_path)` SQL function.
 */
static void DownloadDataset(DataChunk &args, ExpressionState &state,
                            Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_download(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *local_path = gaggle_download_dataset(path_str.c_str());

  if (local_path == nullptr) {
    throw InvalidInputException("Failed to download dataset: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, local_path);
  ConstantVector::SetNull(result, false);
  gaggle_free(local_path);
}

/**
 * @brief Implements the `gaggle_list_files(dataset_path)` SQL function.
 */
static void ListFiles(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_list_files(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

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
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, files_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(files_json);
}

/**
 * @brief Implements the `gaggle_search(query, page, page_size)` SQL function.
 */
static void SearchDatasets(DataChunk &args, ExpressionState &state,
                           Vector &result) {
  if (args.ColumnCount() != 3) {
    throw InvalidInputException(
        "gaggle_search(query, page, page_size) expects exactly 3 arguments");
  }
  if (args.size() == 0) {
    return;
  }

  auto query_val = args.data[0].GetValue(0);
  auto page_val = args.data[1].GetValue(0);
  auto page_size_val = args.data[2].GetValue(0);

  if (query_val.IsNull()) {
    throw InvalidInputException("Query cannot be NULL");
  }

  std::string query_str = query_val.ToString();
  int32_t page = page_val.IsNull() ? 1 : page_val.GetValue<int32_t>();
  int32_t page_size =
      page_size_val.IsNull() ? 20 : page_size_val.GetValue<int32_t>();

  char *results_json = gaggle_search(query_str.c_str(), page, page_size);

  if (results_json == nullptr) {
    throw InvalidInputException("Failed to search datasets: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, results_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(results_json);
}

/**
 * @brief Implements the `gaggle_info(dataset_path)` SQL function.
 */
static void GetDatasetInfo(DataChunk &args, ExpressionState &state,
                           Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_info(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *info_json = gaggle_get_dataset_info(path_str.c_str());

  if (info_json == nullptr) {
    throw InvalidInputException("Failed to get dataset info: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, info_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(info_json);
}

/**
 * @brief Implements the `gaggle_get_version()` SQL function.
 */
static void GetVersion(DataChunk &args, ExpressionState &state,
                       Vector &result) {
  char *info_json_c = gaggle_get_version();
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, info_json_c);
  ConstantVector::SetNull(result, false);
  gaggle_free(info_json_c);
}

/**
 * @brief Implements the `gaggle_clear_cache()` SQL function.
 */
static void ClearCache(DataChunk &args, ExpressionState &state,
                       Vector &result) {
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
 * @brief Implements the `gaggle_cache_info()` SQL function.
 */
static void GetCacheInfo(DataChunk &args, ExpressionState &state,
                         Vector &result) {
  char *cache_info_json = gaggle_get_cache_info();
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, cache_info_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(cache_info_json);
}

/**
 * @brief Implements the `gaggle_enforce_cache_limit()` SQL function.
 */
static void EnforceCacheLimit(DataChunk &args, ExpressionState &state,
                              Vector &result) {
  int rc = gaggle_enforce_cache_limit();
  bool success = rc == 0;
  if (!success) {
    throw InvalidInputException("Failed to enforce cache limit: " +
                                GetGaggleError());
  }
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<bool>(result)[0] = success;
  ConstantVector::SetNull(result, false);
}

/**
 * @brief Implements the `gaggle_is_current(dataset_path)` SQL function.
 */
static void IsDatasetCurrent(DataChunk &args, ExpressionState &state,
                             Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_is_current(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  int rc = gaggle_is_dataset_current(path_str.c_str());

  if (rc < 0) {
    throw InvalidInputException("Failed to check dataset version: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<bool>(result)[0] = (rc == 1);
  ConstantVector::SetNull(result, false);
}

/**
 * @brief Implements the `gaggle_update_dataset(dataset_path)` SQL function.
 */
static void UpdateDataset(DataChunk &args, ExpressionState &state,
                          Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_update_dataset(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *local_path = gaggle_update_dataset(path_str.c_str());

  if (local_path == nullptr) {
    throw InvalidInputException("Failed to update dataset: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, local_path);
  ConstantVector::SetNull(result, false);
  gaggle_free(local_path);
}

/**
 * @brief Implements the `gaggle_version_info(dataset_path)` SQL function.
 */
static void GetDatasetVersionInfo(DataChunk &args, ExpressionState &state,
                                  Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_version_info(dataset_path) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

  auto path_val = args.data[0].GetValue(0);
  if (path_val.IsNull()) {
    throw InvalidInputException("Dataset path cannot be NULL");
  }

  std::string path_str = path_val.ToString();
  char *version_json = gaggle_dataset_version_info(path_str.c_str());

  if (version_json == nullptr) {
    throw InvalidInputException("Failed to get version info: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, version_json);
  ConstantVector::SetNull(result, false);
  gaggle_free(version_json);
}

/**
 * @brief Implements the `gaggle_json_each(json)` SQL function.
 * Returns newline-delimited JSON rows for each element/key in the input JSON.
 */
static void JsonEach(DataChunk &args, ExpressionState &state, Vector &result) {
  if (args.ColumnCount() != 1) {
    throw InvalidInputException(
        "gaggle_json_each(json) expects exactly 1 argument");
  }
  if (args.size() == 0) {
    return;
  }

  auto json_val = args.data[0].GetValue(0);
  if (json_val.IsNull()) {
    throw InvalidInputException("JSON input cannot be NULL");
  }

  std::string json_str = json_val.ToString();
  char *result_str = gaggle_json_each(json_str.c_str());
  if (!result_str) {
    throw InvalidInputException("Failed to parse JSON: " + GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, result_str);
  ConstantVector::SetNull(result, false);
  gaggle_free(result_str);
}

/**
 * @brief Implements the `gaggle_file_path(dataset_path, filename)` SQL
 * function.
 */
static void GetFilePath(DataChunk &args, ExpressionState &state,
                        Vector &result) {
  if (args.ColumnCount() != 2) {
    throw InvalidInputException(
        "gaggle_file_path(dataset_path, filename) expects exactly 2 arguments");
  }
  if (args.size() == 0) {
    return;
  }

  auto ds_val = args.data[0].GetValue(0);
  auto fn_val = args.data[1].GetValue(0);
  if (ds_val.IsNull() || fn_val.IsNull()) {
    throw InvalidInputException("Dataset path and filename cannot be NULL");
  }
  std::string dataset_path = ds_val.ToString();
  std::string filename = fn_val.ToString();

  char *file_path_c =
      gaggle_get_file_path(dataset_path.c_str(), filename.c_str());
  if (!file_path_c) {
    throw InvalidInputException("Failed to resolve file path: " +
                                GetGaggleError());
  }

  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, file_path_c);
  ConstantVector::SetNull(result, false);
  gaggle_free(file_path_c);
}

/**
 * @brief Implements the `gaggle_last_error()` SQL function.
 * Returns the last error message string or NULL if no error is set.
 */
static void GetLastError(DataChunk &args, ExpressionState &state,
                         Vector &result) {
  result.SetVectorType(VectorType::CONSTANT_VECTOR);
  const char *err = gaggle_last_error();
  if (!err) {
    ConstantVector::SetNull(result, true);
    return;
  }
  ConstantVector::GetData<string_t>(result)[0] =
      StringVector::AddString(result, err);
  ConstantVector::SetNull(result, false);
}

/**
 * @brief Table function to read a Kaggle dataset file as a table
 */
struct KaggleReadBindData : public TableFunctionData {
  string dataset_path;
  string filename;
  string local_path;
};

struct GaggleLsBindData : public TableFunctionData {
  string dataset_path;
  vector<string> names;
  vector<int64_t> sizes;
  vector<string> paths;
  idx_t pos = 0;
};

struct GaggleLsGlobalState : public GlobalTableFunctionState {
  idx_t pos = 0;
};

static unique_ptr<FunctionData>
KaggleReadBind(ClientContext &context, TableFunctionBindInput &input,
               vector<LogicalType> &return_types, vector<string> &names) {
  auto result = make_uniq<KaggleReadBindData>();

  result->dataset_path = input.inputs[0].ToString();
  result->filename = input.inputs[1].ToString();

  // Get the local file path
  char *file_path = gaggle_get_file_path(result->dataset_path.c_str(),
                                         result->filename.c_str());
  if (file_path == nullptr) {
    throw InvalidInputException("Failed to get file path: " + GetGaggleError());
  }
  result->local_path = std::string(file_path);
  gaggle_free(file_path);

  return std::move(result);
}

static void KaggleReadFunction(ClientContext &context,
                               TableFunctionInput &data_p, DataChunk &output) {
  // The actual data reading is delegated to DuckDB's CSV reader
  // This is handled by the replacement scan
}

static unique_ptr<TableRef>
KaggleReplacementScan(ClientContext &context, ReplacementScanInput &input,
                      optional_ptr<ReplacementScanData> data) {
  // Check if table_name starts with "kaggle:"
  const string &table_name = input.table_name;
  if (!StringUtil::StartsWith(table_name, "kaggle:")) {
    return nullptr;
  }

  // Parse kaggle:owner/dataset[/pattern]
  string kaggle_ref = table_name.substr(7); // Remove "kaggle:" prefix
  auto last_slash = kaggle_ref.find_last_of('/');
  if (last_slash == string::npos) {
    return nullptr;
  }

  string dataset_path = kaggle_ref.substr(0, last_slash);
  string pattern = kaggle_ref.substr(last_slash + 1);

  string func_name = "read_csv_auto";
  string local_path;

  auto lower_pat = StringUtil::Lower(pattern);
  bool has_wildcard =
      pattern.find('*') != string::npos || pattern.find('?') != string::npos;
  bool is_dir = pattern.empty();

  auto decide_reader = [](const string &lower_ext) -> string {
    if (StringUtil::EndsWith(lower_ext, ".parquet") ||
        StringUtil::EndsWith(lower_ext, ".parq")) {
      return "read_parquet";
    }
    if (StringUtil::EndsWith(lower_ext, ".json") ||
        StringUtil::EndsWith(lower_ext, ".jsonl") ||
        StringUtil::EndsWith(lower_ext, ".ndjson")) {
      return "read_json_auto";
    }
    if (StringUtil::EndsWith(lower_ext, ".xlsx")) {
      return "read_excel";
    }
    // Default CSV/TSV and others to DuckDB's auto CSV reader
    return "read_csv_auto";
  };

  if (is_dir || has_wildcard) {
    // Ensure dataset is downloaded and construct a glob path
    char *dir_c = gaggle_download_dataset(dataset_path.c_str());
    if (!dir_c) {
      throw InvalidInputException("Failed to prepare dataset directory: " +
                                  GetGaggleError());
    }
    string dir_path(dir_c);
    gaggle_free(dir_c);

    // If directory, default to all files; else use provided wildcard
    string tail = is_dir ? string("/*") : (string("/") + pattern);
    local_path = dir_path + tail;

    // Choose reader based on pattern extension if any
    func_name = decide_reader(lower_pat);
  } else {
    // Specific file: resolve exact path
    char *file_path_c =
        gaggle_get_file_path(dataset_path.c_str(), pattern.c_str());
    if (file_path_c == nullptr) {
      // Fallback: dataset may have nested paths; attempt a glob match under
      // dataset root
      char *dir_c = gaggle_download_dataset(dataset_path.c_str());
      if (!dir_c) {
        throw InvalidInputException(
            "Failed to download dataset for pattern resolution: " +
            GetGaggleError());
      }
      string dir_path(dir_c);
      gaggle_free(dir_c);
      local_path = dir_path + "/" + pattern;
      // Keep func_name decision below based on extension
    } else {
      local_path = string(file_path_c);
      gaggle_free(file_path_c);
    }

    // Decide reader based on extension
    auto lower_name = StringUtil::Lower(pattern);
    func_name = decide_reader(lower_name);
  }

  // Construct a table function call: func_name(local_path)
  vector<unique_ptr<ParsedExpression>> children;
  children.push_back(make_uniq<ConstantExpression>(Value(local_path)));
  auto func_expr =
      make_uniq<FunctionExpression>(func_name, std::move(children));

  // Create a TableFunctionRef manually
  auto table_func_ref = make_uniq<TableFunctionRef>();
  table_func_ref->function = std::move(func_expr);
  return std::move(table_func_ref);
}

static unique_ptr<FunctionData> GaggleLsBind(ClientContext &context,
                                             TableFunctionBindInput &input,
                                             vector<LogicalType> &return_types,
                                             vector<string> &names) {
  auto result = make_uniq<GaggleLsBindData>();
  if (input.inputs.size() != 1) {
    throw InvalidInputException(
        "gaggle_ls(dataset_path) expects exactly 1 argument");
  }
  result->dataset_path = input.inputs[0].ToString();

  // Ensure dataset is downloaded and get directory
  char *dir_c = gaggle_download_dataset(result->dataset_path.c_str());
  if (!dir_c) {
    throw InvalidInputException("Failed to download dataset: " +
                                GetGaggleError());
  }
  string dir_path(dir_c);
  gaggle_free(dir_c);

  // Enumerate files (non-recursive)
  try {
    for (const auto &entry : fs::directory_iterator(dir_path)) {
      if (!entry.is_regular_file()) {
        continue;
      }
      auto name = entry.path().filename().string();
      if (name == ".downloaded") {
        continue;
      }
      auto full_path = entry.path().string();
      std::error_code ec;
      auto file_size = entry.file_size(ec);
      if (ec)
        continue;
      int64_t size_mb = static_cast<int64_t>(file_size / (1024 * 1024));
      result->names.push_back(name);
      result->paths.push_back(full_path);
      result->sizes.push_back(size_mb);
    }
  } catch (const std::exception &e) {
    throw InvalidInputException(string("Failed to enumerate files: ") +
                                e.what());
  }

  return_types = {LogicalType::VARCHAR, LogicalType::BIGINT,
                  LogicalType::VARCHAR};
  names = {"name", "size", "path"};
  return std::move(result);
}

static unique_ptr<GlobalTableFunctionState>
GaggleLsInitGlobal(ClientContext &context, TableFunctionInitInput &input) {
  return make_uniq<GaggleLsGlobalState>();
}

static void GaggleLsFunction(ClientContext &context, TableFunctionInput &data_p,
                             DataChunk &output) {
  auto &bind = data_p.bind_data->Cast<GaggleLsBindData>();
  auto &state = data_p.global_state->Cast<GaggleLsGlobalState>();
  if (state.pos >= bind.names.size()) {
    output.SetCardinality(0);
    return;
  }
  idx_t remaining = bind.names.size() - state.pos;
  idx_t count = MinValue<idx_t>(STANDARD_VECTOR_SIZE, remaining);
  output.SetCardinality(count);
  auto name_out = FlatVector::GetData<string_t>(output.data[0]);
  auto size_out = FlatVector::GetData<int64_t>(output.data[1]);
  auto path_out = FlatVector::GetData<string_t>(output.data[2]);
  for (idx_t i = 0; i < count; i++) {
    auto idx = state.pos + i;
    name_out[i] = StringVector::AddString(output.data[0], bind.names[idx]);
    size_out[i] = bind.sizes[idx];
    path_out[i] = StringVector::AddString(output.data[2], bind.paths[idx]);
  }
  state.pos += count;
}

/**
 * @brief Registers all the Gaggle functions with DuckDB.
 */
static void LoadInternal(ExtensionLoader &loader) {
  // Initialize Rust logging once per process
  gaggle_init_logging();

  // Scalar functions (public)
  loader.RegisterFunction(ScalarFunction(
      "gaggle_set_credentials", {LogicalType::VARCHAR, LogicalType::VARCHAR},
      LogicalType::BOOLEAN, SetCredentials));
  loader.RegisterFunction(
      ScalarFunction("gaggle_download", {LogicalType::VARCHAR},
                     LogicalType::VARCHAR, DownloadDataset));
  loader.RegisterFunction(ScalarFunction(
      "gaggle_search",
      {LogicalType::VARCHAR, LogicalType::INTEGER, LogicalType::INTEGER},
      LogicalType::VARCHAR, SearchDatasets));
  loader.RegisterFunction(ScalarFunction("gaggle_info", {LogicalType::VARCHAR},
                                         LogicalType::VARCHAR, GetDatasetInfo));
  // Single canonical version endpoint
  loader.RegisterFunction(
      ScalarFunction("gaggle_version", {}, LogicalType::VARCHAR, GetVersion));
  loader.RegisterFunction(ScalarFunction("gaggle_clear_cache", {},
                                         LogicalType::BOOLEAN, ClearCache));
  loader.RegisterFunction(ScalarFunction("gaggle_cache_info", {},
                                         LogicalType::VARCHAR, GetCacheInfo));
  loader.RegisterFunction(ScalarFunction("gaggle_enforce_cache_limit", {},
                                         LogicalType::BOOLEAN,
                                         EnforceCacheLimit));
  loader.RegisterFunction(
      ScalarFunction("gaggle_is_current", {LogicalType::VARCHAR},
                     LogicalType::BOOLEAN, IsDatasetCurrent));
  loader.RegisterFunction(ScalarFunction("gaggle_update_dataset",
                                         {LogicalType::VARCHAR},
                                         LogicalType::VARCHAR, UpdateDataset));
  loader.RegisterFunction(
      ScalarFunction("gaggle_version_info", {LogicalType::VARCHAR},
                     LogicalType::VARCHAR, GetDatasetVersionInfo));
  loader.RegisterFunction(ScalarFunction("gaggle_json_each",
                                         {LogicalType::VARCHAR},
                                         LogicalType::VARCHAR, JsonEach));
  loader.RegisterFunction(ScalarFunction(
      "gaggle_file_path", {LogicalType::VARCHAR, LogicalType::VARCHAR},
      LogicalType::VARCHAR, GetFilePath));
  loader.RegisterFunction(ScalarFunction("gaggle_last_error", {},
                                         LogicalType::VARCHAR, GetLastError));

  // Table function: gaggle_ls(dataset_path) -> name,size,path
  TableFunction ls_fun("gaggle_ls", {LogicalType::VARCHAR}, GaggleLsFunction,
                       GaggleLsBind, GaggleLsInitGlobal, nullptr);
  loader.RegisterFunction(ls_fun);

  // Register replacement scan for "kaggle:" prefix via DBConfig
  auto &db = loader.GetDatabaseInstance();
  auto &config = DBConfig::GetConfig(db);
  config.replacement_scans.insert(config.replacement_scans.begin(),
                                  ReplacementScan(KaggleReplacementScan));
}

// Provide out-of-line definitions for the extension class
void GaggleExtension::Load(ExtensionLoader &loader) { LoadInternal(loader); }
std::string GaggleExtension::Name() { return "gaggle"; }
std::string GaggleExtension::Version() const { return std::string("0.1.0-alpha.1"); }

} // namespace duckdb

extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(gaggle, loader) { duckdb::LoadInternal(loader); }
}
