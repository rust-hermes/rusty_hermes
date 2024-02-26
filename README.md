# rusty_hermes

> [!WARNING]
> This crate currently only supports on Linux and macOS. Adding Windows support should be trivial.

[Hermes](https://hermesengine.dev) JavaScript engine wrapper for Rust programming language.

[→ Browse Docs](https://rust-hermes.github.io/rusty_hermes/)

Crates:

- [`rusty_hermes`](./) - High-level wrapper for libhermesabi-sys (WIP).
- [`libhermesabi-sys`](./libhermesabi-sys) - Hermes C ABI bindings using bindgen.

Install the required dependencies:

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

```
rusty_hermes = { git = "https://github.com/rust-hermes/rusty_hermes", branch = "main" }
```
