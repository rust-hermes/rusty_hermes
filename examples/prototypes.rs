//! Prototype operations demo — create objects with prototypes, get/set
//! prototype chains, and demonstrate inheritance.
//!
//! Run with:
//!   cargo run --example prototypes

use rusty_hermes::{Object, Runtime, Value};

fn main() {
    let rt = Runtime::new().expect("failed to create Hermes runtime");

    // Create a prototype object with a method
    let animal = Object::new(&rt);
    animal
        .set("type", Value::from(rusty_hermes::JsString::new(&rt, "animal")))
        .unwrap();
    animal
        .set("legs", Value::from_number(4.0))
        .unwrap();

    println!("Prototype: type={}", {
        let t = animal.get("type").unwrap().into_string().unwrap();
        t.to_rust_string().unwrap()
    });

    // Create a child object with the prototype
    let proto_val: Value = animal.into();
    let dog = Object::create_with_prototype(&rt, &proto_val).unwrap();
    dog.set("name", Value::from(rusty_hermes::JsString::new(&rt, "Rex")))
        .unwrap();

    // Child inherits properties from prototype
    let inherited_type = dog.get("type").unwrap().into_string().unwrap();
    println!(
        "Dog name={}, inherited type={}",
        dog.get("name")
            .unwrap()
            .into_string()
            .unwrap()
            .to_rust_string()
            .unwrap(),
        inherited_type.to_rust_string().unwrap()
    );

    // Verify prototype chain
    let retrieved = dog.get_prototype().unwrap();
    assert!(retrieved.is_object());
    println!("get_prototype() returned an object: OK");

    // Change prototype dynamically
    let new_proto = Object::new(&rt);
    new_proto
        .set("sound", Value::from(rusty_hermes::JsString::new(&rt, "meow")))
        .unwrap();
    let new_proto_val: Value = new_proto.into();
    dog.set_prototype(&new_proto_val).unwrap();

    let sound = dog.get("sound").unwrap().into_string().unwrap();
    println!("After set_prototype, sound = {}", sound.to_rust_string().unwrap());

    // Old inherited property is gone
    let old_type = dog.get("type").unwrap();
    assert!(old_type.is_undefined());
    println!("Old inherited 'type' is now undefined: OK");

    // Unique IDs
    let obj_a = Object::new(&rt);
    let obj_b = Object::new(&rt);
    println!(
        "Unique IDs: a={}, b={} (different={})",
        obj_a.unique_id(),
        obj_b.unique_id(),
        obj_a.unique_id() != obj_b.unique_id()
    );

    println!("\nAll prototype operations working!");
}
