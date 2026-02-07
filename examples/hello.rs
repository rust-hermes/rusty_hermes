//! Minimal example — evaluate JavaScript and print the result.
//!
//! Run with:
//!   cargo run --example hello

use rusty_hermes::Runtime;

fn main() {
    let rt = Runtime::new().expect("failed to create Hermes runtime");

    // Evaluate a simple expression
    let val = rt.eval("1 + 2").unwrap();
    println!("1 + 2 = {}", val.as_number().unwrap());

    // Evaluate a string expression
    let val = rt.eval("'Hello' + ' from ' + 'Hermes!'").unwrap();
    let s = val.into_string().unwrap();
    println!("{}", s.to_rust_string().unwrap());
}
