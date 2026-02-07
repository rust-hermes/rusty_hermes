use rusty_hermes::{
    Array, ArrayBuffer, BigInt, Function, JsString, Object, PropNameId, Runtime, RuntimeConfig,
    Scope, Value, WeakObject,
};

#[test]
fn eval_number() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("1 + 2").unwrap();
    assert_eq!(val.as_number(), Some(3.0));
}

#[test]
fn eval_string() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("'hello' + ' ' + 'world'").unwrap();
    let js_str: JsString = val.into_string().unwrap();
    assert_eq!(js_str.to_rust_string().unwrap(), "hello world");
}

#[test]
fn eval_boolean() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("true").unwrap();
    assert_eq!(val.as_bool(), Some(true));

    let val2 = rt.eval("1 > 2").unwrap();
    assert_eq!(val2.as_bool(), Some(false));
}

#[test]
fn eval_undefined_null() {
    let rt = Runtime::new().unwrap();
    let u = rt.eval("undefined").unwrap();
    assert!(u.is_undefined());

    let n = rt.eval("null").unwrap();
    assert!(n.is_null());
}

#[test]
fn global_property() {
    let rt = Runtime::new().unwrap();
    let global = rt.global();

    // Set a number
    global.set("x", Value::from_number(42.0)).unwrap();
    let val = rt.eval("x").unwrap();
    assert_eq!(val.as_number(), Some(42.0));

    // Set a string
    let js_str = JsString::new(&rt, "hello from rust");
    global.set("greeting", js_str.into()).unwrap();
    let val2 = rt.eval("greeting").unwrap();
    let s: JsString = val2.into_string().unwrap();
    assert_eq!(s.to_rust_string().unwrap(), "hello from rust");
}

#[test]
fn object_get_set() {
    let rt = Runtime::new().unwrap();
    let obj: Object = rt.eval("({a: 1, b: 'two'})").unwrap().into_object().unwrap();

    let a = obj.get("a").unwrap();
    assert_eq!(a.as_number(), Some(1.0));

    let b = obj.get("b").unwrap();
    let b_str: JsString = b.into_string().unwrap();
    assert_eq!(b_str.to_rust_string().unwrap(), "two");

    assert!(obj.has("a"));
    assert!(!obj.has("nonexistent"));
}

#[test]
fn object_property_names() {
    let rt = Runtime::new().unwrap();
    let obj: Object = rt.eval("({x: 1, y: 2, z: 3})").unwrap().into_object().unwrap();
    let names = obj.property_names().unwrap();
    assert_eq!(names.len(), 3);
}

#[test]
fn array_ops() {
    let rt = Runtime::new().unwrap();
    let arr: Array = rt.eval("[10, 20, 30]").unwrap().into_array().unwrap();

    assert_eq!(arr.len(), 3);
    assert!(!arr.is_empty());

    assert_eq!(arr.get(0).unwrap().as_number(), Some(10.0));
    assert_eq!(arr.get(1).unwrap().as_number(), Some(20.0));
    assert_eq!(arr.get(2).unwrap().as_number(), Some(30.0));

    // Set a value
    arr.set(1, Value::from_number(99.0)).unwrap();
    let check = rt.eval("[10, 20, 30]").unwrap().into_array().unwrap();
    // Original array was mutated
    assert_eq!(arr.get(1).unwrap().as_number(), Some(99.0));
    // New array is independent
    assert_eq!(check.get(1).unwrap().as_number(), Some(20.0));
}

#[test]
fn create_array() {
    let rt = Runtime::new().unwrap();
    let arr = Array::new(&rt, 3);
    assert_eq!(arr.len(), 3);
    arr.set(0, Value::from_number(1.0)).unwrap();
    arr.set(1, Value::from_number(2.0)).unwrap();
    arr.set(2, Value::from_number(3.0)).unwrap();

    // Put it on global and read back
    let global = rt.global();
    global.set("myArr", arr.into()).unwrap();
    let sum = rt.eval("myArr[0] + myArr[1] + myArr[2]").unwrap();
    assert_eq!(sum.as_number(), Some(6.0));
}

