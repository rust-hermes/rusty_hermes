use cmake::Config;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

fn main() {
    let hermes_src_dir = "hermes";
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let hermes_src = manifest_dir.join(hermes_src_dir);

    println!("cargo:rerun-if-changed=src/binding.cc");
    println!("cargo:rerun-if-changed=src/binding.hpp");
    println!("cargo:rerun-if-changed={}/", hermes_src_dir);

    // Build Hermes via cmake, targeting hermesvm_a (static VM with compiler).
    // All libraries are built as static archives.
    //
    // `.generator()` (not `.configure_arg("-G Ninja")`) — the `cmake` crate
    // only recognizes a Ninja generator through its own `generator` field: on
    // MSVC targets it otherwise assumes a Visual Studio generator and appends
    // `-Thost=x64 -Ax64`, which conflicts with a Ninja generator supplied as
    // a raw configure arg ("Generator Ninja does not support platform
    // specification"). See cmake-rs's `Config::build()` generator handling.
    let hermes_build = Config::new(hermes_src_dir)
        .build_target("hermesvm_a")
        .generator("Ninja")
        // Without this, `cmake`-rs infers CMAKE_BUILD_TYPE from Cargo's own
        // profile — "Debug" for a plain `cargo build`. On MSVC that pulls in
        // CMake's default `/MDd` (debug CRT) flags for Hermes' own object
        // files, while Rust's linked output always expects the release CRT
        // (`/MD`) regardless of `--release` — the mismatch surfaces as
        // dozens of "unresolved external symbol __imp__calloc_dbg" (and
        // other debug-CRT-only symbols) at final link. Building the vendored
        // Hermes in Release always sidesteps it, on every platform — a
        // vendored C++ VM built unoptimized to match our own dev profile
        // buys nothing (we never edit it) and would just make it slower.
        .profile("Release")
        .define("HERMES_ENABLE_EH_RTTI", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("HERMES_BUILD_SHARED_JSI", "OFF")
        .define("HERMES_BUILD_APPLE_FRAMEWORK", "OFF")
        .build();

    let hermes_build_dir = format!("{}/build", hermes_build.display());

    // Expose the build directory to dependent crates via DEP_HERMES_BUILD_DIR.
    println!("cargo:build_dir={}", hermes_build_dir);

    // Compile our C++ binding layer with the cc crate.
    cc::Build::new()
        .cpp(true)
        .file("src/binding.cc")
        .include(hermes_src.join("API"))
        .include(hermes_src.join("API/jsi"))
        .include(hermes_src.join("public"))
        .include("src")
        // `.flag()` passes these through verbatim, so the GCC/Clang spellings
        // silently do nothing under MSVC (cl.exe warns and ignores unknown
        // "-"-prefixed args instead of erroring) — binding.cc then compiles
        // in whatever pre-C++17 mode cl.exe defaults to, and fails on
        // structured bindings. `flag_if_supported` probes each spelling
        // against the actual compiler in use, so both sides get a real flag.
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-fexceptions")
        .flag_if_supported("/EHsc")
        .flag_if_supported("-frtti")
        .flag_if_supported("/GR")
        .compile("hermes_binding");

    // Discover and link all static libraries produced by the Hermes build.
    // Static archives don't embed transitive dependencies, so we need to link
    // every archive CMake produced (the linker discards unused symbols).
    // MSVC's static-library archives are named "*.lib", not "*.a" — a
    // Windows build previously found none at all here (no hard error from
    // that alone: it only surfaces once the linker fails to resolve Hermes'
    // own symbols, downstream of other errors that used to mask it).
    let lib_ext = if cfg!(target_os = "windows") { "lib" } else { "a" };
    let build_path = PathBuf::from(&hermes_build_dir);
    let mut search_dirs = HashSet::new();

    for entry in walkdir(&build_path) {
        if let Some(ext) = entry.extension() {
            if ext == lib_ext {
                let dir = entry.parent().unwrap();
                if search_dirs.insert(dir.to_path_buf()) {
                    println!("cargo:rustc-link-search=native={}", dir.display());
                }
                // Strip the "lib" prefix (POSIX archives only — MSVC .lib
                // files aren't prefixed) and extension to get the link name.
                let stem = entry.file_stem().unwrap().to_str().unwrap();
                let name = if cfg!(target_os = "windows") { stem } else { stem.strip_prefix("lib").unwrap_or(stem) };
                println!("cargo:rustc-link-lib=static={}", name);
            }
        }
    }

    // Link system libraries.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
        // Hermes's Unicode support (PlatformUnicodeCF) uses CoreFoundation.
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=stdc++");
        // Hermes's Unicode support (PlatformUnicodeICU) uses ICU on Linux.
        println!("cargo:rustc-link-lib=icuuc");
        println!("cargo:rustc-link-lib=icui18n");
        println!("cargo:rustc-link-lib=icudata");
    }
    // Windows: MSVC's own C++ runtime is linked automatically (no "stdc++"
    // by that name exists there), and CMake reported "Using Windows 10
    // built-in ICU" — Hermes' own Windows Unicode shim is just another
    // static archive the walk above already discovers and links, not a
    // separate system library to name here.
}

/// Recursively walk a directory and yield all file paths.
fn walkdir(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}
