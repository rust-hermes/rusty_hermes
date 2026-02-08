use std::marker::PhantomData;

use libhermes_sys::*;

use crate::error::Result;
use crate::{JsString, Runtime, Symbol};

/// A JavaScript property name identifier.
///
/// Can be created from a UTF-8 string, ASCII string, [`JsString`], or [`Symbol`].
pub struct PropNameId<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> PropNameId<'rt> {
    /// Create a property name from a UTF-8 string.
    pub fn from_utf8(rt: &'rt Runtime, s: &str) -> Self {
        let pv = unsafe { hermes__PropNameID__ForUtf8(rt.raw, s.as_ptr(), s.len()) };
        PropNameId {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Create a property name from an ASCII string.
    pub fn from_ascii(rt: &'rt Runtime, s: &str) -> Self {
        let pv = unsafe { hermes__PropNameID__ForAscii(rt.raw, s.as_ptr() as *const i8, s.len()) };
        PropNameId {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Create a property name from a [`JsString`].
    pub fn from_string(rt: &'rt Runtime, s: &JsString<'rt>) -> Self {
        let pv = unsafe { hermes__PropNameID__ForString(rt.raw, s.pv) };
        PropNameId {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Create a property name from a [`Symbol`].
    pub fn from_symbol(rt: &'rt Runtime, sym: &Symbol<'rt>) -> Self {
        let pv = unsafe { hermes__PropNameID__ForSymbol(rt.raw, sym.pv) };
        PropNameId {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Get the UTF-8 string representation.
    pub fn to_rust_string(&self) -> Result<String> {
        let needed =
            unsafe { hermes__PropNameID__ToUtf8(self.rt, self.pv, std::ptr::null_mut(), 0) };
        if needed == 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u8; needed];
        unsafe {
            hermes__PropNameID__ToUtf8(self.rt, self.pv, buf.as_mut_ptr() as *mut i8, buf.len());
        }
        String::from_utf8(buf).map_err(|e| crate::error::Error::RuntimeError(e.to_string()))
    }

    /// Check if two property names are equal.
    pub fn equals(&self, other: &PropNameId<'rt>) -> bool {
        unsafe { hermes__PropNameID__Equals(self.rt, self.pv, other.pv) }
    }

    /// Get the unique ID for this property name (Hermes-specific).
    pub fn unique_id(&self) -> u64 {
        unsafe { hermes__PropNameID__GetUniqueID(self.rt, self.pv) }
    }
}

impl Drop for PropNameId<'_> {
    fn drop(&mut self) {
        unsafe { hermes__PropNameID__Release(self.pv) }
    }
}

impl std::fmt::Debug for PropNameId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_rust_string() {
            Ok(s) => write!(f, "PropNameId({s:?})"),
            Err(_) => write!(f, "PropNameId(<error>)"),
        }
    }
}