#[test]
fn host_function_add() {
    let rt = Runtime::new().unwrap();
    rt.set_func("add", |a: f64, b: f64| -> f64 { a + b }).unwrap();

    let result = rt.eval("add(10, 20)").unwrap();
    assert_eq!(result.as_number(), Some(30.0));
}

#[test]
fn host_function_string() {
    let rt = Runtime::new().unwrap();
    rt.set_func("greet", |name: String| -> String {
        format!("Hello, {name}!")
    })
    .unwrap();

    let result = rt.eval("greet('Rust')").unwrap();
    let s: JsString = result.into_string().unwrap();
    assert_eq!(s.to_rust_string().unwrap(), "Hello, Rust!");
}

#[test]
fn host_function_no_args() {
    let rt = Runtime::new().unwrap();
    rt.set_func("getFortyTwo", || -> f64 { 42.0 }).unwrap();

    let result = rt.eval("getFortyTwo()").unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn host_function_three_args() {
    let rt = Runtime::new().unwrap();
    rt.set_func("sum3", |a: f64, b: f64, c: f64| -> f64 { a + b + c })
        .unwrap();

    let result = rt.eval("sum3(1, 2, 3)").unwrap();
    assert_eq!(result.as_number(), Some(6.0));
}

#[test]
fn function_call() {
    let rt = Runtime::new().unwrap();
    let func: Function = rt.eval("(function(x) { return x * 2; })").unwrap().into_function().unwrap();

    let result = func.call(&[Value::from_number(21.0)]).unwrap();
    assert_eq!(result.as_number(), Some(42.0));
}

#[test]
fn eval_error() {
    let rt = Runtime::new().unwrap();
    let result = rt.eval("throw new Error('oops')");
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        rusty_hermes::Error::JsException(msg) => {
            assert!(msg.contains("oops"), "error message was: {msg}");
        }
        other => panic!("expected JsException, got: {other:?}"),
    }
}

#[test]
fn eval_syntax_error() {
    let rt = Runtime::new().unwrap();
    let result = rt.eval("function(");
    assert!(result.is_err());
}

#[test]
fn type_error_conversion() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("42").unwrap();
    let err = val.into_string().unwrap_err();
    match err {
        rusty_hermes::Error::TypeError { expected, got } => {
            assert_eq!(expected, "string");
            assert_eq!(got, "number");
        }
        other => panic!("expected TypeError, got: {other:?}"),
    }
}

#[test]
fn value_kind() {
    let rt = Runtime::new().unwrap();

    assert_eq!(rt.eval("undefined").unwrap().kind(), rusty_hermes::ValueKind::Undefined);
    assert_eq!(rt.eval("null").unwrap().kind(), rusty_hermes::ValueKind::Null);
    assert_eq!(rt.eval("true").unwrap().kind(), rusty_hermes::ValueKind::Boolean);
    assert_eq!(rt.eval("42").unwrap().kind(), rusty_hermes::ValueKind::Number);
    assert_eq!(rt.eval("'hi'").unwrap().kind(), rusty_hermes::ValueKind::String);
    assert_eq!(rt.eval("({})").unwrap().kind(), rusty_hermes::ValueKind::Object);
}

#[test]
fn value_constructors() {
    let u = Value::undefined();
    assert!(u.is_undefined());

    let n = Value::null();
    assert!(n.is_null());

    let b = Value::from_bool(true);
    assert_eq!(b.as_bool(), Some(true));

    let num = Value::from_number(3.14);
    assert_eq!(num.as_number(), Some(3.14));
}

#[test]
fn object_is_checks() {
    let rt = Runtime::new().unwrap();
    let arr_obj: Object = rt.eval("[1,2,3]").unwrap().into_object().unwrap();
    assert!(arr_obj.is_array());
    assert!(!arr_obj.is_function());

    let func_obj: Object = rt.eval("(function(){})").unwrap().into_object().unwrap();
    assert!(func_obj.is_function());
    assert!(!func_obj.is_array());
}

