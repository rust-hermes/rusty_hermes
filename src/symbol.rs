use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::Error;
use crate::value::Value;
use crate::JsString;

/// A JavaScript symbol handle.
pub struct Symbol<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Symbol<'rt> {
    /// Get the string description of this symbol.
    pub fn to_js_string(&self) -> JsString<'rt> {
        let pv = unsafe { hermes__Symbol__ToString(self.rt, self.pv) };
        JsString {
            pv,
            rt: self.rt,
            _marker: PhantomData,
        }
    }

    pub fn strict_equals(&self, other: &Symbol<'rt>) -> bool {
        unsafe { hermes__Symbol__StrictEquals(self.rt, self.pv, other.pv) }
    }
}

impl Drop for Symbol<'_> {
    fn drop(&mut self) {
        unsafe { hermes__Symbol__Release(self.pv) }
    }
}

impl<'rt> From<Symbol<'rt>> for Value<'rt> {
    fn from(s: Symbol<'rt>) -> Value<'rt> {
        let s = std::mem::ManuallyDrop::new(s);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Symbol,
                data: HermesValueData { pointer: s.pv },
            },
            rt: s.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for Symbol<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> crate::error::Result<Self> {
        val.into_symbol()
    }
}

impl std::fmt::Debug for Symbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({:?})", self.pv)
    }
}
