# libhermesabi-sys

> Note: Currently only supports on Linux and macOS. Adding Windows support should be trivial.

This crate contains bindgen-generated Rust bindings for [Hermes](https://hermesengine.dev) C ABI.

Install the required dependencies:

**Ubuntu**

```
apt install cmake git ninja-build libicu-dev python zip libreadline-dev
```

**Arch**

```
pacman -S cmake git ninja icu python zip readline
```

**macOS via Homebrew**

```
brew install cmake git ninja
```

Add to your **Cargo.toml**:

```
libhermesabi-sys = { git = "https://github.com/rust-hermes/rusty_hermes" path = "libhermesabi-sys" branch = "main" }
```

## Examples

```rust
use libhermesabi_sys::*;
use std::ffi::CString;

unsafe extern "C" fn release_wrapper(_buf: *mut HermesABIBuffer) {}

fn main() {
    unsafe {
        let vtable_ptr = get_hermes_abi_vtable();
        let vtable = &*vtable_ptr;

        let config = std::ptr::null();
        let runtime_ptr = (vtable.make_hermes_runtime.unwrap())(config);

        let runtime = &*runtime_ptr;
        let runtime_vt = &*runtime.vt;

        let script = String::from("x = 1 + 2");
        let script_url = CString::new("./src/test.js").expect("CString::new failed");

        let vtable = HermesABIBufferVTable {
            release: Some(release_wrapper),
        };

        let mut x = HermesABIBuffer {
            vtable: &vtable,
            data: script.as_ptr(),
            size: script.len(),
        };

        let buffer_ptr = &mut x as *mut HermesABIBuffer;

        let eval = runtime_vt.evaluate_javascript_source.unwrap();
        let v = eval(
            runtime_ptr,
            buffer_ptr,
            script_url.as_ptr(),
            script_url.as_bytes().len(),
        );

        assert_eq!(v.value.data.number, 3.0);
    }
}
```