#[test]
fn js_string_operations() {
    let rt = Runtime::new().unwrap();
    let s1 = JsString::new(&rt, "hello");
    let s2 = JsString::new(&rt, "hello");
    let s3 = JsString::new(&rt, "world");

    assert!(s1.strict_equals(&s2));
    assert!(!s1.strict_equals(&s3));
    assert_eq!(s1.to_rust_string().unwrap(), "hello");
}

#[test]
fn runtime_with_config_default() {
    let config = RuntimeConfig::builder().build();
    let rt = Runtime::with_config(config).unwrap();
    let val = rt.eval("1 + 2").unwrap();
    assert_eq!(val.as_number(), Some(3.0));
}

#[test]
fn runtime_with_config_no_eval() {
    let config = RuntimeConfig::builder().enable_eval(false).build();
    let rt = Runtime::with_config(config).unwrap();
    // Direct eval should still work (it's top-level evaluation, not JS eval())
    let val = rt.eval("1 + 2").unwrap();
    assert_eq!(val.as_number(), Some(3.0));
    // But eval() inside JS should fail
    let result = rt.eval("eval('1 + 2')");
    assert!(result.is_err());
}

#[test]
fn runtime_with_config_no_intl() {
    let config = RuntimeConfig::builder().intl(false).build();
    let rt = Runtime::with_config(config).unwrap();
    let val = rt.eval("1 + 2").unwrap();
    assert_eq!(val.as_number(), Some(3.0));
}

// ---------------------------------------------------------------------------
// New API tests
// ---------------------------------------------------------------------------

#[test]
fn value_to_js_string() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("42").unwrap();
    let s = val.to_js_string().unwrap();
    assert_eq!(s.to_rust_string().unwrap(), "42");

    let val2 = rt.eval("true").unwrap();
    let s2 = val2.to_js_string().unwrap();
    assert_eq!(s2.to_rust_string().unwrap(), "true");
}

#[test]
fn value_duplicate() {
    let rt = Runtime::new().unwrap();
    // Primitive
    let v1 = rt.eval("42").unwrap();
    let v2 = v1.duplicate();
    assert_eq!(v2.as_number(), Some(42.0));

    // String (pointer type)
    let s1 = rt.eval("'hello'").unwrap();
    let s2 = s1.duplicate();
    assert!(s1.strict_equals(&s2));
}

#[test]
fn value_from_js_clone_pointer() {
    use rusty_hermes::FromJs;
    let rt = Runtime::new().unwrap();
    let s = rt.eval("'hello'").unwrap();
    let cloned = Value::from_js(&rt, &s).unwrap();
    assert!(s.strict_equals(&cloned));
}

#[test]
fn bigint_operations() {
    let rt = Runtime::new().unwrap();
    let bi = BigInt::from_i64(&rt, -42);
    assert!(bi.is_i64());
    assert_eq!(bi.truncate_to_i64(), -42);

    let s = bi.to_js_string(10);
    assert_eq!(s.to_rust_string().unwrap(), "-42");

    let bi2 = BigInt::from_u64(&rt, 100);
    assert!(bi2.is_u64());
    assert_eq!(bi2.truncate_to_u64(), 100);

    let s2 = bi2.to_js_string(16);
    assert_eq!(s2.to_rust_string().unwrap(), "64");
}

#[test]
fn bigint_strict_equals() {
    let rt = Runtime::new().unwrap();
    let a = BigInt::from_i64(&rt, 42);
    let b = BigInt::from_i64(&rt, 42);
    let c = BigInt::from_i64(&rt, 99);
    assert!(a.strict_equals(&b));
    assert!(!a.strict_equals(&c));
}

#[test]
fn function_call_with_this() {
    let rt = Runtime::new().unwrap();
    rt.eval("var obj = { x: 10 }").unwrap();
    let func: Function = rt
        .eval("(function() { return this.x; })")
        .unwrap()
        .into_function()
        .unwrap();
    let obj = rt.eval("obj").unwrap();
    let result = func.call_with_this(&obj, &[]).unwrap();
    assert_eq!(result.as_number(), Some(10.0));
}

#[test]
fn object_external_memory_pressure() {
    let rt = Runtime::new().unwrap();
    let obj = Object::new(&rt);
    // Just verify it doesn't crash
    obj.set_external_memory_pressure(1024);
}

