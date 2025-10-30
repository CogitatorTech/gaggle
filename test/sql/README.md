# Gaggle Extension Test Suite

This directory contains sqllogictest-based tests for the Gaggle DuckDB extension.

## Test Files

### 1. `test_gaggle_core.test`
Tests core Gaggle functionality:
- Extension loading and version information
- Credential management
- Cache information retrieval
- Cache clearing operations
- Error handling for NULL inputs
- Type consistency checks

**Key Tests:**
- `gaggle_get_version()` - Verify version format and content
- `gaggle_set_credentials()` - Test credential setting
- `gaggle_get_cache_info()` - Verify cache information
- `gaggle_clear_cache()` - Test cache clearing
- Error cases with NULL values

### 2. `test_gaggle_edge_cases.test`
Tests edge cases and boundary conditions:
- Idempotent operations (calling same function multiple times)
- Credential overwriting
- Empty credential handling
- Very long credential strings
- Type consistency across function calls
- Return value validation

**Key Tests:**
- Multiple clear_cache calls
- Sequential credential updates
- Edge case string lengths
- Function return type consistency

### 3. `test_gaggle_integration.test`
Tests realistic integration scenarios:
- Multi-step workflows (set credentials → get version → check cache)
- Transaction support
- Functions in aggregate contexts
- Functions in WHERE clauses
- String operations with function results
- Consistency across repeated calls

**Key Tests:**
- Complete workflow sequences
- Transactional operations
- Integration with DuckDB functions
- Consistency checks

## Running Tests

### Run all tests:
```bash
make test
```

### Run a specific test file:
```bash
./build/release/test/unittest test/sql/test_gaggle_core.test
```

### Run with verbose output:
```bash
./build/release/test/unittest test/sql/test_gaggle_core.test -verbose
```

## Test Format (sqllogictest)

Each test file follows the sqllogictest format:

- `# group: [gaggle]` - Marks this as a Gaggle test group
- `statement ok` - SQL statement that should succeed
- `statement error` - SQL statement that should fail
- `query <TYPE>` - SELECT query with expected results
  - `T` = TEXT/VARCHAR
  - `B` = BOOLEAN
  - `I` = INTEGER

Example:
```sql
statement ok
load 'build/release/extension/gaggle/gaggle.duckdb_extension'

query T
select gaggle_get_version()
----
{"version":"0.1.0","name":"Gaggle - Kaggle Dataset DuckDB Extension"}
```

## Adding New Tests

When adding new tests:

1. Choose the appropriate test file or create a new one
2. Follow sqllogictest format
3. Start with `statement ok` to load the extension
4. Use `query <TYPE>` for SELECT statements with expected results
5. Use `statement error` for queries that should fail
6. Use descriptive comments (lines starting with `#`)

### Example test structure:
```sql
# group: [gaggle]

# Description of what this test validates
statement ok
pragma enable_verification

statement ok
load 'build/release/extension/gaggle/gaggle.duckdb_extension'

# Test: [description]
query <TYPE>
select [function]
----
[expected_output]
```

## Expected Results

The tests validate:
- ✅ Extension loads successfully
- ✅ All functions are callable
- ✅ Functions return correct types
- ✅ Error handling works properly
- ✅ Operations are idempotent where appropriate
- ✅ Functions work in various SQL contexts (WHERE, aggregates, transactions, etc.)
- ✅ Return values are consistent and valid

## Troubleshooting

If tests fail:
1. Ensure `make release` completed successfully
2. Check that `build/release/extension/gaggle/gaggle.duckdb_extension` exists
3. Verify the extension loads with: `./build/release/duckdb -c "LOAD 'build/release/extension/gaggle/gaggle.duckdb_extension';"`
4. Check individual test file for specific assertion failures
5. Review function documentation in `docs/README.md`
