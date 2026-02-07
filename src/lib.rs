#![allow(non_upper_case_globals)]

//! High-level, safe Rust bindings for the Hermes JavaScript engine.
//!
//! Built on top of [`libhermesabi_sys`] (flat C FFI), this crate provides
//! ergonomic Rust types with lifetime-based safety: all JS values carry a
//! `'rt` lifetime tied to their [`Runtime`], preventing use-after-free at
//! compile time.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use rusty_hermes::Runtime;
//!
//! let rt = Runtime::new().unwrap();
//!
//! // Evaluate JavaScript
//! let val = rt.eval("1 + 2").unwrap();
//! assert_eq!(val.as_number(), Some(3.0));
//!
//! // Register a host function
//! rt.set_func("add", |a: f64, b: f64| -> f64 { a + b }).unwrap();
//! let result = rt.eval("add(10, 20)").unwrap();
//! assert_eq!(result.as_number(), Some(30.0));
//! ```

mod array;
mod bigint;
mod convert;
mod error;
mod function;
mod object;
mod string;
mod symbol;
mod value;

pub use array::Array;
pub use bigint::BigInt;
pub use convert::{FromJs, IntoJs};
pub use error::{Error, Result};
pub use function::{Function, IntoJsFunc};
pub use object::Object;
pub use string::JsString;
pub use symbol::Symbol;
pub use value::{Value, ValueKind};

use std::marker::PhantomData;

use libhermesabi_sys::*;

/// The Hermes JavaScript runtime.
///
/// Owns the underlying engine instance. All JS values produced by this runtime
/// borrow it via the `'rt` lifetime, ensuring they cannot outlive it.
///
/// **Not `Send` or `Sync`** — Hermes is single-threaded.
pub struct Runtime {
    pub(crate) raw: *mut HermesRt,
    _not_send_sync: PhantomData<*mut ()>,
}

impl Runtime {
    /// Create a new Hermes runtime.
    pub fn new() -> Result<Self> {
        let raw = unsafe { hermes__Runtime__New() };
        if raw.is_null() {
            return Err(Error::RuntimeError("failed to create Hermes runtime".into()));
        }
        Ok(Runtime {
            raw,
            _not_send_sync: PhantomData,
        })
    }

    /// Evaluate a JavaScript string. Source URL defaults to `"<eval>"`.
    pub fn eval(&self, code: &str) -> Result<Value<'_>> {
        self.eval_with_url(code, "<eval>")
    }

    /// Evaluate a JavaScript string with a custom source URL (for stack traces).
    pub fn eval_with_url(&self, code: &str, url: &str) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__EvaluateJavaScript(
                self.raw,
                code.as_ptr(),
                code.len(),
                url.as_ptr() as *const i8,
                url.len(),
            )
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    /// Get the global object.
    pub fn global(&self) -> Object<'_> {
        let pv = unsafe { hermes__Runtime__Global(self.raw) };
        Object {
            pv,
            rt: self.raw,
            _marker: PhantomData,
        }
    }

    /// Register a host function as a global property.
    ///
    /// ```rust,no_run
    /// # let rt = rusty_hermes::Runtime::new().unwrap();
    /// rt.set_func("greet", |name: String| -> String {
    ///     format!("Hello, {name}!")
    /// }).unwrap();
    /// ```
    pub fn set_func<Args, F: IntoJsFunc<Args>>(&self, name: &str, f: F) -> Result<()> {
        let func = function::create_host_function(self, name, f)?;
        let global = self.global();
        global.set(name, func.into())
    }

    /// Drain the microtask queue. Returns `true` if fully drained.
    pub fn drain_microtasks(&self) -> Result<bool> {
        let rc = unsafe { hermes__Runtime__DrainMicrotasks(self.raw, -1) };
        if rc < 0 {
            error::check_error(self.raw)?;
        }
        Ok(rc == 1)
    }

    /// Check if bytecode is valid Hermes bytecode.
    pub fn is_hermes_bytecode(data: &[u8]) -> bool {
        unsafe { hermes__IsHermesBytecode(data.as_ptr(), data.len()) }
    }

    /// Get the Hermes bytecode version.
    pub fn bytecode_version() -> u32 {
        unsafe { hermes__GetBytecodeVersion() }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { hermes__Runtime__Delete(self.raw) }
    }
}
