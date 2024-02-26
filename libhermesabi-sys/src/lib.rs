//! # Example
//!
//! ```rust
//! use hermesabi_sys::*;
//! use std::ffi::CString;
//!
//! unsafe extern "C" fn release_wrapper(_buf: *mut HermesABIBuffer) {}
//!
//! fn main() {
//!     unsafe {
//!         let vtable_ptr = get_hermes_abi_vtable();
//!         let vtable = &*vtable_ptr;
//!
//!         let config = std::ptr::null();
//!         let runtime_ptr = (vtable.make_hermes_runtime.unwrap())(config);
//!
//!         let runtime = &*runtime_ptr;
//!         let runtime_vt = &*runtime.vt;
//!
//!         let script = String::from("x = 1 + 2");
//!         let script_url = CString::new("./src/test.js").expect("CString::new failed");
//!
//!         let vtable = HermesABIBufferVTable {
//!             release: Some(release_wrapper),
//!         };
//!         let mut x = HermesABIBuffer {
//!             vtable: &vtable,
//!             data: script.as_ptr(),
//!             size: script.len(),
//!         };
//!
//!         let buffer_ptr = &mut x as *mut HermesABIBuffer;
//!
//!         let eval = runtime_vt.evaluate_javascript_source.unwrap();
//!         let v = eval(
//!             runtime_ptr,
//!             buffer_ptr,
//!             script_url.as_ptr(),
//!             script_url.as_bytes().len(),
//!         );
//!
//!         assert_eq!(v.value.data.number, 3.0);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
