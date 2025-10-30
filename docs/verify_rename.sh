#!/bin/bash

# Script to verify that the infera -> gaggle rename is complete

echo "========================================="
echo "Verifying Infera -> Gaggle Rename"
echo "========================================="
echo ""

# Check for renamed files
echo "✓ Checking renamed files..."
if [ -f "gaggle/bindings/gaggle_extension.cpp" ]; then
    echo "  ✓ gaggle_extension.cpp exists"
else
    echo "  ✗ gaggle_extension.cpp NOT FOUND"
fi

if [ -f "gaggle/bindings/include/gaggle_extension.hpp" ]; then
    echo "  ✓ gaggle_extension.hpp exists"
else
    echo "  ✗ gaggle_extension.hpp NOT FOUND"
fi

# Check for old files that should be removed
if [ -f "gaggle/bindings/infera_extension.cpp" ]; then
    echo "  ⚠ OLD FILE infera_extension.cpp still exists!"
fi

if [ -f "gaggle/bindings/include/infera_extension.hpp" ]; then
    echo "  ⚠ OLD FILE infera_extension.hpp still exists!"
fi

echo ""

# Check Makefile
echo "✓ Checking Makefile..."
if grep -q "EXT_NAME := gaggle" Makefile; then
    echo "  ✓ EXT_NAME set to gaggle"
else
    echo "  ✗ EXT_NAME not set correctly"
fi

if grep -q "gaggle/target/release" Makefile; then
    echo "  ✓ Rust lib path updated"
else
    echo "  ✗ Rust lib path not updated"
fi

echo ""

# Check CMakeLists.txt
echo "✓ Checking CMakeLists.txt..."
if grep -q "GAGGLE_ENABLE_ONNX" CMakeLists.txt; then
    echo "  ✓ CMake options updated"
else
    echo "  ✗ CMake options not updated"
fi

if grep -q "gaggle/bindings/gaggle_extension.cpp" CMakeLists.txt; then
    echo "  ✓ Extension source path updated"
else
    echo "  ✗ Extension source path not updated"
fi

echo ""

# Check extension_config.cmake
echo "✓ Checking extension_config.cmake..."
if grep -q "duckdb_extension_load(gaggle" extension_config.cmake; then
    echo "  ✓ Extension load updated"
else
    echo "  ✗ Extension load not updated"
fi

if grep -q "GAGGLE_RUST_LIB" extension_config.cmake; then
    echo "  ✓ Rust lib variable updated"
else
    echo "  ✗ Rust lib variable not updated"
fi

echo ""

# Check Rust files
echo "✓ Checking Rust source files..."
RUST_GAGGLE_COUNT=$(grep -r "gaggle_" gaggle/src/*.rs 2>/dev/null | wc -l)
RUST_INFERA_COUNT=$(grep -r "infera_" gaggle/src/*.rs 2>/dev/null | grep -v "gaggle_cache" | wc -l)

if [ "$RUST_GAGGLE_COUNT" -gt 10 ]; then
    echo "  ✓ Rust functions renamed ($RUST_GAGGLE_COUNT gaggle_ references)"
else
    echo "  ⚠ Few gaggle_ references found ($RUST_GAGGLE_COUNT)"
fi

if [ "$RUST_INFERA_COUNT" -eq 0 ]; then
    echo "  ✓ No infera_ references in Rust code"
else
    echo "  ⚠ Still $RUST_INFERA_COUNT infera_ references in Rust code"
fi

echo ""

# Check C++ files
echo "✓ Checking C++ bindings..."
if grep -q "class GaggleExtension" gaggle/bindings/include/gaggle_extension.hpp; then
    echo "  ✓ C++ class renamed to GaggleExtension"
else
    echo "  ✗ C++ class not renamed"
fi

if grep -q "gaggle::gaggle_" gaggle/bindings/gaggle_extension.cpp; then
    echo "  ✓ C++ namespace calls updated"
else
    echo "  ✗ C++ namespace calls not updated"
fi

echo ""

# Check for remaining infera references (excluding expected files)
echo "✓ Checking for remaining 'infera' references..."
INFERA_COUNT=$(grep -r "infera" \
    --exclude-dir=external \
    --exclude-dir=target \
    --exclude-dir=.git \
    --exclude="*.svg" \
    --exclude="*.lock" \
    --exclude="rust.h" \
    --exclude="verify_rename.sh" \
    --exclude="RENAME_SUMMARY.md" \
    . 2>/dev/null | \
    grep -v "Binary file" | \
    grep -v "docs/" | \
    grep -v "test/" | \
    wc -l)

if [ "$INFERA_COUNT" -eq 0 ]; then
    echo "  ✓ No unexpected infera references in core files"
else
    echo "  ⚠ Found $INFERA_COUNT infera references in core files (excluding docs/tests)"
    echo "    Run: grep -r 'infera' --exclude-dir={external,target,.git} --exclude='*.svg' --exclude='*.lock' --exclude='rust.h' . | grep -v docs/ | grep -v test/"
fi

echo ""
echo "========================================="
echo "Next Steps:"
echo "========================================="
echo "1. Regenerate C bindings:"
echo "   make create-bindings"
echo ""
echo "2. Update documentation and tests:"
echo "   - docs/README.md"
echo "   - docs/CONFIGURATION.md"
echo "   - docs/examples/*.sql"
echo "   - test/sql/*.test"
echo "   - test/concurrency/*.py"
echo ""
echo "3. Build and test:"
echo "   make release"
echo "   make test"
echo ""
