# AGENTS.md

This file provides guidance to coding agents collaborating on this repository.

## Mission

Gaggle is a DuckDB extension for accessing Kaggle datasets from SQL.
It has two tightly coupled layers:

1. A Rust core that talks to the Kaggle API, manages cache state, validates inputs, and exposes a C ABI.
2. A C++ DuckDB extension layer that registers SQL functions, table functions, and replacement-scan behavior on top of that Rust ABI.

Priorities, in order:

1. Correctness and safety of the SQL-facing behavior.
2. Compatibility with supported DuckDB versions and extension CI.
3. Reliable dataset-path parsing, cache behavior, and error propagation.
4. Small, well-tested changes that preserve the existing Rust/C++ boundary.

## Core Rules

- Use English for code, comments, docs, tests, and commit messages.
- Prefer focused fixes over broad refactoring.
- Preserve the existing Rust/C ABI unless the task explicitly requires changing it.
- Treat `gaggle/bindings/include/rust.h` as generated code. If Rust FFI signatures change, regenerate it with `make create-bindings`.
- Do not edit vendored code under `external/` unless the task is explicitly about updating or patching a vendored dependency.
- Do not add new dependencies, network behavior, or background processes unless the requirement clearly calls for them.
- Keep docs and examples aligned with user-visible SQL behavior.

## Writing Style

- Use Oxford commas in inline lists: "a, b, and c" not "a, b, c".
- Do not use em dashes. Restructure the sentence, or use a colon or semicolon instead.
- Avoid colorful adjectives and adverbs. Write "TCP proxy" not "lightweight TCP proxy", "scoring components" not "transparent scoring components".
- Use noun phrases for checklist items, not imperative verbs. Write "redundant index detection" not "detect redundant indexes".
- Headings in Markdown files must be in the title case: "Build from Source" not "Build from source". Minor words (a, an, the, and, but, or, for, in,
  on, at, to, by, of, is, are, was, were, be) stay lowercase unless they are the first word.

## Repository Layout

- `gaggle/src/lib.rs`: Rust crate entry point and public exports for the C ABI surface.
- `gaggle/src/ffi.rs`: `extern "C"` functions exported to the C++ extension layer.
- `gaggle/src/error.rs`: Error types and last-error plumbing shared across the FFI boundary.
- `gaggle/src/config.rs`: Runtime configuration and environment-driven settings.
- `gaggle/src/utils.rs`: Shared helpers.
- `gaggle/src/kaggle/`: Kaggle-specific logic, including credentials, downloads, metadata, search, and dataset-path parsing.
- `gaggle/tests/`: Rust integration, regression, security, replacement-scan, and offline/mock-based tests.
- `gaggle/bindings/gaggle_extension.cpp`: DuckDB extension implementation that maps SQL calls to the Rust ABI.
- `gaggle/bindings/include/gaggle_extension.hpp`: C++ extension declarations.
- `gaggle/bindings/include/rust.h`: Generated C header for the Rust ABI.
- `CMakeLists.txt`: Top-level CMake integration, platform detection, and Corrosion setup.
- `extension_config.cmake`: DuckDB extension wiring and linkage to the prebuilt Rust static library.
- `test/sql/`: Sqllogictest files for SQL-level extension behavior.
- `docs/examples/`: SQL examples that should remain runnable against a local build.
- `.github/workflows/tests.yml`: Rust tests and SQL tests in CI.
- `.github/workflows/lints.yml`: Rust formatting and clippy checks in CI.
- `.github/workflows/dist_pipeline.yml`: cross-platform extension packaging against DuckDB `main` and `v1.5.4`.

## Architecture Notes

### Rust Core

The Rust crate owns Kaggle-facing behavior: credentials, dataset metadata, downloads, local cache management, file resolution, JSON shaping, and error
handling.
All SQL-visible behavior should ultimately reduce to deterministic Rust operations exposed through `ffi.rs`.

### FFI Boundary

The boundary between Rust and C++ is intentionally narrow:

- Rust returns primitive values or heap-allocated C strings.
- C++ is responsible for converting those values into DuckDB vectors and freeing Rust-allocated strings with `gaggle_free`.
- Errors should cross the boundary through the existing last-error mechanism instead of ad hoc conventions.

When changing anything on one side of the boundary, inspect the matching code on the other side in the same change.

### DuckDB Layer

