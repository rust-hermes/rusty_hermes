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

macro_rules! impl_into_js_via_f64 {
    ($($ty:ty),*) => { $(
        impl<'rt> IntoJs<'rt> for $ty {
            fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
                (self as f64).into_js(rt)
            }
        }
    )* };
}

impl_into_js_via_f64!(f32, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

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

macro_rules! impl_from_js_via_f64 {
    ($($ty:ty),*) => { $(
        impl<'rt> FromJs<'rt> for $ty {
            fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
                f64::from_js(rt, value).map(|n| n as $ty)
            }
        }
    )* };
}

impl_from_js_via_f64!(f32, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

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

impl<'rt, T: FromJs<'rt>> FromJs<'rt> for Vec<T> {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        let arr = value.duplicate().into_array()?;
        let len = arr.len();
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            out.push(T::from_js(rt, &arr.get(i)?)?);
        }
        Ok(out)
    }
}

impl<'rt, T: IntoJs<'rt>> IntoJs<'rt> for std::collections::HashMap<String, T> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let obj = crate::Object::new(rt);
        for (key, val) in self {
            obj.set(&key, val.into_js(rt)?)?;
        }
        Ok(obj.into())
    }
}

impl<'rt, T: FromJs<'rt>> FromJs<'rt> for std::collections::HashMap<String, T> {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        let obj = value.duplicate().into_object()?;
        let names = obj.property_names()?;
        let len = names.len();
        let mut map = std::collections::HashMap::with_capacity(len);
        for i in 0..len {
            let key_val = names.get(i)?;
            let key = String::from_js(rt, &key_val)?;
            let val = obj.get(&key)?;
            map.insert(key, T::from_js(rt, &val)?);
        }
        Ok(map)
    }
}

impl<'rt, T: IntoJs<'rt>> IntoJs<'rt> for std::collections::BTreeMap<String, T> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let obj = crate::Object::new(rt);
        for (key, val) in self {
            obj.set(&key, val.into_js(rt)?)?;
        }
        Ok(obj.into())
    }
}

impl<'rt, T: FromJs<'rt>> FromJs<'rt> for std::collections::BTreeMap<String, T> {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        let obj = value.duplicate().into_object()?;
        let names = obj.property_names()?;
        let len = names.len();
        let mut map = std::collections::BTreeMap::new();
        for i in 0..len {
            let key_val = names.get(i)?;
            let key = String::from_js(rt, &key_val)?;
            let val = obj.get(&key)?;
            map.insert(key, T::from_js(rt, &val)?);
        }
        Ok(map)
    }
}

impl<'rt, T: IntoJs<'rt>> IntoJs<'rt> for std::collections::HashSet<T> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let arr = crate::Array::new(rt, self.len());
        for (i, v) in self.into_iter().enumerate() {
            arr.set(i, v.into_js(rt)?)?;
        }
        Ok(arr.into())
    }
}

impl<'rt, T: FromJs<'rt> + Eq + std::hash::Hash> FromJs<'rt> for std::collections::HashSet<T> {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        let arr = value.duplicate().into_array()?;
        let len = arr.len();
        let mut set = std::collections::HashSet::with_capacity(len);
        for i in 0..len {
            set.insert(T::from_js(rt, &arr.get(i)?)?);
        }
        Ok(set)
    }
}

impl<'rt, T: IntoJs<'rt>> IntoJs<'rt> for std::collections::BTreeSet<T> {
    fn into_js(self, rt: &'rt Runtime) -> Result<Value<'rt>> {
        let arr = crate::Array::new(rt, self.len());
        for (i, v) in self.into_iter().enumerate() {
            arr.set(i, v.into_js(rt)?)?;
        }
        Ok(arr.into())
    }
}

impl<'rt, T: FromJs<'rt> + Ord> FromJs<'rt> for std::collections::BTreeSet<T> {
    fn from_js(rt: &'rt Runtime, value: &Value<'rt>) -> Result<Self> {
        let arr = value.duplicate().into_array()?;
        let len = arr.len();
        let mut set = std::collections::BTreeSet::new();
        for i in 0..len {
            set.insert(T::from_js(rt, &arr.get(i)?)?);
        }
        Ok(set)
    }
}

