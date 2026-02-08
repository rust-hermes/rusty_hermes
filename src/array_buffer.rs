use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::Error;
use crate::value::Value;
use crate::Runtime;

/// A JavaScript ArrayBuffer handle.
///
/// Provides access to the raw byte buffer backing an `ArrayBuffer`.
pub struct ArrayBuffer<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> ArrayBuffer<'rt> {
    /// Create a new `ArrayBuffer` with the given size in bytes.
    pub fn new(rt: &'rt Runtime, size: usize) -> Self {
        let pv = unsafe { hermes__ArrayBuffer__New(rt.raw, size) };
        ArrayBuffer {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Size of the buffer in bytes.
    pub fn size(&self) -> usize {
        unsafe { hermes__ArrayBuffer__Size(self.rt, self.pv) }
    }

    /// Get a slice to the underlying data.
    ///
    /// # Safety
    /// The returned slice borrows the ArrayBuffer. Do not call into JS
    /// while holding this reference, as the buffer could be detached.
    pub fn data(&self) -> &[u8] {
        let ptr = unsafe { hermes__ArrayBuffer__Data(self.rt, self.pv) };
        let len = self.size();
        if ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }

    /// Get a mutable slice to the underlying data.
    ///
    /// # Safety
    /// The returned slice borrows the ArrayBuffer mutably. Do not call into JS
    /// while holding this reference.
    pub fn data_mut(&mut self) -> &mut [u8] {
        let ptr = unsafe { hermes__ArrayBuffer__Data(self.rt, self.pv) };
        let len = self.size();
        if ptr.is_null() || len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(ptr, len) }
        }
    }
}

impl Drop for ArrayBuffer<'_> {
    fn drop(&mut self) {
        // ArrayBuffer is an Object; release with Object release.
        unsafe { hermes__Object__Release(self.pv) }
    }
}

impl<'rt> From<ArrayBuffer<'rt>> for Value<'rt> {
    fn from(buf: ArrayBuffer<'rt>) -> Value<'rt> {
        let buf = std::mem::ManuallyDrop::new(buf);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: buf.pv },
            },
            rt: buf.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for ArrayBuffer<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> crate::error::Result<Self> {
        val.into_array_buffer()
    }
}

impl std::fmt::Debug for ArrayBuffer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArrayBuffer(size={})", self.size())
    }
}