#[test]
fn object_native_state() {
    let rt = Runtime::new().unwrap();
    let obj = Object::new(&rt);

    assert!(!obj.has_native_state());

    unsafe extern "C" fn noop(_data: *mut std::ffi::c_void) {}

    let data = Box::into_raw(Box::new(42u64)) as *mut std::ffi::c_void;
    unsafe { obj.set_native_state(data, noop) };

    assert!(obj.has_native_state());
    let ptr = obj.get_native_state();
    assert!(!ptr.is_null());
    let val = unsafe { *(ptr as *const u64) };
    assert_eq!(val, 42);

    // Clean up manually since our noop finalizer doesn't free
    unsafe { drop(Box::from_raw(ptr as *mut u64)) };
}

#[test]
fn object_is_host_object() {
    let rt = Runtime::new().unwrap();
    let obj = Object::new(&rt);
    assert!(!obj.is_host_object());
}

#[test]
fn propnameid_from_utf8() {
    let rt = Runtime::new().unwrap();
    let name = PropNameId::from_utf8(&rt, "hello");
    assert_eq!(name.to_rust_string().unwrap(), "hello");
}

#[test]
fn propnameid_equals() {
    let rt = Runtime::new().unwrap();
    let a = PropNameId::from_utf8(&rt, "foo");
    let b = PropNameId::from_utf8(&rt, "foo");
    let c = PropNameId::from_utf8(&rt, "bar");
    assert!(a.equals(&b));
    assert!(!a.equals(&c));
}

#[test]
fn propnameid_from_string() {
    let rt = Runtime::new().unwrap();
    let s = JsString::new(&rt, "test");
    let name = PropNameId::from_string(&rt, &s);
    assert_eq!(name.to_rust_string().unwrap(), "test");
}

#[test]
fn array_buffer_create() {
    let rt = Runtime::new().unwrap();
    let mut buf = ArrayBuffer::new(&rt, 16);
    assert_eq!(buf.size(), 16);

    let data = buf.data_mut();
    data[0] = 0xAA;
    data[1] = 0xBB;

    let data_read = buf.data();
    assert_eq!(data_read[0], 0xAA);
    assert_eq!(data_read[1], 0xBB);
}

#[test]
fn array_buffer_from_js() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("new ArrayBuffer(8)").unwrap();
    assert!(val.is_object());
    let buf: ArrayBuffer = val.into_array_buffer().unwrap();
    assert_eq!(buf.size(), 8);
}

#[test]
fn array_buffer_into_value() {
    let rt = Runtime::new().unwrap();
    let buf = ArrayBuffer::new(&rt, 4);
    let val: Value = buf.into();
    assert!(val.is_object());
}

#[test]
fn weak_object_lock() {
    let rt = Runtime::new().unwrap();
    let obj = Object::new(&rt);
    let weak = WeakObject::new(&rt, &obj);
    // Object is still alive, so lock should succeed
    let locked = weak.lock().unwrap();
    assert!(locked.is_some());
    assert!(locked.unwrap().is_object());
}

#[test]
fn scope_create() {
    let rt = Runtime::new().unwrap();
    {
        let _scope = Scope::new(&rt);
        let _val = rt.eval("42").unwrap();
    }
    // After scope drops, eval still works
    let val = rt.eval("1 + 1").unwrap();
    assert_eq!(val.as_number(), Some(2.0));
}

#[test]
fn prepared_javascript() {
    let rt = Runtime::new().unwrap();
    let prepared = rt.prepare_javascript("1 + 2", "test.js").unwrap();
    let result = rt.evaluate_prepared_javascript(&prepared).unwrap();
    assert_eq!(result.as_number(), Some(3.0));

    // Evaluate again (reuse)
    let result2 = rt.evaluate_prepared_javascript(&prepared).unwrap();
    assert_eq!(result2.as_number(), Some(3.0));
}

