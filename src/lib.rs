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
mod array_buffer;
mod bigint;
mod convert;
mod error;
mod function;
mod object;
mod prepared_js;
mod propnameid;
mod scope;
mod string;
mod symbol;
mod value;
mod weak_object;

pub use array::Array;
pub use array_buffer::ArrayBuffer;
pub use bigint::BigInt;
pub use convert::{FromJs, IntoJs};
pub use error::{Error, Result};
pub use function::{Function, IntoJsFunc};
pub use object::Object;
pub use prepared_js::PreparedJavaScript;
pub use propnameid::PropNameId;
pub use scope::Scope;
pub use string::JsString;
pub use symbol::Symbol;
pub use value::{Value, ValueKind};
pub use weak_object::WeakObject;
// Re-exported so users don't need libhermesabi_sys directly.
pub use libhermesabi_sys::HermesRuntimeConfig;
pub use libhermesabi_sys::HermesNativeStateFinalizer;
pub use libhermesabi_sys::{
    HermesHostObjectFinalizer, HermesHostObjectGetCallback,
    HermesHostObjectGetPropertyNamesCallback, HermesHostObjectSetCallback,
};

use std::marker::PhantomData;

use libhermesabi_sys::*;

/// Configuration options for creating a Hermes runtime.
///
/// Use the builder pattern to customize the runtime:
///
/// ```rust,no_run
/// use rusty_hermes::{Runtime, RuntimeConfig};
///
/// let config = RuntimeConfig::builder()
///     .enable_eval(false)
///     .microtask_queue(true)
///     .intl(false)
///     .build();
/// let rt = Runtime::with_config(config).unwrap();
/// ```
pub struct RuntimeConfig {
    raw: HermesRuntimeConfig,
}

impl RuntimeConfig {
    /// Create a builder with Hermes defaults.
    pub fn builder() -> RuntimeConfigBuilder {
        RuntimeConfigBuilder {
            raw: HermesRuntimeConfig {
                enable_eval: true,
                es6_promise: true,
                es6_proxy: true,
                es6_class: false,
                intl: true,
                microtask_queue: false,
                enable_generator: true,
                enable_block_scoping: false,
                enable_hermes_internal: true,
                enable_hermes_internal_test_methods: false,
                max_num_registers: 128 * 1024,
            },
        }
    }
}

/// Builder for [`RuntimeConfig`].
pub struct RuntimeConfigBuilder {
    raw: HermesRuntimeConfig,
}

impl RuntimeConfigBuilder {
    /// Allow `eval()` and the `Function()` constructor. Default: `true`.
    pub fn enable_eval(mut self, v: bool) -> Self {
        self.raw.enable_eval = v;
        self
    }

    /// Enable ES6 Promise support. Default: `true`.
    pub fn es6_promise(mut self, v: bool) -> Self {
        self.raw.es6_promise = v;
        self
    }

    /// Enable ES6 Proxy support. Default: `true`.
    pub fn es6_proxy(mut self, v: bool) -> Self {
        self.raw.es6_proxy = v;
        self
    }

    /// Enable ES6 class support. Default: `false`.
    pub fn es6_class(mut self, v: bool) -> Self {
        self.raw.es6_class = v;
        self
    }

    /// Enable ECMA-402 Intl APIs. Default: `true`.
    pub fn intl(mut self, v: bool) -> Self {
        self.raw.intl = v;
        self
    }

    /// Enable the microtask queue. Default: `false`.
    pub fn microtask_queue(mut self, v: bool) -> Self {
        self.raw.microtask_queue = v;
        self
    }

    /// Enable generator support. Default: `true`.
    pub fn enable_generator(mut self, v: bool) -> Self {
        self.raw.enable_generator = v;
        self
    }

    /// Enable block scoping (`let`/`const`). Default: `false`.
    pub fn enable_block_scoping(mut self, v: bool) -> Self {
        self.raw.enable_block_scoping = v;
        self
    }

    /// Enable the `HermesInternal` object. Default: `true`.
    pub fn enable_hermes_internal(mut self, v: bool) -> Self {
        self.raw.enable_hermes_internal = v;
        self
    }

    /// Enable HermesInternal test methods. Default: `false`.
    pub fn enable_hermes_internal_test_methods(mut self, v: bool) -> Self {
        self.raw.enable_hermes_internal_test_methods = v;
        self
    }

    /// Maximum number of registers. Default: `131072` (128K).
    pub fn max_num_registers(mut self, v: u32) -> Self {
        self.raw.max_num_registers = v;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> RuntimeConfig {
        RuntimeConfig { raw: self.raw }
    }
}

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
    /// Create a new Hermes runtime with default configuration.
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

