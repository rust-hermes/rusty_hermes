use cmake::Config;
use std::env;
use std::path::PathBuf;

fn main() {
    let hermes_src_dir = "hermes";
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let hermes_src = manifest_dir.join(hermes_src_dir);

    println!("cargo:rerun-if-changed=src/binding.cc");
    println!("cargo:rerun-if-changed=src/binding.hpp");
    println!("cargo:rerun-if-changed={}/", hermes_src_dir);

    // Build Hermes via cmake, targeting libhermes (JSI implementation).
    // On macOS this produces hermes.framework by default.
    let hermes_build = Config::new(hermes_src_dir)
        .build_target("libhermes")
        .configure_arg("-G Ninja")
        .define("HERMES_ENABLE_EH_RTTI", "ON")
        .build();

    let hermes_build_dir = format!("{}/build", hermes_build.display());

    // Expose the build directory to dependent crates via DEP_HERMESABI_BUILD_DIR.
    println!("cargo:build_dir={}", hermes_build_dir);

    // Compile our C++ binding layer with the cc crate.
    cc::Build::new()
        .cpp(true)
        .file("src/binding.cc")
        .include(hermes_src.join("API"))
        .include(hermes_src.join("API/jsi"))
        .include(hermes_src.join("public"))
        .include("src")
        .flag("-std=c++17")
        .flag("-fexceptions")
        .flag("-frtti")
        .compile("hermes_binding");

    // Link against the hermes framework (macOS) or shared library,
    // plus the jsi static library for JSI symbols.
    if cfg!(target_os = "macos") {
        // On macOS, cmake builds hermes.framework in API/hermes/.
        println!(
            "cargo:rustc-link-search=framework={}/API/hermes",
            hermes_build_dir
        );
        println!("cargo:rustc-link-lib=framework=hermes");

        // Also link JSI shared lib (framework doesn't re-export all symbols).
        println!(
            "cargo:rustc-link-search=native={}/jsi",
            hermes_build_dir
        );
        println!("cargo:rustc-link-lib=dylib=jsi");

        // Set rpath so the framework/dylibs can be found at runtime.
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}/API/hermes",
            hermes_build_dir
        );
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}/jsi",
            hermes_build_dir
        );
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!(
            "cargo:rustc-link-search=native={}/API/hermes",
            hermes_build_dir
        );
        println!("cargo:rustc-link-lib=dylib=hermes");
        println!("cargo:rustc-link-lib=stdc++");
    }
}
