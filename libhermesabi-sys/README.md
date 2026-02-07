# libhermesabi-sys

> [!WARNING]
> This crate currently only supports Linux and macOS. Adding Windows support should be trivial.

Low-level Rust FFI bindings for the [Hermes](https://hermesengine.dev) JavaScript engine.

Provides flat `extern "C"` functions wrapping the Hermes JSI C++ API (rusty_v8 style). Pointer types (String, Object, Array, etc.) are represented as opaque `*mut c_void` handles that must be explicitly released.

For a safe, high-level API, see [`rusty_hermes`](../).

## Example

```rust
use libhermesabi_sys::*;

unsafe {
    let rt = hermes__Runtime__New();

    let script = b"1 + 2";
    let url = b"test.js";

    let result = hermes__Runtime__EvaluateJavaScript(
        rt,
        script.as_ptr(),
        script.len(),
        url.as_ptr() as *const i8,
        url.len(),
    );

    assert_eq!(result.kind, HermesValueKind_Number);
    assert_eq!(result.data.number, 3.0);

    hermes__Runtime__Delete(rt);
}
```

## API surface

- **Runtime** — create (default/custom config), delete, evaluate JS, evaluate with source map, drain microtasks, get global object, parse JSON, description, inspectable
- **RuntimeConfig** — eval, Promise, Proxy, Class, Intl, microtask queue, generators, block scoping, HermesInternal, max registers
- **PreparedJavaScript** — prepare, evaluate, delete
- **Scope** — push/pop handle scopes
- **String** — create from UTF-8/ASCII, convert to UTF-8, equality, release
- **PropNameID** — create from string/UTF-8/ASCII/symbol, convert to UTF-8, equality, release
- **Object** — create, get/set/has property (string and PropNameID keys), property names, type checks, instanceof, external memory pressure, NativeState (has/get/set), HostObject (create/get/is), release
- **Array** — create, size, get/set by index, release
- **ArrayBuffer** — create, size, data pointer
- **Function** — call (with/without this), call as constructor, create from host function, is host function, release
- **Value** — release, strict equality, to string, clone
- **Symbol** — to string, equality, release
- **BigInt** — create from i64/u64, type checks, truncate, to string, equality, release
- **WeakObject** — create, lock, release
- **Hermes-specific** — bytecode check/version/sanity/prefetch, time limits (watch/unwatch/trigger), sampling profiler (enable/disable/dump)

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
libhermesabi-sys = { git = "https://github.com/rust-hermes/rusty_hermes", branch = "main" }
```
