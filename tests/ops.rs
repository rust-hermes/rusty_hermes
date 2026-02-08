use rusty_hermes::{hermes_op, FromJs, IntoJs, Runtime};

// -- Basic hermes_op ---------------------------------------------------------

#[hermes_op]
fn add(a: f64, b: f64) -> f64 {
    a + b
}

#[test]
fn hermes_op_basic() {
    let rt = Runtime::new().unwrap();
    add::register(&rt).unwrap();
    let val = rt.eval("add(10, 20)").unwrap();
    assert_eq!(val.as_number(), Some(30.0));
}

// -- hermes_op with custom name ----------------------------------------------

#[hermes_op(name = "greet")]
fn greet_impl(name: String) -> String {
    format!("Hello, {name}!")
}

#[test]
fn hermes_op_custom_name() {
    let rt = Runtime::new().unwrap();
    greet_impl::register(&rt).unwrap();
    let val = rt.eval("greet('World')").unwrap();
    let s = val.into_string().unwrap().to_rust_string().unwrap();
    assert_eq!(s, "Hello, World!");
}

// -- hermes_op with Result return type (error propagation) -------------------

#[hermes_op]
fn divide(a: f64, b: f64) -> rusty_hermes::Result<f64> {
    if b == 0.0 {
        Err(rusty_hermes::Error::RuntimeError("division by zero".into()))
    } else {
        Ok(a / b)
    }
}

#[test]
fn hermes_op_result_ok() {
    let rt = Runtime::new().unwrap();
    divide::register(&rt).unwrap();
    let val = rt.eval("divide(10, 2)").unwrap();
    assert_eq!(val.as_number(), Some(5.0));
}

#[test]
fn hermes_op_result_err() {
    let rt = Runtime::new().unwrap();
    divide::register(&rt).unwrap();
    let result = rt.eval("divide(10, 0)");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("division by zero"), "error: {err_msg}");
}

// -- hermes_op with derived type args/returns --------------------------------

#[derive(IntoJs, FromJs, Debug, PartialEq)]
struct Vec2 {
    x: f64,
    y: f64,
}

#[hermes_op]
fn vec2_add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

#[test]
fn hermes_op_with_derived_types() {
    let rt = Runtime::new().unwrap();
    vec2_add::register(&rt).unwrap();
    let val = rt
        .eval("var r = vec2_add({x: 1, y: 2}, {x: 3, y: 4}); r.x + r.y")
        .unwrap();
    assert_eq!(val.as_number(), Some(10.0));
}

// -- hermes_op with no args --------------------------------------------------

#[hermes_op]
fn get_answer() -> i32 {
    42
}

#[test]
fn hermes_op_no_args() {
    let rt = Runtime::new().unwrap();
    get_answer::register(&rt).unwrap();
    let val = rt.eval("get_answer()").unwrap();
    assert_eq!(val.as_number(), Some(42.0));
}

// -- hermes_op with many args (proves no 8-arg limit) ------------------------

#[hermes_op]
#[allow(clippy::too_many_arguments)]
fn sum10(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64, h: f64, i: f64, j: f64) -> f64 {
    a + b + c + d + e + f + g + h + i + j
}

#[test]
fn hermes_op_ten_args() {
    let rt = Runtime::new().unwrap();
    sum10::register(&rt).unwrap();
    let val = rt.eval("sum10(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)").unwrap();
    assert_eq!(val.as_number(), Some(55.0));
}
