use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::{check_error, Error, Result};
use crate::value::Value;
use crate::Runtime;

/// A JavaScript function handle.
pub struct Function<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Function<'rt> {
    /// Call this function with `undefined` as `this`.
    pub fn call(&self, args: &[Value<'rt>]) -> Result<Value<'rt>> {
        let c_args: Vec<HermesValue> = args.iter().map(|a| raw_copy(&a.raw)).collect();
        let this = HermesValue {
            kind: HermesValueKind_Undefined,
            data: HermesValueData { number: 0.0 },
        };
        let raw = unsafe {
            hermes__Function__Call(
                self.rt,
                self.pv,
                &this,
                c_args.as_ptr(),
                c_args.len(),
            )
        };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    /// Call as a constructor (`new Func(...args)`).
    pub fn call_as_constructor(&self, args: &[Value<'rt>]) -> Result<Value<'rt>> {
        let c_args: Vec<HermesValue> = args.iter().map(|a| raw_copy(&a.raw)).collect();
        let raw = unsafe {
            hermes__Function__CallAsConstructor(
                self.rt,
                self.pv,
                c_args.as_ptr(),
                c_args.len(),
            )
        };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    pub fn is_host_function(&self) -> bool {
        unsafe { hermes__Function__IsHostFunction(self.rt, self.pv) }
    }
}

/// Make a shallow copy of a `HermesValue` for passing to FFI *without*
/// transferring ownership.  The C layer's `c_to_jsi_value` clones pointer
/// types, so the original remains valid.
fn raw_copy(v: &HermesValue) -> HermesValue {
    HermesValue {
        kind: v.kind,
        data: unsafe {
            // Copy the union bits.
            let mut d = HermesValueData { number: 0.0 };
            std::ptr::copy_nonoverlapping(
                v.data.number.to_ne_bytes().as_ptr(),
                (&raw mut d) as *mut u8,
                std::mem::size_of::<HermesValueData>(),
            );
            d
        },
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
        let val = Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: f.pv },
            },
            rt: f.rt,
            _marker: PhantomData,
        };
        std::mem::forget(f);
        val
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
// Host function support
// =============================================================================

/// Trait for Rust closures that can be turned into JS host functions.
///
/// Implemented automatically for `Fn` closures whose arguments implement
/// [`FromJsArg`] and whose return type implements [`IntoJsRet`].
pub trait IntoJsFunc<Args> {
    fn param_count(&self) -> u32;

    /// Box the closure and return a raw trampoline + user_data + finalizer
    /// suitable for `CreateFromHostFunction`.
    fn into_parts(
        self,
    ) -> (
        HermesHostFunctionCallback,
        *mut std::ffi::c_void,
        HermesHostFunctionFinalizer,
    );
}

// -- Lightweight arg/ret conversion used only inside the trampoline ----------

/// Extract a Rust value from a raw `HermesValue` arg during a host call.
pub trait FromJsArg: Sized {
    fn from_arg(rt: *mut HermesRt, raw: &HermesValue) -> Result<Self>;
}

impl FromJsArg for f64 {
    fn from_arg(_rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
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
    fn from_arg(_rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
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
    fn from_arg(rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
        if raw.kind != HermesValueKind_String {
            return Err(Error::TypeError {
                expected: "string",
                got: crate::value::ValueKind::from_raw(raw.kind).name(),
            });
        }
        let pv = unsafe { raw.data.pointer };
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

impl FromJsArg for i32 {
    fn from_arg(rt: *mut HermesRt, raw: &HermesValue) -> Result<Self> {
        f64::from_arg(rt, raw).map(|n| n as i32)
    }
}

/// Convert a Rust return value into a raw `HermesValue`.
pub trait IntoJsRet {
    fn into_ret(self, rt: *mut HermesRt) -> Result<HermesValue>;
}

impl IntoJsRet for () {
    fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Undefined,
            data: HermesValueData { number: 0.0 },
        })
    }
}

impl IntoJsRet for f64 {
    fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Number,
            data: HermesValueData { number: self },
        })
    }
}

impl IntoJsRet for bool {
    fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Boolean,
            data: HermesValueData { boolean: self },
        })
    }
}

impl IntoJsRet for String {
    fn into_ret(self, rt: *mut HermesRt) -> Result<HermesValue> {
        let pv = unsafe {
            hermes__String__CreateFromUtf8(rt, self.as_ptr(), self.len())
        };
        Ok(HermesValue {
            kind: HermesValueKind_String,
            data: HermesValueData { pointer: pv },
        })
    }
}

impl IntoJsRet for i32 {
    fn into_ret(self, _rt: *mut HermesRt) -> Result<HermesValue> {
        Ok(HermesValue {
            kind: HermesValueKind_Number,
            data: HermesValueData {
                number: self as f64,
            },
        })
    }
}

impl<T: IntoJsRet> IntoJsRet for Result<T> {
    fn into_ret(self, rt: *mut HermesRt) -> Result<HermesValue> {
        self.and_then(|v| v.into_ret(rt))
    }
}

// -- Macro to generate IntoJsFunc for N-ary tuples ---------------------------

