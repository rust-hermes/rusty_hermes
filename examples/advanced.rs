//! Advanced features demo — RuntimeConfig, JSON, PreparedJS, ArrayBuffer,
//! BigInt, Scope, and time limits.
//!
//! Run with:
//!   cargo run --example advanced

use rusty_hermes::{
    ArrayBuffer, BigInt, PropNameId, Runtime, RuntimeConfig, Scope,
};

fn main() {
    // -- Custom runtime configuration ----------------------------------------
    let config = RuntimeConfig::builder()
        .microtask_queue(true)
        .enable_eval(false) // disable eval() inside JS
        .build();
    let rt = Runtime::with_config(config).unwrap();

    println!("Runtime: {}", rt.description());
    println!("Bytecode version: {}", Runtime::bytecode_version());

    // -- Parse JSON ----------------------------------------------------------
    let val = rt
        .create_value_from_json(r#"{"name": "Hermes", "version": 1}"#)
        .unwrap();
    let obj = val.into_object().unwrap();
    let name = obj.get("name").unwrap().into_string().unwrap();
    println!("Parsed JSON: name = {}", name.to_rust_string().unwrap());

    // -- PropNameId-based property access ------------------------------------
    let key = PropNameId::from_utf8(&rt, "version");
    let version = obj.get_with_propname(&key).unwrap();
    println!("version (via PropNameId) = {}", version.as_number().unwrap());

    // -- PreparedJavaScript (compile once, run many) -------------------------
    let prepared = rt.prepare_javascript("1 + 2 + 3", "compiled.js").unwrap();
    let r1 = rt.evaluate_prepared_javascript(&prepared).unwrap();
    let r2 = rt.evaluate_prepared_javascript(&prepared).unwrap();
    println!(
        "PreparedJS: first={}, second={}",
        r1.as_number().unwrap(),
        r2.as_number().unwrap()
    );

    // -- ArrayBuffer ---------------------------------------------------------
    let mut buf = ArrayBuffer::new(&rt, 4);
    {
        let data = buf.data_mut();
        data[0] = 0xDE;
        data[1] = 0xAD;
        data[2] = 0xBE;
        data[3] = 0xEF;
    }
    println!(
        "ArrayBuffer: [{:#04X}, {:#04X}, {:#04X}, {:#04X}]",
        buf.data()[0],
        buf.data()[1],
        buf.data()[2],
        buf.data()[3]
    );

    // Pass ArrayBuffer to JS and read it back
    let global = rt.global();
    global.set("myBuf", buf.into()).unwrap();
    let size = rt
        .eval("new Uint8Array(myBuf).length")
        .unwrap()
        .as_number()
        .unwrap();
    println!("ArrayBuffer size from JS: {}", size);

    // -- BigInt --------------------------------------------------------------
    let big = BigInt::from_i64(&rt, 9_007_199_254_740_993); // 2^53 + 1
    let s = big.to_js_string(10);
    println!("BigInt: {}", s.to_rust_string().unwrap());

    let hex = big.to_js_string(16);
    println!("BigInt (hex): 0x{}", hex.to_rust_string().unwrap());

    // -- Scope ---------------------------------------------------------------
    {
        let _scope = Scope::new(&rt);
        // Temporary values created here are scoped
        let _tmp = rt.eval("'temporary value'").unwrap();
    }
    // Scope dropped, temporaries can be GC'd
    println!("Scope test passed");

    // -- Value cloning -------------------------------------------------------
    let original = rt.eval("'hello world'").unwrap();
    let cloned = original.duplicate();
    assert!(original.strict_equals(&cloned));
    println!("Value clone: strict_equals = true");

    // -- Value to string -----------------------------------------------------
    let num = rt.eval("3.14").unwrap();
    let s = num.to_js_string().unwrap();
    println!("Value::to_js_string(3.14) = {}", s.to_rust_string().unwrap());

    // -- Time limit ----------------------------------------------------------
    rt.watch_time_limit(5000); // 5 second limit
    let val = rt.eval("42").unwrap();
    println!("With time limit: {}", val.as_number().unwrap());
    rt.unwatch_time_limit();

    // -- Drain microtasks ----------------------------------------------------
    let drained = rt.drain_microtasks().unwrap();
    println!("Microtasks drained: {drained}");

    println!("\nAll advanced features working!");
}
