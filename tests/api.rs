use rusty_hermes::{Array, Function, JsString, Object, Runtime, Value};

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
