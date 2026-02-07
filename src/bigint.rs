use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::Error;
use crate::value::Value;
use crate::Runtime;

/// A JavaScript BigInt handle.
pub struct BigInt<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> BigInt<'rt> {
    pub fn from_i64(rt: &'rt Runtime, val: i64) -> Self {
        let pv = unsafe { hermes__BigInt__FromInt64(rt.raw, val) };
        BigInt {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    pub fn from_u64(rt: &'rt Runtime, val: u64) -> Self {
        let pv = unsafe { hermes__BigInt__FromUint64(rt.raw, val) };
        BigInt {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    pub fn is_i64(&self) -> bool {
        unsafe { hermes__BigInt__IsInt64(self.rt, self.pv) }
    }

    pub fn is_u64(&self) -> bool {
        unsafe { hermes__BigInt__IsUint64(self.rt, self.pv) }
    }

    /// Truncate to a `u64`. Use [`is_u64`](Self::is_u64) to check lossless-ness.
    pub fn truncate_to_u64(&self) -> u64 {
        unsafe { hermes__BigInt__Truncate(self.rt, self.pv) }
    }

    /// Truncate to an `i64`. Use [`is_i64`](Self::is_i64) to check lossless-ness.
    pub fn truncate_to_i64(&self) -> i64 {
        unsafe { hermes__BigInt__Truncate(self.rt, self.pv) as i64 }
    }

    /// Convert to a JS string with the given radix (2-36).
    pub fn to_js_string(&self, radix: i32) -> crate::JsString<'rt> {
        let pv = unsafe { hermes__BigInt__ToString(self.rt, self.pv, radix) };
        crate::JsString {
            pv,
            rt: self.rt,
            _marker: PhantomData,
        }
    }

    /// Check strict equality with another BigInt.
    pub fn strict_equals(&self, other: &BigInt<'rt>) -> bool {
        unsafe { hermes__BigInt__StrictEquals(self.rt, self.pv, other.pv) }
    }
}

impl Drop for BigInt<'_> {
    fn drop(&mut self) {
        unsafe { hermes__BigInt__Release(self.pv) }
    }
}

impl<'rt> From<BigInt<'rt>> for Value<'rt> {
    fn from(bi: BigInt<'rt>) -> Value<'rt> {
        let val = Value {
            raw: HermesValue {
                kind: HermesValueKind_BigInt,
                data: HermesValueData { pointer: bi.pv },
            },
            rt: bi.rt,
            _marker: PhantomData,
        };
        std::mem::forget(bi);
        val
    }
}

impl<'rt> TryFrom<Value<'rt>> for BigInt<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> crate::error::Result<Self> {
        val.into_bigint()
    }
}

impl std::fmt::Debug for BigInt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BigInt({:?})", self.pv)
    }
}
