//! Stable C bindings for Hermes JavaScript engine (rusty_v8 style).
//!
//! This crate provides flat `extern "C"` functions wrapping the Hermes JSI
//! C++ API. Pointer types (String, Object, etc.) are represented as opaque
//! `*mut c_void` handles that must be explicitly released.
//!
//! # Example
//!
//! ```rust,no_run
//! use libhermesabi_sys::*;
//!
//! fn main() {
//!     unsafe {
//!         let rt = hermes__Runtime__New();
//!
//!         let script = b"1 + 2";
//!         let url = b"test.js";
//!
//!         let result = hermes__Runtime__EvaluateJavaScript(
//!             rt,
//!             script.as_ptr(),
//!             script.len(),
//!             url.as_ptr() as *const i8,
//!             url.len(),
//!         );
//!
//!         assert_eq!(result.kind, HermesValueKind_Number);
//!         assert_eq!(result.data.number, 3.0);
//!
//!         hermes__Runtime__Delete(rt);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]

use std::os::raw::c_char;

/// Opaque runtime handle.
#[repr(C)]
pub struct HermesRt {
    _private: [u8; 0],
}

/// Value kind tags, matching jsi::Value::ValueKind.
pub const HermesValueKind_Undefined: i32 = 0;
pub const HermesValueKind_Null: i32 = 1;
pub const HermesValueKind_Boolean: i32 = 2;
pub const HermesValueKind_Number: i32 = 3;
pub const HermesValueKind_Symbol: i32 = 4;
pub const HermesValueKind_BigInt: i32 = 5;
pub const HermesValueKind_String: i32 = 6;
pub const HermesValueKind_Object: i32 = 7;

/// C-compatible tagged union mirroring jsi::Value.
/// For pointer kinds (Symbol, BigInt, String, Object), `data.pointer` holds
/// a `PointerValue*` that must be released via the appropriate `Release` fn
/// or `hermes__Value__Release`.
#[repr(C)]
pub struct HermesValue {
    pub kind: i32,
    pub data: HermesValueData,
}

#[repr(C)]
pub union HermesValueData {
    pub boolean: bool,
    pub number: f64,
    pub pointer: *mut std::ffi::c_void,
}

/// Host function callback signature.
pub type HermesHostFunctionCallback = unsafe extern "C" fn(
    rt: *mut HermesRt,
    this_val: *const HermesValue,
    args: *const HermesValue,
    arg_count: usize,
    user_data: *mut std::ffi::c_void,
) -> HermesValue;

/// Called when a host function's closure is garbage collected.
pub type HermesHostFunctionFinalizer =
    unsafe extern "C" fn(user_data: *mut std::ffi::c_void);

