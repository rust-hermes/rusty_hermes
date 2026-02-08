use std::fmt;

use libhermesabi_sys::*;

unsafe extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

/// Check the runtime for a pending error and return it as `Err` if present.
///
/// The C layer stores JS exceptions in `pending_js_error` and native C++
/// exceptions in `pending_error_message`.  We drain both to fully reset
/// the error state.
pub(crate) fn check_error(rt: *mut HermesRt) -> Result<()> {
    unsafe {
        if !hermes__Runtime__HasPendingError(rt) {
            return Ok(());
        }

        // 1. Drain the native error message (strdup'd, caller frees).
        let c_msg = hermes__Runtime__GetAndClearErrorMessage(rt);

        // 2. Drain the JS error value and try to extract a message.
        let mut err_val = hermes__Runtime__GetAndClearError(rt);
        let js_msg = extract_error_message(rt, &err_val);
        // Release pointer-typed error values.
        if matches!(
            err_val.kind,
            HermesValueKind_String
                | HermesValueKind_Object
                | HermesValueKind_Symbol
                | HermesValueKind_BigInt
        ) {
            hermes__Value__Release(&mut err_val);
        }

        // Prefer the native message (more descriptive for Error objects).
        if !c_msg.is_null() {
            let s = std::ffi::CStr::from_ptr(c_msg)
                .to_string_lossy()
                .into_owned();
            free(c_msg as *mut _);
            return Err(Error::JsException(s));
        }
        if !js_msg.is_empty() {
            return Err(Error::JsException(js_msg));
        }
        Err(Error::JsException("unknown error".into()))
    }
}

/// Try to get a human-readable message from a JS error value.
///
/// Handles: string values (direct), Error objects (.message property),
/// and falls back to empty string for other types.
unsafe fn extract_error_message(rt: *mut HermesRt, val: &HermesValue) -> String { unsafe {
    fn read_string_pv(rt: *mut HermesRt, pv: *const std::ffi::c_void) -> String {
        unsafe {
            let needed = hermes__String__ToUtf8(rt, pv, std::ptr::null_mut(), 0);
            if needed == 0 {
                return String::new();
            }
            let mut buf = vec![0u8; needed];
            hermes__String__ToUtf8(rt, pv, buf.as_mut_ptr() as *mut i8, buf.len());
            String::from_utf8_lossy(&buf).into_owned()
        }
    }

    match val.kind {
        HermesValueKind_String => {
            let pv = val.data.pointer;
            read_string_pv(rt, pv)
        }
        HermesValueKind_Object => {
            // Try to read the .message property from Error objects.
            let key = b"message";
            let key_pv =
                hermes__String__CreateFromUtf8(rt, key.as_ptr(), key.len());
            let msg_val =
                hermes__Object__GetProperty__String(rt, val.data.pointer, key_pv);
            hermes__String__Release(key_pv);
            if msg_val.kind == HermesValueKind_String {
                let s = read_string_pv(rt, msg_val.data.pointer);
                let mut mv = msg_val;
                hermes__Value__Release(&mut mv);
                s
            } else {
                // Release if it was a pointer type, then return empty.
                if matches!(
                    msg_val.kind,
                    HermesValueKind_String
                        | HermesValueKind_Object
                        | HermesValueKind_Symbol
                        | HermesValueKind_BigInt
                ) {
                    let mut mv = msg_val;
                    hermes__Value__Release(&mut mv);
                }
                String::new()
            }
        }
        _ => String::new(),
    }
}}

/// Error type for Hermes operations.
#[derive(Debug, Clone)]
pub enum Error {
    /// A JavaScript exception was thrown.
    JsException(String),
    /// Type conversion error (e.g. tried to get a number from a string value).
    TypeError {
        expected: &'static str,
        got: &'static str,
    },
    /// Runtime-level error (e.g. failed to create runtime).
    RuntimeError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::JsException(msg) => write!(f, "JavaScript exception: {msg}"),
            Error::TypeError { expected, got } => {
                write!(f, "type error: expected {expected}, got {got}")
            }
            Error::RuntimeError(msg) => write!(f, "runtime error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;
