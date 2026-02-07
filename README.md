# rusty_hermes

> [!WARNING]
> This crate currently only supports Linux and macOS. Adding Windows support should be trivial.

Rust wrapper for [Hermes](https://hermesengine.dev) JavaScript engine.

[→ Browse Docs](https://rust-hermes.github.io/rusty_hermes/)

## Quick start

```rust
use rusty_hermes::Runtime;

let rt = Runtime::new().unwrap();

// Evaluate JavaScript
let val = rt.eval("1 + 2").unwrap();
assert_eq!(val.as_number(), Some(3.0));

// Register a host function
rt.set_func("add", |a: f64, b: f64| -> f64 { a + b }).unwrap();
let result = rt.eval("add(10, 20)").unwrap();
assert_eq!(result.as_number(), Some(30.0));
```

## Crates

- [`rusty_hermes`](./) - High-level, safe Rust bindings with lifetime-based memory safety.
- [`libhermesabi-sys`](./libhermesabi-sys) - Low-level C FFI bindings (rusty_v8 style).

## Features

- Evaluate JavaScript and get typed results (numbers, strings, booleans, objects, arrays)
- Register Rust functions as JS host functions with automatic type conversion
- Manipulate JS objects and arrays from Rust
- Lifetime-based safety — all JS values are tied to their `Runtime`, preventing use-after-free at compile time
- Error handling with `Result` types for JS exceptions and type errors

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
```
