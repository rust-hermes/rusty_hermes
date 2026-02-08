use libhermes_sys::*;

#[test]
fn eval_simple_expression() {
    unsafe {
        let rt = hermes__Runtime__New();
        assert!(!rt.is_null());

        let script = b"1 + 2";
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

        hermes__Runtime__Delete(rt);
    }
}

#[test]
fn eval_string_result() {
    unsafe {
        let rt = hermes__Runtime__New();

        let script = b"'hello' + ' ' + 'world'";
        let url = b"test.js";

        let result = hermes__Runtime__EvaluateJavaScript(
            rt,
            script.as_ptr(),
            script.len(),
            url.as_ptr() as *const i8,
            url.len(),
        );

        assert_eq!(result.kind, HermesValueKind_String);

        // Read the string value.
        let needed = hermes__String__ToUtf8(rt, result.data.pointer, std::ptr::null_mut(), 0);
        assert_eq!(needed, 11); // "hello world"

        let mut buf = vec![0u8; needed + 1];
        hermes__String__ToUtf8(
            rt,
            result.data.pointer,
            buf.as_mut_ptr() as *mut i8,
            buf.len(),
        );
        let s = std::str::from_utf8(&buf[..needed]).unwrap();
        assert_eq!(s, "hello world");

        hermes__Value__Release(&result as *const HermesValue as *mut HermesValue);
        hermes__Runtime__Delete(rt);
    }
}

#[test]
fn global_object_property() {
    unsafe {
        let rt = hermes__Runtime__New();

        // Set x = 42 via eval.
        let script = b"var x = 42; x";
        let url = b"test.js";
        let result = hermes__Runtime__EvaluateJavaScript(
            rt,
            script.as_ptr(),
            script.len(),
            url.as_ptr() as *const i8,
            url.len(),
        );
        assert_eq!(result.kind, HermesValueKind_Number);
        assert_eq!(result.data.number, 42.0);

        // Read x from global object.
        let global = hermes__Runtime__Global(rt);
        let prop_name = hermes__PropNameID__ForAscii(rt, b"x".as_ptr() as *const i8, 1);
        let val = hermes__Object__GetProperty__PropNameID(rt, global, prop_name);
        assert_eq!(val.kind, HermesValueKind_Number);
        assert_eq!(val.data.number, 42.0);

        hermes__PropNameID__Release(prop_name);
        hermes__Object__Release(global);
        hermes__Runtime__Delete(rt);
    }
}
