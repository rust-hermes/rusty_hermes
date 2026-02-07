//! Working with JS objects and arrays from Rust.
//!
//! Run with:
//!   cargo run --example objects_and_arrays

use rusty_hermes::{Array, JsString, Runtime, Value};

fn main() {
    let rt = Runtime::new().expect("failed to create Hermes runtime");

    // Create an array from Rust and pass it to JS
    let arr = Array::new(&rt, 3);
    arr.set(0, Value::from_number(10.0)).unwrap();
    arr.set(1, Value::from_number(20.0)).unwrap();
    arr.set(2, Value::from_number(30.0)).unwrap();

    let global = rt.global();
    global.set("nums", arr.into()).unwrap();

    let sum = rt.eval("nums.reduce((a, b) => a + b, 0)").unwrap();
    println!("sum of [10, 20, 30] = {}", sum.as_number().unwrap());

    // Read object properties from JS
    let obj = rt
        .eval("({name: 'Hermes', version: 1})")
        .unwrap()
        .into_object()
        .unwrap();

    let name: JsString = obj.get("name").unwrap().into_string().unwrap();
    let version = obj.get("version").unwrap().as_number().unwrap();
    println!("engine: {} v{}", name.to_rust_string().unwrap(), version);

    // Get property names
    let names = obj.property_names().unwrap();
    print!("properties:");
    for i in 0..names.len() {
        let key = names.get(i).unwrap().into_string().unwrap();
        print!(" {}", key.to_rust_string().unwrap());
    }
    println!();
}
