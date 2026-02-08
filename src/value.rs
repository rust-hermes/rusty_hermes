use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::{Error, Result};
use crate::{Array, ArrayBuffer, BigInt, Function, JsString, Object, Symbol};

/// Kind tag for a [`Value`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Undefined,
    Null,
    Boolean,
    Number,
    Symbol,
    BigInt,
    String,
    Object,
}

impl ValueKind {
    pub(crate) fn from_raw(kind: i32) -> Self {
        match kind {
            HermesValueKind_Null => ValueKind::Null,
            HermesValueKind_Boolean => ValueKind::Boolean,
            HermesValueKind_Number => ValueKind::Number,
            HermesValueKind_Symbol => ValueKind::Symbol,
            HermesValueKind_BigInt => ValueKind::BigInt,
            HermesValueKind_String => ValueKind::String,
            HermesValueKind_Object => ValueKind::Object,
            _ => ValueKind::Undefined,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ValueKind::Undefined => "undefined",
            ValueKind::Null => "null",
            ValueKind::Boolean => "boolean",
            ValueKind::Number => "number",
            ValueKind::Symbol => "symbol",
            ValueKind::BigInt => "bigint",
            ValueKind::String => "string",
            ValueKind::Object => "object",
        }
    }
}

/// A JavaScript value handle tied to the lifetime of a [`Runtime`](crate::Runtime).
///
/// Owns the underlying `HermesValue`. Pointer-typed values (string, object,
/// symbol, bigint) are released on drop.
pub struct Value<'rt> {
    pub(crate) raw: HermesValue,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Value<'rt> {
    // -- constructors for primitives (no runtime needed) -----------------------

    /// Create an `undefined` value.
    pub fn undefined() -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Undefined,
                data: HermesValueData { number: 0.0 },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    /// Create a `null` value.
    pub fn null() -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Null,
                data: HermesValueData { number: 0.0 },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    /// Create a boolean value.
    pub fn from_bool(v: bool) -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Boolean,
                data: HermesValueData { boolean: v },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    /// Create a number value.
    pub fn from_number(v: f64) -> Self {
        Value {
            raw: HermesValue {
                kind: HermesValueKind_Number,
                data: HermesValueData { number: v },
            },
            rt: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    // -- from raw FFI result ---------------------------------------------------

    /// Wrap a raw `HermesValue` returned from the FFI layer.
    ///
    /// # Safety
    /// The caller must ensure `rt` is valid and `raw` was produced by the same
    /// runtime.
    pub(crate) unsafe fn from_raw(rt: *mut HermesRt, raw: HermesValue) -> Self {
        Value {
            raw,
            rt,
            _marker: PhantomData,
        }
    }

    /// Clone a raw `HermesValue` and wrap it as an owned `Value`.
    ///
    /// For pointer types this calls `hermes__Value__Clone`; for primitives
    /// the raw bits are simply copied.
    ///
    /// # Safety
    /// `rt` must be a valid runtime pointer. `raw` must belong to that runtime.
    pub unsafe fn from_raw_clone(rt: *mut HermesRt, raw: &HermesValue) -> Self { unsafe {
        match raw.kind {
            HermesValueKind_String | HermesValueKind_Object | HermesValueKind_Symbol
            | HermesValueKind_BigInt => {
                let cloned = hermes__Value__Clone(rt, raw);
                Value {
                    raw: cloned,
                    rt,
                    _marker: PhantomData,
                }
            }
            _ => Value {
                raw: std::ptr::read(raw),
                rt,
                _marker: PhantomData,
            },
        }
    }}

    // -- kind checks -----------------------------------------------------------

    pub fn kind(&self) -> ValueKind {
        ValueKind::from_raw(self.raw.kind)
    }

    pub fn is_undefined(&self) -> bool {
        self.raw.kind == HermesValueKind_Undefined
    }
    pub fn is_null(&self) -> bool {
        self.raw.kind == HermesValueKind_Null
    }
    pub fn is_boolean(&self) -> bool {
        self.raw.kind == HermesValueKind_Boolean
    }
    pub fn is_number(&self) -> bool {
        self.raw.kind == HermesValueKind_Number
    }
    pub fn is_symbol(&self) -> bool {
        self.raw.kind == HermesValueKind_Symbol
    }
    pub fn is_bigint(&self) -> bool {
        self.raw.kind == HermesValueKind_BigInt
    }
    pub fn is_string(&self) -> bool {
        self.raw.kind == HermesValueKind_String
    }
    pub fn is_object(&self) -> bool {
        self.raw.kind == HermesValueKind_Object
    }

    // -- primitive extraction --------------------------------------------------

    pub fn as_bool(&self) -> Option<bool> {
        if self.is_boolean() {
            Some(unsafe { self.raw.data.boolean })
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if self.is_number() {
            Some(unsafe { self.raw.data.number })
        } else {
            None
        }
    }

    // -- consuming conversions to typed wrappers --------------------------------

    /// Convert to [`JsString`], consuming `self`. Returns `Err` if the value is
    /// not a string.
    pub fn into_string(self) -> Result<JsString<'rt>> {
        if !self.is_string() {
            return Err(Error::TypeError {
                expected: "string",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let rt = self.rt;
        std::mem::forget(self); // prevent double-release
        Ok(JsString {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    /// Convert to [`Object`], consuming `self`.
    pub fn into_object(self) -> Result<Object<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "object",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let rt = self.rt;
        std::mem::forget(self);
        Ok(Object {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    /// Convert to [`Function`], consuming `self`.
    /// Checks that the value is an object **and** is callable.
    pub fn into_function(self) -> Result<Function<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "function",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let is_fn = unsafe { hermes__Object__IsFunction(self.rt, ptr) };
        if !is_fn {
            return Err(Error::TypeError {
                expected: "function",
                got: "object",
            });
        }
        let rt = self.rt;
        std::mem::forget(self);
        Ok(Function {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    /// Convert to [`Array`], consuming `self`.
    pub fn into_array(self) -> Result<Array<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "array",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let is_arr = unsafe { hermes__Object__IsArray(self.rt, ptr) };
        if !is_arr {
            return Err(Error::TypeError {
                expected: "array",
                got: "object",
            });
        }
        let rt = self.rt;
        std::mem::forget(self);
        Ok(Array {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    /// Convert to [`Symbol`], consuming `self`.
    pub fn into_symbol(self) -> Result<Symbol<'rt>> {
        if !self.is_symbol() {
            return Err(Error::TypeError {
                expected: "symbol",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let rt = self.rt;
        std::mem::forget(self);
        Ok(Symbol {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    /// Convert to [`BigInt`], consuming `self`.
    pub fn into_bigint(self) -> Result<BigInt<'rt>> {
        if !self.is_bigint() {
            return Err(Error::TypeError {
                expected: "bigint",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let rt = self.rt;
        std::mem::forget(self);
        Ok(BigInt {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    /// Convert to [`ArrayBuffer`], consuming `self`.
    pub fn into_array_buffer(self) -> Result<ArrayBuffer<'rt>> {
        if !self.is_object() {
            return Err(Error::TypeError {
                expected: "arraybuffer",
                got: self.kind().name(),
            });
        }
        let ptr = unsafe { self.raw.data.pointer };
        let is_ab = unsafe { hermes__Object__IsArrayBuffer(self.rt, ptr) };
        if !is_ab {
            return Err(Error::TypeError {
                expected: "arraybuffer",
                got: "object",
            });
        }
        let rt = self.rt;
        std::mem::forget(self);
        Ok(ArrayBuffer {
            pv: ptr,
            rt,
            _marker: PhantomData,
        })
    }

    // -- conversion to string --------------------------------------------------

    /// Convert any value to a JS string (JS `String(value)` semantics).
    pub fn to_js_string(&self) -> Result<JsString<'rt>> {
        let pv = unsafe { hermes__Value__ToString(self.rt, &self.raw) };
        crate::error::check_error(self.rt)?;
        Ok(JsString {
            pv,
            rt: self.rt,
            _marker: PhantomData,
        })
    }

    /// Deep-clone this value. Creates a new `PointerValue` for pointer types.
    /// Primitive types (undefined, null, boolean, number) are copied inline.
    pub fn duplicate(&self) -> Value<'rt> {
        match self.raw.kind {
            HermesValueKind_String | HermesValueKind_Object | HermesValueKind_Symbol
            | HermesValueKind_BigInt => {
                let raw = unsafe { hermes__Value__Clone(self.rt, &self.raw) };
                Value {
                    raw,
                    rt: self.rt,
                    _marker: PhantomData,
                }
            }
            // Primitives have no pointer — just copy the raw bits.
            _ => Value {
                raw: unsafe { std::ptr::read(&self.raw) },
                rt: self.rt,
                _marker: PhantomData,
            },
        }
    }

    /// Consume this `Value` and return the underlying `HermesValue` without
    /// running the destructor. The caller takes ownership of any pointer value.
    pub fn into_raw(self) -> HermesValue {
        let raw = unsafe { std::ptr::read(&self.raw) };
        std::mem::forget(self);
        raw
    }

    // -- comparison ------------------------------------------------------------

    pub fn strict_equals(&self, other: &Value<'rt>) -> bool {
        unsafe { hermes__Value__StrictEquals(self.rt, &self.raw, &other.raw) }
    }
}

impl Drop for Value<'_> {
    fn drop(&mut self) {
        // Only pointer kinds need releasing.
        match self.raw.kind {
            HermesValueKind_String | HermesValueKind_Object | HermesValueKind_Symbol
            | HermesValueKind_BigInt => unsafe {
                hermes__Value__Release(&mut self.raw);
            },
            _ => {}
        }
    }
}

impl std::fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind() {
            ValueKind::Undefined => write!(f, "Value(undefined)"),
            ValueKind::Null => write!(f, "Value(null)"),
            ValueKind::Boolean => write!(f, "Value({})", self.as_bool().unwrap()),
            ValueKind::Number => write!(f, "Value({})", self.as_number().unwrap()),
            ValueKind::String => write!(f, "Value(string)"),
            ValueKind::Object => write!(f, "Value(object)"),
            ValueKind::Symbol => write!(f, "Value(symbol)"),
            ValueKind::BigInt => write!(f, "Value(bigint)"),
        }
    }
}
