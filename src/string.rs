use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::{Error, Result};
use crate::value::Value;
use crate::Runtime;

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

    /// Extract the contents as a Rust `String`.
    pub fn to_rust_string(&self) -> Result<String> {
        // First call to get the required buffer size.
        let needed =
            unsafe { hermes__String__ToUtf8(self.rt, self.pv, std::ptr::null_mut(), 0) };
        if needed == 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u8; needed];
        unsafe {
            hermes__String__ToUtf8(
                self.rt,
                self.pv,
                buf.as_mut_ptr() as *mut i8,
                buf.len(),
            );
        }
        String::from_utf8(buf).map_err(|e| Error::RuntimeError(e.to_string()))
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
        let val = Value {
            raw: HermesValue {
                kind: HermesValueKind_String,
                data: HermesValueData { pointer: s.pv },
            },
            rt: s.rt,
            _marker: PhantomData,
        };
        std::mem::forget(s); // ownership transferred to Value
        val
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
