include_directories(${CMAKE_CURRENT_LIST_DIR}/gaggle/bindings/include)

duckdb_extension_load(gaggle
    SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}
    LOAD_TESTS
)

# Manually link the pre-built Rust static library into the generated extension targets.
# The Rust library is built beforehand by the Makefile (cargo build --release --features duckdb_extension).
set(GAGGLE_RUST_LIB ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/release/libgaggle.a)

# Collect candidate paths in priority order
set(_GAGGLE_RUST_CANDIDATES)

# 1. If CARGO_TARGET_DIR env var set (relative like target/<triple> or absolute) add its release path
if(DEFINED ENV{CARGO_TARGET_DIR})
    set(_CTD $ENV{CARGO_TARGET_DIR})
    # Normalize possible relative path (cargo executed from gaggle crate dir)
    if(NOT IS_ABSOLUTE "${_CTD}")
        set(_CTD_FULL ${CMAKE_CURRENT_LIST_DIR}/gaggle/${_CTD})
    else()
        set(_CTD_FULL ${_CTD})
    endif()
    list(APPEND _GAGGLE_RUST_CANDIDATES ${_CTD_FULL}/release/libgaggle.a ${_CTD_FULL}/release/gaggle.lib)
endif()

# 2. Target-triple specific directory if Rust_CARGO_TARGET defined
if(DEFINED Rust_CARGO_TARGET AND NOT "${Rust_CARGO_TARGET}" STREQUAL "")
    list(APPEND _GAGGLE_RUST_CANDIDATES
        ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/${Rust_CARGO_TARGET}/release/libgaggle.a
        ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/${Rust_CARGO_TARGET}/release/gaggle.lib)
endif()

# 3. Default host target release dir (pre-CARGO_TARGET_DIR layout)
list(APPEND _GAGGLE_RUST_CANDIDATES ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/release/libgaggle.a ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/release/gaggle.lib)

# Select first existing candidate
foreach(_cand IN LISTS _GAGGLE_RUST_CANDIDATES)
    if(EXISTS ${_cand})
        set(GAGGLE_RUST_LIB ${_cand})
        message(STATUS "[gaggle] Selected Rust static library: ${GAGGLE_RUST_LIB}")
        break()
    endif()
endforeach()

# If the expected default path does not exist (common on Windows MSVC or when using target triples),
# try alternate locations and naming conventions.
if(NOT EXISTS ${GAGGLE_RUST_LIB})
    # Look for MSVC-style static library name (gaggle.lib) in the release root
    if (EXISTS ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/release/gaggle.lib)
        set(GAGGLE_RUST_LIB ${CMAKE_CURRENT_LIST_DIR}/gaggle/target/release/gaggle.lib)
    else()
        # Glob any target-triple subdirectory build products (first match wins)
        file(GLOB _GAGGLE_ALT_LIBS
            "${CMAKE_CURRENT_LIST_DIR}/gaggle/target/*/release/libgaggle.a"
            "${CMAKE_CURRENT_LIST_DIR}/gaggle/target/*/release/gaggle.lib"
        )
        list(LENGTH _GAGGLE_ALT_LIBS _ALT_COUNT)
        if(_ALT_COUNT GREATER 0)
            list(GET _GAGGLE_ALT_LIBS 0 GAGGLE_RUST_LIB)
            message(STATUS "[gaggle] Using alternate discovered Rust library: ${GAGGLE_RUST_LIB}")
        endif()
    endif()
endif()

if (EXISTS ${GAGGLE_RUST_LIB})
    message(STATUS "[gaggle] Found Rust library at: ${GAGGLE_RUST_LIB}")

    # Create an imported target for the Rust library
    add_library(gaggle_rust STATIC IMPORTED GLOBAL)
    if(UNIX)
        # We always use pthread, dl, and m on Unix
        # liblzma will be linked separately via link_libraries() below
        set(_GAGGLE_RUST_LINK_LIBS "pthread;dl;m")
    else()
        set(_GAGGLE_RUST_LINK_LIBS "")
    endif()
    set_target_properties(gaggle_rust PROPERTIES
        IMPORTED_LOCATION ${GAGGLE_RUST_LIB}
        INTERFACE_LINK_LIBRARIES "${_GAGGLE_RUST_LINK_LIBS}"
    )

    # Add the Rust library to global link libraries so it gets linked to everything
    if(UNIX)
        link_libraries(${GAGGLE_RUST_LIB} pthread dl m)
        # Link against liblzma - the xz submodule target will be used if available,
        # otherwise falls back to system lzma. The target is typically added by the
        # main CMakeLists.txt before extension targets are built.
        if(TARGET liblzma)
            link_libraries(liblzma)
            message(STATUS "[gaggle] Linking against xz submodule's liblzma target")
        else()
            link_libraries(lzma)
            message(STATUS "[gaggle] Linking against system lzma (xz submodule target not yet available)")
        endif()
    else()
        link_libraries(${GAGGLE_RUST_LIB})
        if(WIN32)
            # Explicitly add Windows system libraries required by Rust dependencies (mio, std I/O paths)
            # Nt* symbols come from ntdll; others from userenv/dbghelp; bcrypt often pulled by crypto backends.
            set(_GAGGLE_WIN_SYSTEM_LIBS ntdll userenv dbghelp bcrypt)
            link_libraries(${_GAGGLE_WIN_SYSTEM_LIBS})
            target_link_libraries(gaggle_rust INTERFACE ${_GAGGLE_WIN_SYSTEM_LIBS})
        endif()
    endif()

    if(TARGET gaggle_extension)
        target_link_libraries(gaggle_extension gaggle_rust)
        message(STATUS "[gaggle] Linked Rust library to gaggle_extension")
    endif()

    if(TARGET gaggle_loadable_extension)
        target_link_libraries(gaggle_loadable_extension gaggle_rust)
        message(STATUS "[gaggle] Linked Rust library to gaggle_loadable_extension")
    endif()

    if(UNIX)
        add_link_options($<$<STREQUAL:$<TARGET_PROPERTY:TYPE>,EXECUTABLE>:${GAGGLE_RUST_LIB}>)
        add_link_options($<$<STREQUAL:$<TARGET_PROPERTY:TYPE>,EXECUTABLE>:-lpthread>)
        add_link_options($<$<STREQUAL:$<TARGET_PROPERTY:TYPE>,EXECUTABLE>:-ldl>)
        add_link_options($<$<STREQUAL:$<TARGET_PROPERTY:TYPE>,EXECUTABLE>:-lm>)
        # Note: liblzma linking is handled via link_libraries(liblzma) above
    else()
        add_link_options($<$<STREQUAL:$<TARGET_PROPERTY:TYPE>,EXECUTABLE>:${GAGGLE_RUST_LIB}>)
    endif()
else()
    message(WARNING "[gaggle] Expected Rust static library not found at ${GAGGLE_RUST_LIB}. Build Rust crate first.")
endif()
