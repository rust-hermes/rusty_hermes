//! Rust port of https://github.com/tmikov/hermes-jsi-demos/tree/master/host-functions
//!
//! Registers two host functions (`add` and `myPrint`) into the Hermes runtime,
//! then evaluates a JS script that calls them.

#![allow(non_upper_case_globals)]

use libhermesabi_sys::*;
use std::ffi::c_void;
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

/// Host function: adds all its numeric arguments together.
///
/// Throws (via returning undefined + pending error) if any argument is not a number.
unsafe extern "C" fn host_add(
    rt: *mut HermesRt,
    _this_val: *const HermesValue,
    args: *const HermesValue,
    arg_count: usize,
    _user_data: *mut c_void,
) -> HermesValue {
    let mut sum = 0.0_f64;
    for i in 0..arg_count {
        let arg = &*args.add(i);
        if arg.kind != HermesValueKind_Number {
            // Evaluate a throw expression to set the pending error.
            let msg = b"add(): all arguments must be numbers";
            let url = b"native";
            let script = format!(
                "throw new TypeError({:?});",
                std::str::from_utf8_unchecked(msg)
            );
            hermes__Runtime__EvaluateJavaScript(
                rt,
                script.as_ptr(),
                script.len(),
                url.as_ptr() as *const i8,
                url.len(),
            );
            return HermesValue {
                kind: HermesValueKind_Undefined,
                data: HermesValueData { number: 0.0 },
            };
        }
        sum += arg.data.number;
    }
    HermesValue {
        kind: HermesValueKind_Number,
        data: HermesValueData { number: sum },
    }
}

/// Convert a HermesValue to a Rust String for display purposes.
unsafe fn value_to_string(rt: *mut HermesRt, val: &HermesValue) -> String {
    match val.kind {
        HermesValueKind_Undefined => "undefined".to_string(),
        HermesValueKind_Null => "null".to_string(),
        HermesValueKind_Boolean => {
            if val.data.boolean {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        HermesValueKind_Number => {
            let n = val.data.number;
            if n == (n as i64) as f64 && n.abs() < 1e15 {
                format!("{}", n as i64)
            } else {
                format!("{}", n)
            }
        }
        HermesValueKind_String => {
            let pv = val.data.pointer;
            let mut buf = [0i8; 1024];
            let len =
                hermes__String__ToUtf8(rt, pv, buf.as_mut_ptr(), buf.len());
            let bytes: Vec<u8> =
                buf[..len].iter().map(|&b| b as u8).collect();
            String::from_utf8_lossy(&bytes).into_owned()
        }
        HermesValueKind_Object => "[object]".to_string(),
        HermesValueKind_Symbol => "[symbol]".to_string(),
        HermesValueKind_BigInt => "[bigint]".to_string(),
        _ => "[unknown]".to_string(),
    }
}

/// Host function: prints all arguments separated by spaces to stdout.
unsafe extern "C" fn host_my_print(
    rt: *mut HermesRt,
    _this_val: *const HermesValue,
    args: *const HermesValue,
    arg_count: usize,
    _user_data: *mut c_void,
) -> HermesValue {
    let mut parts = Vec::with_capacity(arg_count);
    for i in 0..arg_count {
        let arg = &*args.add(i);
        parts.push(value_to_string(rt, arg));
    }
    println!("{}", parts.join(" "));
    HermesValue {
        kind: HermesValueKind_Undefined,
        data: HermesValueData { number: 0.0 },
    }
}

unsafe extern "C" fn noop_finalizer(_user_data: *mut c_void) {}

/// Register a host function on the global object.
unsafe fn register_host_function(
    rt: *mut HermesRt,
    global: *mut c_void,
    name: &str,
    param_count: u32,
    callback: HermesHostFunctionCallback,
) {
    let name_pni = hermes__PropNameID__ForAscii(
        rt,
        name.as_ptr() as *const i8,
        name.len(),
    );
    let func = hermes__Function__CreateFromHostFunction(
        rt,
        name_pni,
        param_count,
        callback,
        ptr::null_mut(),
        noop_finalizer,
    );
    let func_val = HermesValue {
        kind: HermesValueKind_Object,
        data: HermesValueData { pointer: func },
    };
    hermes__Object__SetProperty__PropNameID(rt, global, name_pni, &func_val);

    hermes__PropNameID__Release(name_pni);
    hermes__Function__Release(func);
}

fn main() {
    // JS source — either from a command-line file or the inline default.
    let js_source = if let Some(path) = std::env::args().nth(1) {
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
    } else {
        r#"myPrint("Host function add() returned", add(10, 20, 30));"#
            .to_string()
    };

    unsafe {
        let rt = hermes__Runtime__New();
        let global = hermes__Runtime__Global(rt);

        // Register host functions.
        register_host_function(rt, global, "add", 2, host_add);
        register_host_function(rt, global, "myPrint", 0, host_my_print);

        // Evaluate the script.
        let url = b"demo.js";
        let result = hermes__Runtime__EvaluateJavaScript(
            rt,
            js_source.as_ptr(),
            js_source.len(),
            url.as_ptr() as *const i8,
            url.len(),
        );

        // Check for errors.
        if hermes__Runtime__HasPendingError(rt) {
            let msg = hermes__Runtime__GetAndClearErrorMessage(rt);
            if !msg.is_null() {
                let cstr = std::ffi::CStr::from_ptr(msg);
                eprintln!("JS Error: {}", cstr.to_string_lossy());
                free(msg as *mut c_void);
            } else {
                eprintln!("JS Error (unknown)");
            }
            std::process::exit(1);
        }

        // Release result if it holds a pointer.
        if result.kind >= HermesValueKind_Symbol {
            hermes__Value__Release(
                &result as *const HermesValue as *mut HermesValue,
            );
        }

        hermes__Object__Release(global);
        hermes__Runtime__Delete(rt);
    }
}
