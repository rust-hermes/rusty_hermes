use libhermesabi_sys::*;
use std::ptr;

unsafe extern "C" fn add_callback(
    _rt: *mut HermesRt,
    _this_val: *const HermesValue,
    args: *const HermesValue,
    arg_count: usize,
    _user_data: *mut std::ffi::c_void,
) -> HermesValue { unsafe {
    assert!(arg_count >= 2);
    let a = &*args;
    let b = &*args.add(1);

    assert_eq!(a.kind, HermesValueKind_Number);
    assert_eq!(b.kind, HermesValueKind_Number);

    HermesValue {
        kind: HermesValueKind_Number,
        data: HermesValueData {
            number: a.data.number + b.data.number,
        },
    }
}}

unsafe extern "C" fn noop_finalizer(_user_data: *mut std::ffi::c_void) {}

#[test]
fn host_function_add() {
    unsafe {
        let rt = hermes__Runtime__New();

        // Create a PropNameID for "add".
        let name_pni =
            hermes__PropNameID__ForAscii(rt, b"add".as_ptr() as *const i8, 3);
        assert!(!name_pni.is_null());

        // Create a host function.
        let func = hermes__Function__CreateFromHostFunction(
            rt,
            name_pni,
            2, // param count
            add_callback,
            ptr::null_mut(),
            noop_finalizer,
        );
        assert!(!func.is_null());

        // Set it on the global object.
        let global = hermes__Runtime__Global(rt);
        let func_val = HermesValue {
            kind: HermesValueKind_Object,
            data: HermesValueData { pointer: func },
        };
        let ok = hermes__Object__SetProperty__PropNameID(
            rt,
            global,
            name_pni,
            &func_val,
        );
        assert!(ok);

        // Evaluate "add(1, 2)".
        let script = b"add(1, 2)";
        let url = b"test.js";
        let result = hermes__Runtime__EvaluateJavaScript(
            rt,
            script.as_ptr(),
            script.len(),
            url.as_ptr() as *const i8,
            url.len(),
        );

        assert!(!hermes__Runtime__HasPendingError(rt));
        assert_eq!(result.kind, HermesValueKind_Number);
        assert_eq!(result.data.number, 3.0);

        // Clean up — the func pointer was borrowed by func_val and set on
        // global, so global holds a reference. We release our handles.
        hermes__PropNameID__Release(name_pni);
        hermes__Object__Release(global);
        // func was transferred into func_val which was set as a property.
        // We still own the PV from CreateFromHostFunction, release it.
        hermes__Function__Release(func);

        hermes__Runtime__Delete(rt);
    }
}

#[test]
fn host_function_string_concat() {
    unsafe {
        let rt = hermes__Runtime__New();

        // Evaluate a script that returns an array.
        let script = b"[1, 'hello', true, null]";
        let url = b"test.js";
        let result = hermes__Runtime__EvaluateJavaScript(
            rt,
            script.as_ptr(),
            script.len(),
            url.as_ptr() as *const i8,
            url.len(),
        );

        assert_eq!(result.kind, HermesValueKind_Object);
        let obj = result.data.pointer;

        // Check it's an array.
        assert!(hermes__Object__IsArray(rt, obj));

        // Get size.
        let size = hermes__Array__Size(rt, obj);
        assert_eq!(size, 4);

        // Get elements.
        let v0 = hermes__Array__GetValueAtIndex(rt, obj, 0);
        assert_eq!(v0.kind, HermesValueKind_Number);
        assert_eq!(v0.data.number, 1.0);

        let v1 = hermes__Array__GetValueAtIndex(rt, obj, 1);
        assert_eq!(v1.kind, HermesValueKind_String);

        let v2 = hermes__Array__GetValueAtIndex(rt, obj, 2);
        assert_eq!(v2.kind, HermesValueKind_Boolean);
        assert!(v2.data.boolean);

        let v3 = hermes__Array__GetValueAtIndex(rt, obj, 3);
        assert_eq!(v3.kind, HermesValueKind_Null);

        // Clean up.
        hermes__Value__Release(&v1 as *const HermesValue as *mut HermesValue);
        hermes__Object__Release(obj);
        hermes__Runtime__Delete(rt);
    }
}