macro_rules! impl_into_js_func {
    // Base case: 0 args
    (@impl () ($($idx:tt)*)) => {
        impl<F, R> IntoJsFunc<()> for F
        where
            F: Fn() -> R + 'static,
            R: IntoJsRet,
        {
            fn param_count(&self) -> u32 { 0 }

            fn into_parts(self) -> (HermesHostFunctionCallback, *mut std::ffi::c_void, HermesHostFunctionFinalizer) {
                let boxed: Box<Box<dyn Fn() -> R>> = Box::new(Box::new(self));
                let user_data = Box::into_raw(boxed) as *mut std::ffi::c_void;

                unsafe extern "C" fn trampoline<F2, R2>(
                    rt: *mut HermesRt,
                    _this: *const HermesValue,
                    _args: *const HermesValue,
                    _argc: usize,
                    user_data: *mut std::ffi::c_void,
                ) -> HermesValue
                where
                    F2: Fn() -> R2,
                    R2: IntoJsRet,
                {
                    let closure = &*(user_data as *const Box<dyn Fn() -> R2>);
                    match closure().into_ret(rt) {
                        Ok(v) => v,
                        Err(_) => HermesValue {
                            kind: HermesValueKind_Undefined,
                            data: HermesValueData { number: 0.0 },
                        },
                    }
                }

                unsafe extern "C" fn drop_fn<F2, R2>(user_data: *mut std::ffi::c_void)
                where
                    F2: Fn() -> R2,
                    R2: IntoJsRet,
                {
                    drop(Box::from_raw(user_data as *mut Box<dyn Fn() -> R2>));
                }

                (trampoline::<F, R>, user_data, drop_fn::<F, R>)
            }
        }
    };
    // Recursive case: N args
    (@impl ($($A:ident),+) ($($idx:tt)+)) => {
        #[allow(non_snake_case)]
        impl<F, $($A,)+ R> IntoJsFunc<($($A,)+)> for F
        where
            F: Fn($($A),+) -> R + 'static,
            $($A: FromJsArg + 'static,)+
            R: IntoJsRet + 'static,
        {
            fn param_count(&self) -> u32 {
                [$($idx,)+].len() as u32
            }

            fn into_parts(self) -> (HermesHostFunctionCallback, *mut std::ffi::c_void, HermesHostFunctionFinalizer) {
                // Type-erase via trait object.
                let boxed: Box<Box<dyn Fn($($A),+) -> R>> = Box::new(Box::new(self));
                let user_data = Box::into_raw(boxed) as *mut std::ffi::c_void;

                unsafe extern "C" fn trampoline<FF, $($A,)+ RR>(
                    rt: *mut HermesRt,
                    _this: *const HermesValue,
                    args: *const HermesValue,
                    _argc: usize,
                    user_data: *mut std::ffi::c_void,
                ) -> HermesValue
                where
                    FF: Fn($($A),+) -> RR,
                    $($A: FromJsArg,)+
                    RR: IntoJsRet,
                {
                    let closure = &*(user_data as *const Box<dyn Fn($($A),+) -> RR>);
                    let _args_slice = std::slice::from_raw_parts(args, _argc);
                    // Extract each argument.
                    $(
                        let $A = match $A::from_arg(rt, _args_slice.get($idx).unwrap_or(&HermesValue {
                            kind: HermesValueKind_Undefined,
                            data: HermesValueData { number: 0.0 },
                        })) {
                            Ok(v) => v,
                            Err(_) => return HermesValue {
                                kind: HermesValueKind_Undefined,
                                data: HermesValueData { number: 0.0 },
                            },
                        };
                    )+
                    match closure($($A),+).into_ret(rt) {
                        Ok(v) => v,
                        Err(_) => HermesValue {
                            kind: HermesValueKind_Undefined,
                            data: HermesValueData { number: 0.0 },
                        },
                    }
                }

                unsafe extern "C" fn drop_fn<FF, $($A,)+ RR>(user_data: *mut std::ffi::c_void)
                where
                    FF: Fn($($A),+) -> RR,
                    $($A: FromJsArg,)+
                    RR: IntoJsRet,
                {
                    drop(Box::from_raw(user_data as *mut Box<dyn Fn($($A),+) -> RR>));
                }

                (trampoline::<F, $($A,)+ R>, user_data, drop_fn::<F, $($A,)+ R>)
            }
        }
    };
    // Entry: expand for 0..N args
    () => {
        impl_into_js_func!(@impl () ());
    };
    ($A:ident $idx:tt) => {
        impl_into_js_func!(@impl ($A) ($idx));
    };
    ($A:ident $aidx:tt, $($B:ident $bidx:tt),+) => {
        impl_into_js_func!(@impl ($A, $($B),+) ($aidx $($bidx)+));
    };
}

// Generate implementations for 0..8 arguments.
impl_into_js_func!();
impl_into_js_func!(A 0);
impl_into_js_func!(A 0, B 1);
impl_into_js_func!(A 0, B 1, C 2);
impl_into_js_func!(A 0, B 1, C 2, D 3);
impl_into_js_func!(A 0, B 1, C 2, D 3, E 4);
impl_into_js_func!(A 0, B 1, C 2, D 3, E 4, Fa 5);
impl_into_js_func!(A 0, B 1, C 2, D 3, E 4, Fa 5, G 6);
impl_into_js_func!(A 0, B 1, C 2, D 3, E 4, Fa 5, G 6, H 7);

/// Create a host function from a Rust closure and register it on the runtime.
///
/// This is the internal plumbing used by [`Runtime::set_func`].
pub(crate) fn create_host_function<'rt, Args, F: IntoJsFunc<Args>>(
    rt: &'rt Runtime,
    name: &str,
    f: F,
) -> Result<Function<'rt>> {
    let param_count = f.param_count();
    let (callback, user_data, finalizer) = f.into_parts();

    let name_pv = unsafe {
        hermes__PropNameID__ForUtf8(rt.raw, name.as_ptr(), name.len())
    };
    let func_pv = unsafe {
        hermes__Function__CreateFromHostFunction(
            rt.raw,
            name_pv,
            param_count,
            callback,
            user_data,
            finalizer,
        )
    };
    unsafe { hermes__PropNameID__Release(name_pv) };
    check_error(rt.raw)?;

    Ok(Function {
        pv: func_pv,
        rt: rt.raw,
        _marker: PhantomData,
    })
}
