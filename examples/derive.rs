//! Using derive macros for automatic Rust ↔ JS conversion.
//!
//! Run with:
//!   cargo run --example derive

use rusty_hermes::{FromJs, IntoJs, Runtime, hermes_op};

#[derive(IntoJs, FromJs, Debug, PartialEq)]
struct User {
    name: String,
    age: i32,
    active: bool,
    scores: Vec<f64>,
}

#[derive(IntoJs, FromJs, Debug, PartialEq)]
enum Role {
    Admin,
    Member { level: i32 },
}

#[hermes_op]
fn create_user(name: String, age: i32) -> User {
    User {
        name,
        age,
        active: true,
        scores: vec![],
    }
}

fn main() {
    let rt = Runtime::new().expect("failed to create Hermes runtime");

    // Serialize a Rust struct to a JS object via derive(IntoJs)
    let user = User {
        name: "Alice".into(),
        age: 30,
        active: true,
        scores: vec![95.0, 87.5, 92.0],
    };
    let js_val = rusty_hermes::IntoJs::into_js(user, &rt).unwrap();

    // Pass it to JS and use it
    let global = rt.global();
    global.set("user", js_val).unwrap();
    let greeting = rt
        .eval("'Hello, ' + user.name + '! You have ' + user.scores.length + ' scores.'")
        .unwrap();
    let greeting_str = greeting.into_string().unwrap().to_rust_string().unwrap();
    println!("{greeting_str}");

    // Deserialize a JS object back to Rust via derive(FromJs)
    let js_obj = rt
        .eval("({name: 'Bob', age: 25, active: false, scores: [100, 88]})")
        .unwrap();
    let bob: User = rusty_hermes::FromJs::from_js(&rt, &js_obj).unwrap();
    println!("{bob:?}");

    // Enums
    let admin = rusty_hermes::IntoJs::into_js(Role::Admin, &rt).unwrap();
    let member = rusty_hermes::IntoJs::into_js(Role::Member { level: 5 }, &rt).unwrap();
    let a: Role = rusty_hermes::FromJs::from_js(&rt, &admin).unwrap();
    let m: Role = rusty_hermes::FromJs::from_js(&rt, &member).unwrap();
    println!("admin = {a:?}, member = {m:?}");

    // Register a hermes_op and call it from JS
    create_user::register(&rt).unwrap();
    let result = rt
        .eval("var u = create_user('Charlie', 28); u.name + ' is ' + u.age")
        .unwrap();
    let s = result.into_string().unwrap().to_rust_string().unwrap();
    println!("{s}");
}
