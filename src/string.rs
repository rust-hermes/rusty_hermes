use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::{Error, Result};
use crate::value::Value;
use crate::Runtime;

/// Read a JS string pointer value into a Rust `String`.
///
/// Calls `hermes__String__ToUtf8` twice: once for the byte length, once to
/// fill the buffer. Returns `Err` if the bytes are not valid UTF-8.
pub(crate) fn pv_to_rust_string(
    rt: *mut HermesRt,
    pv: *const std::ffi::c_void,
) -> Result<String> {
    let needed = unsafe { hermes__String__ToUtf8(rt, pv, std::ptr::null_mut(), 0) };
    if needed == 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; needed];
    unsafe {
        hermes__String__ToUtf8(rt, pv, buf.as_mut_ptr() as *mut i8, buf.len());
    }
    String::from_utf8(buf).map_err(|e| Error::RuntimeError(e.to_string()))
}

/// Read a JS string pointer value into a Rust `String`, using lossy
/// conversion for invalid UTF-8.
pub(crate) fn pv_to_rust_string_lossy(
    rt: *mut HermesRt,
    pv: *const std::ffi::c_void,
) -> String {
    let needed = unsafe { hermes__String__ToUtf8(rt, pv, std::ptr::null_mut(), 0) };
    if needed == 0 {
        return String::new();
    }
    let mut buf = vec![0u8; needed];
    unsafe {
        hermes__String__ToUtf8(rt, pv, buf.as_mut_ptr() as *mut i8, buf.len());
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// A JavaScript string handle.
pub struct JsString<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> JsString<'rt> {
    /// Create a JS string from a Rust `&str`.
    pub fn new(rt: &'rt Runtime, s: &str) -> Self {
        let pv = unsafe {
            hermes__String__CreateFromUtf8(rt.raw, s.as_ptr(), s.len())
        };
        JsString {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Create a JS string from an ASCII `&str`.
    ///
    /// Slightly more efficient than [`new`](Self::new) when the input is known
    /// to be pure ASCII.
    pub fn from_ascii(rt: &'rt Runtime, s: &str) -> Self {
        let pv = unsafe {
            hermes__String__CreateFromAscii(rt.raw, s.as_ptr() as *const i8, s.len())
        };
        JsString {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Extract the contents as a Rust `String`.
    pub fn to_rust_string(&self) -> Result<String> {
        pv_to_rust_string(self.rt, self.pv)
    }

    pub fn strict_equals(&self, other: &JsString<'rt>) -> bool {
        unsafe { hermes__String__StrictEquals(self.rt, self.pv, other.pv) }
    }
}

impl Drop for JsString<'_> {
    fn drop(&mut self) {
        unsafe { hermes__String__Release(self.pv) }
    }
}

// -- Into<Value> / TryFrom<Value> ---------------------------------------------

impl<'rt> From<JsString<'rt>> for Value<'rt> {
    fn from(s: JsString<'rt>) -> Value<'rt> {
        let s = std::mem::ManuallyDrop::new(s);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_String,
                data: HermesValueData { pointer: s.pv },
            },
            rt: s.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for JsString<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> Result<Self> {
        val.into_string()
    }
}

impl std::fmt::Debug for JsString<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_rust_string() {
            Ok(s) => write!(f, "JsString({s:?})"),
            Err(_) => write!(f, "JsString(<error>)"),
        }
    }
}
