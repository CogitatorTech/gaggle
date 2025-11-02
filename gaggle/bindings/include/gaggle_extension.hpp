#pragma once

#include "duckdb.hpp"
#include "duckdb/main/extension/extension_loader.hpp"

namespace duckdb {

/**
 * @brief The GaggleExtension class is the main entry point for the Gaggle
 * DuckDB extension.
 *
 * This class is responsible for loading the extension, providing its name, and
 * its version. It inherits from the `duckdb::Extension` base class.
 */
class GaggleExtension : public Extension {
public:
  /**
   * @brief Loads the extension's functions into the DuckDB instance.
   *
   * This method is called by DuckDB when the extension is loaded. It registers
   * all the custom scalar and table functions provided by Gaggle.
   * @param loader The extension loader provided by DuckDB.
   */
  void Load(ExtensionLoader &loader) override;

  /**
   * @brief Returns the name of the extension.
   * @return The string "gaggle".
   */
  std::string Name() override;

  /**
   * @brief Returns the version of the extension.
   * @return The version string.
   */
  std::string Version() const override;
};

} // namespace duckdb
