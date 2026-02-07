use libhermesabi_sys::*;

use crate::error::{Error, Result};
use crate::value::Value;
use crate::Runtime;

/// Convert a Rust value into a JS [`Value`].
pub trait IntoJs<'rt> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>>;
}

/// Extract a Rust value from a JS [`Value`].
pub trait FromJs<'rt>: Sized {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self>;
}

// -- IntoJs impls -------------------------------------------------------------

impl<'rt> IntoJs<'rt> for Value<'rt> {
    fn into_js(self, _rt: &'rt Runtime) -> Result<Value<'rt>> {
        Ok(self)
    }
}

impl<'rt> IntoJs<'rt> for () {
    fn into_js(self, _rt: &'rt Runtime) -> Result<Value<'rt>> {
        Ok(Value::undefined())
    }
}

impl<'rt> IntoJs<'rt> for bool {
    fn into_js(self, _rt: &'rt Runtime) -> Result<Value<'rt>> {
        Ok(Value::from_bool(self))
    }
}

impl<'rt> IntoJs<'rt> for f64 {
    fn into_js(self, _rt: &'rt Runtime) -> Result<Value<'rt>> {
        Ok(Value::from_number(self))
    }
}

impl<'rt> IntoJs<'rt> for f32 {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        (self as f64).into_js(rt)
    }
}

impl<'rt> IntoJs<'rt> for i32 {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        (self as f64).into_js(rt)
    }
}

impl<'rt> IntoJs<'rt> for u32 {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        (self as f64).into_js(rt)
    }
}

impl<'rt> IntoJs<'rt> for i64 {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        (self as f64).into_js(rt)
    }
}

impl<'rt> IntoJs<'rt> for u64 {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        (self as f64).into_js(rt)
    }
}

impl<'rt> IntoJs<'rt> for String {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let js = crate::JsString::new(rt, &self);
        Ok(js.into())
    }
}

impl<'rt> IntoJs<'rt> for &str {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let js = crate::JsString::new(rt, self);
        Ok(js.into())
    }
}

impl<'rt, T: IntoJs<'rt>> IntoJs<'rt> for Option<T> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        match self {
            Some(v) => v.into_js(rt),
            None => Ok(Value::null()),
        }
    }
}

impl<'rt, T: IntoJs<'rt>> IntoJs<'rt> for Vec<T> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let arr = crate::Array::new(rt, self.len());
        for (i, v) in self.into_iter().enumerate() {
            arr.set(i, v.into_js(rt)?)?;
        }
        Ok(arr.into())
    }
}

// -- FromJs impls -------------------------------------------------------------

impl<'rt> FromJs<'rt> for Value<'rt> {
    fn from_js(_rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        Ok(value.duplicate())
    }
}

impl<'rt> FromJs<'rt> for bool {
    fn from_js(_rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        value.as_bool().ok_or(Error::TypeError {
            expected: "boolean",
            got: value.kind().name(),
        })
    }
}

impl<'rt> FromJs<'rt> for f64 {
    fn from_js(_rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        value.as_number().ok_or(Error::TypeError {
            expected: "number",
            got: value.kind().name(),
        })
    }
}

impl<'rt> FromJs<'rt> for f32 {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        f64::from_js(rt, value).map(|n| n as f32)
    }
}

impl<'rt> FromJs<'rt> for i32 {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        f64::from_js(rt, value).map(|n| n as i32)
    }
}

impl<'rt> FromJs<'rt> for u32 {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        f64::from_js(rt, value).map(|n| n as u32)
    }
}

impl<'rt> FromJs<'rt> for i64 {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        f64::from_js(rt, value).map(|n| n as i64)
    }
}

impl<'rt> FromJs<'rt> for u64 {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        f64::from_js(rt, value).map(|n| n as u64)
    }
}

impl<'rt> FromJs<'rt> for String {
    fn from_js(_rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        if value.raw.kind != HermesValueKind_String {
            return Err(Error::TypeError {
                expected: "string",
                got: value.kind().name(),
            });
        }
        let pv = unsafe { value.raw.data.pointer };
        let rt = value.rt;
        let needed =
            unsafe { hermes__String__ToUtf8(rt, pv, std::ptr::null_mut(), 0) };
        if needed == 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u8; needed];
        unsafe {
            hermes__String__ToUtf8(rt, pv, buf.as_mut_ptr() as *mut i8, buf.len());
        }
        String::from_utf8(buf).map_err(|e| Error::RuntimeError(e.to_string()))
    }
}

impl<'rt, T: FromJs<'rt>> FromJs<'rt> for Option<T> {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        if value.is_null() || value.is_undefined() {
            Ok(None)
        } else {
            T::from_js(rt, value).map(Some)
        }
    }
}
