use bindgen::Builder;
use cmake::Config;
use std::env;
use std::path::PathBuf;

fn main() {
    let hermes_src_dir = "hermes";

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={}/", hermes_src_dir);

    // Set up the build
    let hermes_build = Config::new(hermes_src_dir)
        .build_target("hermesabi")
        .configure_arg("-G Ninja")
        .build();

    let hermes_build_dir = format!("{}/build", hermes_build.display());

    // Configure bindgen
    let bindings = Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}/API/hermes_abi", hermes_src_dir))
        .allowlist_function(".*") // Avoids junk
        .layout_tests(false)
        // .rustified_enum(".*") // enums: HermesABIValueKind, HermesABIErrorCode
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Add link paths and libraries
    println!(
        "cargo:rustc-link-search=native={}/API/hermes_abi",
        hermes_build_dir
    );
    println!("cargo:rustc-link-lib=dylib=hermesabi");
}
