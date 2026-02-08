# rusty_hermes

> [!WARNING]
> This crate currently only supports Linux and macOS. Adding Windows support should be trivial.

Rust wrapper for [Hermes](https://hermesengine.dev) JavaScript engine.

[→ Browse Docs](https://rust-hermes.github.io/rusty_hermes/)

## Quick start

```rust
use rusty_hermes::{Runtime, hermes_op};

#[hermes_op]
fn add(a: f64, b: f64) -> f64 { a + b }

let rt = Runtime::new().unwrap();

// Evaluate JavaScript
let val = rt.eval("1 + 2").unwrap();
assert_eq!(val.as_number(), Some(3.0));

// Register a host function and call it from JS
add::register(&rt).unwrap();
let result = rt.eval("add(10, 20)").unwrap();
assert_eq!(result.as_number(), Some(30.0));
```

## Derive macros

Use `#[derive(IntoJs)]` and `#[derive(FromJs)]` for automatic Rust ↔ JS conversion:

```rust
use rusty_hermes::{IntoJs, FromJs, Runtime};

#[derive(IntoJs, FromJs, Debug)]
struct User { name: String, age: i32 }

let rt = Runtime::new().unwrap();

// Rust struct → JS object
let val = rusty_hermes::IntoJs::into_js(
    User { name: "Alice".into(), age: 30 }, &rt
).unwrap();

// JS object → Rust struct
let js_obj = rt.eval("({name: 'Bob', age: 25})").unwrap();
let user: User = rusty_hermes::FromJs::from_js(&rt, &js_obj).unwrap();
```

## Ops system

Use `#[hermes_op]` to define host functions with automatic type conversion and error propagation:

```rust
use rusty_hermes::{hermes_op, IntoJs, FromJs, Runtime};

#[derive(IntoJs, FromJs)]
struct Point { x: f64, y: f64 }

#[hermes_op]
fn add_points(a: Point, b: Point) -> Point {
    Point { x: a.x + b.x, y: a.y + b.y }
}

let rt = Runtime::new().unwrap();
add_points::register(&rt).unwrap();
rt.eval("add_points({x: 1, y: 2}, {x: 3, y: 4})").unwrap();
```

## Crates

- [`rusty_hermes`](./) - High-level, safe Rust bindings with lifetime-based memory safety.
- [`rusty_hermes_macros`](./rusty_hermes_macros) - Derive macros for `IntoJs`, `FromJs`, and `#[hermes_op]`.
- [`libhermes-sys`](./libhermes-sys) - Low-level C FFI bindings (rusty_v8 style).

## Features

- **Evaluate JavaScript** — eval strings, prepared scripts, and JSON
- **Type-safe values** — numbers, strings, booleans, objects, arrays, symbols, bigints, arraybuffers
- **Rich type conversions** — `IntoJs`/`FromJs` for all numeric types, `String`, `bool`, `Option<T>`, `Vec<T>`, `HashMap<String, T>`, `BTreeMap<String, T>`, `HashSet<T>`, `BTreeSet<T>`
- **Host functions** — register Rust closures as JS functions with automatic type conversion
- **Host objects** — create JS objects backed by Rust callbacks for custom get/set/property enumeration
- **Object manipulation** — get/set/has/delete properties, enumeration, instanceof, NativeState, prototypes
- **Lifetime safety** — all JS values carry a `'rt` lifetime tied to their `Runtime`, preventing use-after-free at compile time
- **Derive macros** — `#[derive(IntoJs, FromJs)]` for automatic Rust ↔ JS struct/enum conversion
- **Ops system** — `#[hermes_op]` attribute macro for declaring host functions with auto type conversion and error propagation

## Installation

Install the required build dependencies:

**Ubuntu**

```
apt install cmake git ninja-build libicu-dev python zip libreadline-dev
```

**Arch**

```
pacman -S cmake git ninja icu python zip readline
```

**macOS via Homebrew**

```
brew install cmake git ninja
```

Add to your **Cargo.toml**:

```toml
rusty_hermes = { git = "https://github.com/rust-hermes/rusty_hermes", branch = "main" }
```

## Examples

Run the examples with:

```
cargo run --example hello
cargo run --example host_functions
cargo run --example objects_and_arrays
cargo run --example advanced
cargo run --example derive
cargo run --example prototypes
```