#[test]
fn create_value_from_json() {
    let rt = Runtime::new().unwrap();
    let val = rt.create_value_from_json(r#"{"a": 1, "b": "two"}"#).unwrap();
    let obj = val.into_object().unwrap();
    assert_eq!(obj.get("a").unwrap().as_number(), Some(1.0));
    let b = obj.get("b").unwrap().into_string().unwrap();
    assert_eq!(b.to_rust_string().unwrap(), "two");
}

#[test]
fn runtime_description() {
    let rt = Runtime::new().unwrap();
    let desc = rt.description();
    assert!(!desc.is_empty());
}

#[test]
fn runtime_is_inspectable() {
    let rt = Runtime::new().unwrap();
    // Just verify it doesn't crash; result depends on build config
    let _ = rt.is_inspectable();
}

#[test]
fn bytecode_version() {
    let version = Runtime::bytecode_version();
    assert!(version > 0);
}

#[test]
fn bytecode_checks() {
    // Random bytes are not valid bytecode
    let data = b"not bytecode";
    assert!(!Runtime::is_hermes_bytecode(data));
    assert!(!Runtime::bytecode_sanity_check(data));
}

#[test]
fn watch_time_limit() {
    let rt = Runtime::new().unwrap();
    rt.watch_time_limit(5000);
    let val = rt.eval("1 + 1").unwrap();
    assert_eq!(val.as_number(), Some(2.0));
    rt.unwatch_time_limit();
}

#[test]
fn drain_microtasks() {
    let config = RuntimeConfig::builder().microtask_queue(true).build();
    let rt = Runtime::with_config(config).unwrap();
    let drained = rt.drain_microtasks().unwrap();
    assert!(drained); // nothing to drain
}

#[test]
fn object_get_set_with_propname() {
    let rt = Runtime::new().unwrap();
    let obj = Object::new(&rt);
    let key = PropNameId::from_utf8(&rt, "myProp");

    obj.set_with_propname(&key, Value::from_number(99.0)).unwrap();
    assert!(obj.has_with_propname(&key));

    let val = obj.get_with_propname(&key).unwrap();
    assert_eq!(val.as_number(), Some(99.0));
}

#[test]
fn js_string_from_ascii() {
    let rt = Runtime::new().unwrap();
    let s = JsString::from_ascii(&rt, "hello ascii");
    assert_eq!(s.to_rust_string().unwrap(), "hello ascii");
}

#[test]
fn host_object_create() {
    use rusty_hermes::{
        HermesHostObjectFinalizer, HermesHostObjectGetCallback,
        HermesHostObjectGetPropertyNamesCallback, HermesHostObjectSetCallback,
    };

    unsafe extern "C" fn get_cb(
        _rt: *mut libhermesabi_sys::HermesRt,
        _name: *const std::ffi::c_void,
        _user_data: *mut std::ffi::c_void,
    ) -> libhermesabi_sys::HermesValue {
        libhermesabi_sys::HermesValue {
            kind: libhermesabi_sys::HermesValueKind_Number,
            data: libhermesabi_sys::HermesValueData { number: 42.0 },
        }
    }

    unsafe extern "C" fn set_cb(
        _rt: *mut libhermesabi_sys::HermesRt,
        _name: *const std::ffi::c_void,
        _value: *const libhermesabi_sys::HermesValue,
        _user_data: *mut std::ffi::c_void,
    ) {}

    unsafe extern "C" fn get_names_cb(
        _rt: *mut libhermesabi_sys::HermesRt,
        out_count: *mut usize,
        _user_data: *mut std::ffi::c_void,
    ) -> *mut *mut std::ffi::c_void {
        unsafe { *out_count = 0 };
        std::ptr::null_mut()
    }

    unsafe extern "C" fn finalizer(_user_data: *mut std::ffi::c_void) {}

    let rt = Runtime::new().unwrap();
    let host_obj = unsafe {
        Object::create_host_object(
            &rt,
            get_cb as HermesHostObjectGetCallback,
            set_cb as HermesHostObjectSetCallback,
            get_names_cb as HermesHostObjectGetPropertyNamesCallback,
            std::ptr::null_mut(),
            finalizer as HermesHostObjectFinalizer,
        )
    };

    assert!(host_obj.is_host_object());

    // Getting any property should return 42
    let val = host_obj.get("anything").unwrap();
    assert_eq!(val.as_number(), Some(42.0));
}