`gaggle/bindings/gaggle_extension.cpp` registers scalar functions such as `gaggle_search` and `gaggle_info`, plus table-style behavior such as
`gaggle_ls` and replacement scans for `kaggle:` paths.
DuckDB API compatibility matters here. If a change touches vector access, function registration, or scans, verify against the vendored DuckDB headers
in `external/duckdb`.

### Build Integration

`make release` and `make debug` build the Rust crate first, then build DuckDB plus the extension.
`extension_config.cmake` expects a prebuilt Rust static library and links it into the DuckDB extension targets.
`CMakeLists.txt` also contains platform and Rust-target selection logic used by local builds and CI distribution builds.

## Generated and Derived Files

- `gaggle/bindings/include/rust.h` is generated from the Rust crate via `cbindgen`.
- `gaggle/target/`, `build/`, and coverage outputs such as `gaggle/cobertura.xml` are build artifacts, not source.
- Do not hand-edit generated artifacts unless the task explicitly requires it, and you explain why.

## Rust Conventions

- Edition: Rust 2021.
- Format with `cargo fmt` through `make rust-format`.
- Lint with `cargo clippy` through `make rust-lint`.
- Follow the existing error style: return typed errors internally, then translate them once at the FFI boundary.
- Avoid `unwrap()` and `expect()` in production code. CI denies them via clippy.
- Prefer existing crates and helpers already in use before introducing new abstractions.

## C++ and DuckDB Conventions

- Keep C++ changes narrowly scoped to DuckDB integration concerns.
- Match current DuckDB APIs used by the vendored headers in `external/duckdb/src/include`.
- Be careful with vector mutability and ownership. Many DuckDB helpers have separate const and mutable accessors.
- Keep user-facing SQL function names, signatures, and error messages stable unless the task explicitly changes them.

## Required Validation

Run the narrowest relevant checks, then expand if the change crosses layers.

| Area                 | Command            | Use When                                                                         |
|----------------------|--------------------|----------------------------------------------------------------------------------|
| Rust formatting      | `make rust-format` | Any Rust code changed                                                            |
| Rust lint            | `make rust-lint`   | Any Rust code changed                                                            |
| Rust tests           | `make rust-test`   | Rust logic, FFI, parsing, cache, or HTTP/mock behavior changed                   |
| Extension build      | `make release`     | C++, CMake, linkage, or SQL-facing behavior changed                              |
| SQL tests            | `make test`        | SQL functions, table functions, replacement scans, or DuckDB integration changed |
| Examples             | `make examples`    | User-visible SQL behavior or docs/examples changed                               |
| Combined local check | `make check`       | Small Rust-only changes                                                          |

Minimum expectations:

- Rust-only logic changes: `make rust-test` and `make rust-lint`.
- FFI changes: `make create-bindings`, `make rust-test`, and `make release`.
- C++ or SQL-surface changes: `make release` and `make test`.
- Docs/examples updates that affect runnable SQL: `make examples`.

## Testing Expectations

- Rust tests in `gaggle/tests/` cover crate behavior, regression cases, property tests, offline mode, replacement-scan behavior, and mock HTTP flows.
- SQL tests in `test/sql/*.test` validate the extension from DuckDB's side and are the right place for user-visible SQL regressions.
- Prefer mock or offline-friendly tests for Kaggle-facing logic. Do not make CI depend on live Kaggle credentials or network access unless the
  repository already does so for that path.
- If you change dataset-path parsing, cache semantics, replacement-scan logic, or SQL function output shape, add or update tests in the layer where
  the regression would be caught first.

## Change Design Checklist

Before coding:

1. Identify whether the task belongs to Rust core, FFI boundary, C++ DuckDB layer, build wiring, or tests.
2. Check whether the change affects one DuckDB version or both `main` and `v1.5.2`.
3. Decide whether `rust.h`, docs examples, or SQL tests need to move with the code.
4. Confirm whether the change is safe under offline or mocked test conditions.

Before submitting:

1. Relevant build and test commands pass locally, or any gaps are explicitly called out.
2. Generated headers are refreshed if the Rust ABI is changed.
3. User-visible SQL changes are covered by `test/sql` and reflected in docs/examples where appropriate.
4. Changes to DuckDB integration are reviewed against the vendored headers, not memory of older APIs.

## Commit and PR Hygiene

- Keep commits scoped to one logical change.
- Mention both layers when relevant: for example, "update Rust FFI and DuckDB binding for X".
- PR descriptions should include:
    1. Behavioral change summary.
    2. Validation runs locally.
    3. Whether the change affects Rust only, SQL surface, or cross-version DuckDB compatibility.
