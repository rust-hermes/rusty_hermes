//! Host functions demo — Rust equivalent of
//! https://github.com/tmikov/hermes-jsi-demos/tree/master/host-functions
//!
//! Registers two host functions into the Hermes runtime:
//!   - `add(a, b, c)` — returns the sum of three numbers
//!   - `myPrint(msg, val)` — prints a message and a value to stdout
//!
//! Then evaluates a JS file passed as a command-line argument (or runs an
//! inline demo if none is given).
//!
//! Run with:
//!   cargo run --example host_functions
//!   cargo run --example host_functions -- examples/demo.js

use rusty_hermes::{Runtime, hermes_op};
use std::env;
use std::fs;
use std::process;

#[hermes_op]
fn add(a: f64, b: f64, c: f64) -> f64 {
    a + b + c
}

#[hermes_op(name = "myPrint")]
fn my_print(msg: String, val: f64) {
    println!("{msg} {val}");
}

fn main() {
    let rt = Runtime::new().expect("failed to create Hermes runtime");

    add::register(&rt).expect("failed to register add()");
    my_print::register(&rt).expect("failed to register myPrint()");

    // Either evaluate a JS file from argv[1] or run the inline demo.
    let code = match env::args().nth(1) {
        Some(path) => fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("error reading {path}: {e}");
            process::exit(1);
        }),
        None => r#"myPrint("Host function add() returned", add(10, 20, 30));"#.to_string(),
    };

    match rt.eval(&code) {
        Ok(val) => drop(val),
        Err(e) => {
            eprintln!("JS error: {e}");
            process::exit(1);
        }
    };
}