    /// Create a new Hermes runtime with custom configuration.
    pub fn with_config(config: RuntimeConfig) -> Result<Self> {
        let raw = unsafe { hermes__Runtime__NewWithConfig(&config.raw) };
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

    /// Parse a JSON string into a JS value.
    pub fn create_value_from_json(&self, json: &str) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__CreateValueFromJsonUtf8(
                self.raw,
                json.as_ptr(),
                json.len(),
            )
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    /// Evaluate JavaScript with an associated source map.
    pub fn eval_with_source_map(
        &self,
        code: &str,
        source_map: &[u8],
        url: &str,
    ) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__EvaluateJavaScriptWithSourceMap(
                self.raw,
                code.as_ptr(),
                code.len(),
                source_map.as_ptr(),
                source_map.len(),
                url.as_ptr() as *const i8,
                url.len(),
            )
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    /// Pre-compile JavaScript for later evaluation.
    pub fn prepare_javascript(
        &self,
        code: &str,
        url: &str,
    ) -> Result<PreparedJavaScript> {
        let raw = unsafe {
            hermes__Runtime__PrepareJavaScript(
                self.raw,
                code.as_ptr(),
                code.len(),
                url.as_ptr() as *const i8,
                url.len(),
            )
        };
        error::check_error(self.raw)?;
        if raw.is_null() {
            return Err(Error::RuntimeError(
                "failed to prepare JavaScript".into(),
            ));
        }
        Ok(PreparedJavaScript { raw })
    }

    /// Evaluate a previously prepared script.
    pub fn evaluate_prepared_javascript(
        &self,
        prepared: &PreparedJavaScript,
    ) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__EvaluatePreparedJavaScript(self.raw, prepared.raw)
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    /// Get a description of this runtime (e.g. "HermesRuntime").
    pub fn description(&self) -> String {
        let mut buf = vec![0u8; 256];
        let len = unsafe {
            hermes__Runtime__Description(
                self.raw,
                buf.as_mut_ptr() as *mut i8,
                buf.len(),
            )
        };
        buf.truncate(len);
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Check if this runtime supports debugging/inspection.
    pub fn is_inspectable(&self) -> bool {
        unsafe { hermes__Runtime__IsInspectable(self.raw) }
    }

    /// Set an execution time limit. After `timeout_ms` milliseconds,
    /// the runtime will throw a timeout error.
    pub fn watch_time_limit(&self, timeout_ms: u32) {
        unsafe { hermes__Runtime__WatchTimeLimit(self.raw, timeout_ms) }
    }

    /// Remove the execution time limit.
    pub fn unwatch_time_limit(&self) {
        unsafe { hermes__Runtime__UnwatchTimeLimit(self.raw) }
    }

    /// Trigger a timeout asynchronously (from another thread).
    pub fn async_trigger_timeout(&self) {
        unsafe { hermes__Runtime__AsyncTriggerTimeout(self.raw) }
    }

    /// Check if bytecode is valid Hermes bytecode.
    pub fn is_hermes_bytecode(data: &[u8]) -> bool {
        unsafe { hermes__IsHermesBytecode(data.as_ptr(), data.len()) }
    }

    /// Get the Hermes bytecode version.
    pub fn bytecode_version() -> u32 {
        unsafe { hermes__GetBytecodeVersion() }
    }

    /// Perform a sanity check on Hermes bytecode.
    pub fn bytecode_sanity_check(data: &[u8]) -> bool {
        unsafe { hermes__HermesBytecodeSanityCheck(data.as_ptr(), data.len()) }
    }

    /// Prefetch Hermes bytecode into the page cache.
    pub fn prefetch_bytecode(data: &[u8]) {
        unsafe { hermes__PrefetchHermesBytecode(data.as_ptr(), data.len()) }
    }

    /// Enable the sampling profiler globally.
    pub fn enable_sampling_profiler() {
        unsafe { hermes__EnableSamplingProfiler() }
    }

    /// Disable the sampling profiler globally.
    pub fn disable_sampling_profiler() {
        unsafe { hermes__DisableSamplingProfiler() }
    }

    /// Dump the sampled profiler trace to a file.
    pub fn dump_sampled_trace_to_file(filename: &str) {
        let c_str = std::ffi::CString::new(filename).expect("invalid filename");
        unsafe { hermes__DumpSampledTraceToFile(c_str.as_ptr()) }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { hermes__Runtime__Delete(self.raw) }
    }
}