unsafe extern "C" {
    // -----------------------------------------------------------------------
    // Runtime lifecycle
    // -----------------------------------------------------------------------

    pub fn hermes__Runtime__New() -> *mut HermesRt;
    pub fn hermes__Runtime__Delete(rt: *mut HermesRt);

    pub fn hermes__Runtime__HasPendingError(rt: *const HermesRt) -> bool;
    pub fn hermes__Runtime__GetAndClearError(rt: *mut HermesRt) -> HermesValue;
    /// Returns a malloc'd C string the caller must free, or null.
    pub fn hermes__Runtime__GetAndClearErrorMessage(
        rt: *mut HermesRt,
    ) -> *const c_char;

    pub fn hermes__Runtime__Global(rt: *mut HermesRt) -> *mut std::ffi::c_void;

    // -----------------------------------------------------------------------
    // Evaluate
    // -----------------------------------------------------------------------

    pub fn hermes__Runtime__EvaluateJavaScript(
        rt: *mut HermesRt,
        data: *const u8,
        len: usize,
        source_url: *const c_char,
        source_url_len: usize,
    ) -> HermesValue;

    /// Returns: 1 = drained, 0 = more work, -1 = error.
    pub fn hermes__Runtime__DrainMicrotasks(
        rt: *mut HermesRt,
        max_hint: i32,
    ) -> i32;

    // -----------------------------------------------------------------------
    // String
    // -----------------------------------------------------------------------

    pub fn hermes__String__CreateFromUtf8(
        rt: *mut HermesRt,
        utf8: *const u8,
        len: usize,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__String__CreateFromAscii(
        rt: *mut HermesRt,
        ascii: *const c_char,
        len: usize,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__String__ToUtf8(
        rt: *mut HermesRt,
        str: *const std::ffi::c_void,
        buf: *mut c_char,
        buf_len: usize,
    ) -> usize;

    pub fn hermes__String__StrictEquals(
        rt: *mut HermesRt,
        a: *const std::ffi::c_void,
        b: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__String__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // PropNameID
    // -----------------------------------------------------------------------

    pub fn hermes__PropNameID__ForAscii(
        rt: *mut HermesRt,
        str: *const c_char,
        len: usize,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__PropNameID__ForUtf8(
        rt: *mut HermesRt,
        utf8: *const u8,
        len: usize,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__PropNameID__ForString(
        rt: *mut HermesRt,
        str: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__PropNameID__ToUtf8(
        rt: *mut HermesRt,
        pni: *const std::ffi::c_void,
        buf: *mut c_char,
        buf_len: usize,
    ) -> usize;

    pub fn hermes__PropNameID__Equals(
        rt: *mut HermesRt,
        a: *const std::ffi::c_void,
        b: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__PropNameID__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // Object
    // -----------------------------------------------------------------------

    pub fn hermes__Object__New(
        rt: *mut HermesRt,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__Object__GetProperty__String(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
    ) -> HermesValue;

    pub fn hermes__Object__GetProperty__PropNameID(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
    ) -> HermesValue;

    pub fn hermes__Object__SetProperty__String(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
        val: *const HermesValue,
    ) -> bool;

    pub fn hermes__Object__SetProperty__PropNameID(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
        val: *const HermesValue,
    ) -> bool;

    pub fn hermes__Object__HasProperty__String(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__HasProperty__PropNameID(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__GetPropertyNames(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__Object__IsArray(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__IsFunction(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__IsArrayBuffer(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__StrictEquals(
        rt: *mut HermesRt,
        a: *const std::ffi::c_void,
        b: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__InstanceOf(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
        func: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Object__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // Array
    // -----------------------------------------------------------------------

    pub fn hermes__Array__New(
        rt: *mut HermesRt,
        length: usize,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__Array__Size(
        rt: *mut HermesRt,
        arr: *const std::ffi::c_void,
    ) -> usize;

    pub fn hermes__Array__GetValueAtIndex(
        rt: *mut HermesRt,
        arr: *const std::ffi::c_void,
        index: usize,
    ) -> HermesValue;

    pub fn hermes__Array__SetValueAtIndex(
        rt: *mut HermesRt,
        arr: *const std::ffi::c_void,
        index: usize,
        val: *const HermesValue,
    ) -> bool;

    pub fn hermes__Array__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // Function
    // -----------------------------------------------------------------------

    pub fn hermes__Function__Call(
        rt: *mut HermesRt,
        func: *const std::ffi::c_void,
        this_val: *const HermesValue,
        args: *const HermesValue,
        argc: usize,
    ) -> HermesValue;

    pub fn hermes__Function__CallAsConstructor(
        rt: *mut HermesRt,
        func: *const std::ffi::c_void,
        args: *const HermesValue,
        argc: usize,
    ) -> HermesValue;

    pub fn hermes__Function__CreateFromHostFunction(
        rt: *mut HermesRt,
        name: *const std::ffi::c_void,
        param_count: u32,
        callback: HermesHostFunctionCallback,
        user_data: *mut std::ffi::c_void,
        finalizer: HermesHostFunctionFinalizer,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__Function__IsHostFunction(
        rt: *mut HermesRt,
        func: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Function__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // Value
    // -----------------------------------------------------------------------

    pub fn hermes__Value__Release(val: *mut HermesValue);

    pub fn hermes__Value__StrictEquals(
        rt: *mut HermesRt,
        a: *const HermesValue,
        b: *const HermesValue,
    ) -> bool;

    // -----------------------------------------------------------------------
    // Symbol
    // -----------------------------------------------------------------------

    pub fn hermes__Symbol__ToString(
        rt: *mut HermesRt,
        sym: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__Symbol__StrictEquals(
        rt: *mut HermesRt,
        a: *const std::ffi::c_void,
        b: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__Symbol__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // BigInt
    // -----------------------------------------------------------------------

    pub fn hermes__BigInt__FromInt64(
        rt: *mut HermesRt,
        val: i64,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__BigInt__FromUint64(
        rt: *mut HermesRt,
        val: u64,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__BigInt__IsInt64(
        rt: *mut HermesRt,
        bi: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__BigInt__IsUint64(
        rt: *mut HermesRt,
        bi: *const std::ffi::c_void,
    ) -> bool;

    pub fn hermes__BigInt__Truncate(
        rt: *mut HermesRt,
        bi: *const std::ffi::c_void,
    ) -> u64;

    pub fn hermes__BigInt__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // WeakObject
    // -----------------------------------------------------------------------

    pub fn hermes__WeakObject__Create(
        rt: *mut HermesRt,
        obj: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;

    pub fn hermes__WeakObject__Lock(
        rt: *mut HermesRt,
        wo: *const std::ffi::c_void,
    ) -> HermesValue;

    pub fn hermes__WeakObject__Release(pv: *mut std::ffi::c_void);

    // -----------------------------------------------------------------------
    // HermesRuntime-specific (static)
    // -----------------------------------------------------------------------

    pub fn hermes__IsHermesBytecode(data: *const u8, len: usize) -> bool;
    pub fn hermes__GetBytecodeVersion() -> u32;
}
