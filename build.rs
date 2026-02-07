use std::env;

fn main() {
    // Read the Hermes build directory from the sys crate's metadata.
    let build_dir = env::var("DEP_HERMESABI_BUILD_DIR")
        .expect("DEP_HERMESABI_BUILD_DIR not set — libhermesabi-sys must be built first");

    // Set rpath so the framework/dylibs can be found at runtime.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}/API/hermes", build_dir);
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}/jsi", build_dir);
    }
}
