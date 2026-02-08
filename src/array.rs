use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::{check_error, Error, Result};
use crate::value::Value;
use crate::Runtime;

/// A JavaScript array handle.
pub struct Array<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Array<'rt> {
    /// Create a new JS array with the given length (elements are `undefined`).
    pub fn new(rt: &'rt Runtime, len: usize) -> Self {
        let pv = unsafe { hermes__Array__New(rt.raw, len) };
        Array {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        unsafe { hermes__Array__Size(self.rt, self.pv) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the value at `index`.
    pub fn get(&self, index: usize) -> Result<Value<'rt>> {
        let raw = unsafe { hermes__Array__GetValueAtIndex(self.rt, self.pv, index) };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    /// Set the value at `index`.
    pub fn set(&self, index: usize, val: Value<'rt>) -> Result<()> {
        let ok = unsafe {
            hermes__Array__SetValueAtIndex(self.rt, self.pv, index, &val.raw)
        };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }
}

impl Drop for Array<'_> {
    fn drop(&mut self) {
        unsafe { hermes__Array__Release(self.pv) }
    }
}

// -- Into<Value> / TryFrom<Value> ---------------------------------------------

impl<'rt> From<Array<'rt>> for Value<'rt> {
    fn from(arr: Array<'rt>) -> Value<'rt> {
        let arr = std::mem::ManuallyDrop::new(arr);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: arr.pv },
            },
            rt: arr.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for Array<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> Result<Self> {
        val.into_array()
    }
}

impl std::fmt::Debug for Array<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Array(len={})", self.len())
    }
}
