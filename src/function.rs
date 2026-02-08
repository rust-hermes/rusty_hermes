use std::marker::PhantomData;

use libhermes_sys::*;

use crate::error::{Error, Result, check_error};
use crate::value::Value;

/// A JavaScript function handle.
pub struct Function<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Function<'rt> {
    /// Call this function with `undefined` as `this`.
    pub fn call(&self, args: &[Value<'rt>]) -> Result<Value<'rt>> {
        let c_args: Vec<HermesValue> = args.iter().map(|a| a.raw).collect();
        let this = HermesValue {
            kind: HermesValueKind_Undefined,
            data: HermesValueData { number: 0.0 },
        };
        let raw = unsafe {
            hermes__Function__Call(self.rt, self.pv, &this, c_args.as_ptr(), c_args.len())
        };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    /// Call as a constructor (`new Func(...args)`).
    pub fn call_as_constructor(&self, args: &[Value<'rt>]) -> Result<Value<'rt>> {
        let c_args: Vec<HermesValue> = args.iter().map(|a| a.raw).collect();
        let raw = unsafe {
            hermes__Function__CallAsConstructor(self.rt, self.pv, c_args.as_ptr(), c_args.len())
        };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    /// Call this function with a specific `this` value.
    pub fn call_with_this(&self, this: &Value<'rt>, args: &[Value<'rt>]) -> Result<Value<'rt>> {
        let c_args: Vec<HermesValue> = args.iter().map(|a| a.raw).collect();
        let raw = unsafe {
            hermes__Function__Call(self.rt, self.pv, &this.raw, c_args.as_ptr(), c_args.len())
        };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    pub fn is_host_function(&self) -> bool {
        unsafe { hermes__Function__IsHostFunction(self.rt, self.pv) }
    }
}

impl Drop for Function<'_> {
    fn drop(&mut self) {
        unsafe { hermes__Function__Release(self.pv) }
    }
}

// -- Into<Value> / TryFrom<Value> ---------------------------------------------

impl<'rt> From<Function<'rt>> for Value<'rt> {
    fn from(f: Function<'rt>) -> Value<'rt> {
        let f = std::mem::ManuallyDrop::new(f);
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: f.pv },
            },
            rt: f.rt,
            _marker: PhantomData,
        }
    }
}

impl<'rt> TryFrom<Value<'rt>> for Function<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> Result<Self> {
        val.into_function()
    }
}

impl std::fmt::Debug for Function<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Function({:?})", self.pv)
    }
}

// =============================================================================
// Trampoline support (used by #[hermes_op] generated code)
// =============================================================================

/// Extract a Rust value from a raw `HermesValue` arg during a host call.
///
/// Implemented for primitive types and by `#[derive(FromJs)]`.
pub trait FromJsArg: Sized {
    /// # Safety
    /// `rt` must be a valid, non-null pointer to an active Hermes runtime.
    unsafe fn from_arg(rt: *mut HermesRt, raw: &HermesValue) -> Result<Self>;
}

impl FromJsArg for f64 {
    unsafe fn from_arg(_rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
        if raw.kind == HermesValueKind_Number {
            Ok(unsafe { raw.data.number })
        } else {
            Err(Error::TypeError {
                expected: "number",
                got: crate::value::ValueKind::from_raw(raw.kind).name(),
            })
        }
    }
}

impl FromJsArg for bool {
    unsafe fn from_arg(_rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
        if raw.kind == HermesValueKind_Boolean {
            Ok(unsafe { raw.data.boolean })
        } else {
            Err(Error::TypeError {
                expected: "boolean",
                got: crate::value::ValueKind::from_raw(raw.kind).name(),
            })
        }
    }
}

impl FromJsArg for String {
    unsafe fn from_arg(rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
        if raw.kind != HermesValueKind_String {
            return Err(Error::TypeError {
                expected: "string",
                got: crate::value::ValueKind::from_raw(raw.kind).name(),
            });
        }
        let pv = unsafe { raw.data.pointer };
        crate::string::pv_to_rust_string(rt, pv)
    }
}

macro_rules! impl_from_js_arg_via_f64 {
    ($($ty:ty),*) => { $(
        impl FromJsArg for $ty {
            unsafe fn from_arg(rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> { unsafe {
                f64::from_arg(rt, raw).map(|n| n as $ty)
            }}
        }
    )* };
}

impl_from_js_arg_via_f64!(f32, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

/// Convert a Rust return value into a raw `HermesValue`.
///
/// Implemented for primitive types and by `#[derive(IntoJs)]`.
pub trait IntoJsRet {
    /// # Safety
    /// `rt` must be a valid, non-null pointer to an active Hermes runtime.
    unsafe fn into_ret(self, rt: *mut HermesRt) -> Result<HermesValue>;
}

impl IntoJsRet for () {
    unsafe fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Undefined,
            data: HermesValueData { number: 0.0 },
        })
    }
}

impl IntoJsRet for f64 {
    unsafe fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Number,
            data: HermesValueData { number: self },
        })
    }
}

impl IntoJsRet for bool {
    unsafe fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Boolean,
            data: HermesValueData { boolean: self },
        })
    }
}

impl IntoJsRet for String {
    unsafe fn into_ret(self, rt: *mut HermesRt) -> Result<HermesValue> {
        let pv = unsafe { hermes__String__CreateFromUtf8(rt, self.as_ptr(), self.len()) };
        Ok(HermesValue {
            kind: HermesValueKind_String,
            data: HermesValueData { pointer: pv },
        })
    }
}

macro_rules! impl_into_js_ret_via_f64 {
    ($($ty:ty),*) => { $(
        impl IntoJsRet for $ty {
            unsafe fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
                Ok(HermesValue {
                    kind: HermesValueKind_Number,
                    data: HermesValueData { number: self as f64 },
                })
            }
        }
    )* };
}

impl_into_js_ret_via_f64!(f32, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

impl<T: IntoJsRet> IntoJsRet for Result<T> {
    unsafe fn into_ret(self, rt: *mut HermesRt) -> Result<HermesValue> {
        self.and_then(|v| unsafe { v.into_ret(rt) })
    }
}
