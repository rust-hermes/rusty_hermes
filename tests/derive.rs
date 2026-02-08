use rusty_hermes::{hermes_op, FromJs, IntoJs, Runtime};

// -- IntoJs / FromJs derive for structs ---------------------------------------

#[derive(IntoJs, FromJs, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

#[test]
fn roundtrip_named_struct() {
    let rt = Runtime::new().unwrap();
    let p = Point { x: 1.0, y: 2.0 };
    let val = rusty_hermes::IntoJs::into_js(p, &rt).unwrap();
    let p2: Point = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(p2, Point { x: 1.0, y: 2.0 });
}

#[derive(IntoJs, FromJs, Debug, PartialEq)]
struct Wrapper(i32);

#[test]
fn roundtrip_newtype_struct() {
    let rt = Runtime::new().unwrap();
    let w = Wrapper(42);
    let val = rusty_hermes::IntoJs::into_js(w, &rt).unwrap();
    let w2: Wrapper = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(w2, Wrapper(42));
}

#[derive(IntoJs, FromJs, Debug, PartialEq)]
struct Pair(f64, f64);

#[test]
fn roundtrip_tuple_struct() {
    let rt = Runtime::new().unwrap();
    let p = Pair(3.0, 4.0);
    let val = rusty_hermes::IntoJs::into_js(p, &rt).unwrap();
    let p2: Pair = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(p2, Pair(3.0, 4.0));
}

// -- Nested struct -----------------------------------------------------------

#[derive(IntoJs, FromJs, Debug, PartialEq)]
struct Line {
    start: Point,
    end: Point,
}

#[test]
fn roundtrip_nested_struct() {
    let rt = Runtime::new().unwrap();
    let line = Line {
        start: Point { x: 0.0, y: 0.0 },
        end: Point { x: 10.0, y: 20.0 },
    };
    let val = rusty_hermes::IntoJs::into_js(line, &rt).unwrap();
    let l2: Line = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(
        l2,
        Line {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 20.0 },
        }
    );
}

// -- Enum: unit variants -----------------------------------------------------

#[derive(IntoJs, FromJs, Debug, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn roundtrip_enum_unit_variant() {
    let rt = Runtime::new().unwrap();

    let val = rusty_hermes::IntoJs::into_js(Color::Red, &rt).unwrap();
    let c: Color = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(c, Color::Red);

    let val = rusty_hermes::IntoJs::into_js(Color::Green, &rt).unwrap();
    let c: Color = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(c, Color::Green);

    let val = rusty_hermes::IntoJs::into_js(Color::Blue, &rt).unwrap();
    let c: Color = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(c, Color::Blue);
}

// -- Enum: struct variant ----------------------------------------------------

#[derive(IntoJs, FromJs, Debug, PartialEq)]
enum Shape {
    Point,
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
}

#[test]
fn roundtrip_enum_struct_variant() {
    let rt = Runtime::new().unwrap();
    let shape = Shape::Circle { radius: 5.0 };
    let val = rusty_hermes::IntoJs::into_js(shape, &rt).unwrap();
    let s2: Shape = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(s2, Shape::Circle { radius: 5.0 });
}

#[test]
fn roundtrip_enum_unit_variant_mixed() {
    let rt = Runtime::new().unwrap();
    let val = rusty_hermes::IntoJs::into_js(Shape::Point, &rt).unwrap();
    let s2: Shape = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(s2, Shape::Point);
}

// -- Enum: newtype variant ---------------------------------------------------

#[derive(IntoJs, FromJs, Debug, PartialEq)]
enum Message {
    Text(String),
    Coords(f64, f64),
}

#[test]
fn roundtrip_enum_newtype_variant() {
    let rt = Runtime::new().unwrap();
    let msg = Message::Text("hello".into());
    let val = rusty_hermes::IntoJs::into_js(msg, &rt).unwrap();
    let m2: Message = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(m2, Message::Text("hello".into()));
}

#[test]
fn roundtrip_enum_tuple_variant() {
    let rt = Runtime::new().unwrap();
    let msg = Message::Coords(1.0, 2.0);
    let val = rusty_hermes::IntoJs::into_js(msg, &rt).unwrap();
    let m2: Message = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(m2, Message::Coords(1.0, 2.0));
}

// -- FromJs from JS eval -----------------------------------------------------

#[test]
fn from_js_eval_object() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("({x: 3.125, y: 2.75})").unwrap();
    let p: Point = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(p, Point { x: 3.125, y: 2.75 });
}

// -- Derived types as host function args/returns -----------------------------

#[hermes_op]
fn get_x(p: Point) -> f64 {
    p.x
}

#[test]
fn derived_type_as_host_func_arg() {
    let rt = Runtime::new().unwrap();
    get_x::register(&rt).unwrap();
    let val = rt.eval("get_x({x: 42, y: 0})").unwrap();
    assert_eq!(val.as_number(), Some(42.0));
}

#[hermes_op]
fn make_point(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn derived_type_as_host_func_return() {
    let rt = Runtime::new().unwrap();
    make_point::register(&rt).unwrap();
    let val = rt.eval("var p = make_point(10, 20); p.x + p.y").unwrap();
    assert_eq!(val.as_number(), Some(30.0));
}

// -- Vec<T> FromJs -----------------------------------------------------------

#[test]
fn from_js_vec() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("[1, 2, 3, 4, 5]").unwrap();
    let v: Vec<i32> = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
}

// -- HashMap FromJs ----------------------------------------------------------

#[test]
fn from_js_hashmap() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("({a: 1, b: 2, c: 3})").unwrap();
    let map: std::collections::HashMap<String, i32> =
        rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(map.len(), 3);
    assert_eq!(map["a"], 1);
    assert_eq!(map["b"], 2);
    assert_eq!(map["c"], 3);
}

// -- Error propagation in trampoline -----------------------------------------

#[hermes_op]
fn fail() -> rusty_hermes::Result<i32> {
    Err(rusty_hermes::Error::RuntimeError("boom".into()))
}

#[test]
fn trampoline_error_becomes_js_exception() {
    let rt = Runtime::new().unwrap();
    fail::register(&rt).unwrap();
    let result = rt.eval("fail()");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("boom"), "error should contain 'boom': {err_msg}");
}

#[hermes_op]
fn need_num(n: f64) -> f64 {
    n * 2.0
}

#[test]
fn trampoline_type_error_becomes_js_exception() {
    let rt = Runtime::new().unwrap();
    need_num::register(&rt).unwrap();
    let result = rt.eval("need_num('hello')");
    assert!(result.is_err());
}

// -- Small integer types -----------------------------------------------------

#[test]
fn roundtrip_small_integers() {
    let rt = Runtime::new().unwrap();

    let val = rusty_hermes::IntoJs::into_js(42i8, &rt).unwrap();
    assert_eq!(i8::from_js(&rt, &val).unwrap(), 42i8);

    let val = rusty_hermes::IntoJs::into_js(200u8, &rt).unwrap();
    assert_eq!(u8::from_js(&rt, &val).unwrap(), 200u8);

    let val = rusty_hermes::IntoJs::into_js(-1000i16, &rt).unwrap();
    assert_eq!(i16::from_js(&rt, &val).unwrap(), -1000i16);

    let val = rusty_hermes::IntoJs::into_js(50000u16, &rt).unwrap();
    assert_eq!(u16::from_js(&rt, &val).unwrap(), 50000u16);

    let val = rusty_hermes::IntoJs::into_js(42usize, &rt).unwrap();
    assert_eq!(usize::from_js(&rt, &val).unwrap(), 42usize);

    let val = rusty_hermes::IntoJs::into_js(-7isize, &rt).unwrap();
    assert_eq!(isize::from_js(&rt, &val).unwrap(), -7isize);
}

#[hermes_op]
fn add_u8(a: u8, b: u8) -> u8 {
    a + b
}

#[test]
fn hermes_op_small_integers() {
    let rt = Runtime::new().unwrap();
    add_u8::register(&rt).unwrap();
    let val = rt.eval("add_u8(10, 20)").unwrap();
    assert_eq!(val.as_number(), Some(30.0));
}

// -- HashMap IntoJs ----------------------------------------------------------

#[test]
fn roundtrip_hashmap() {
    let rt = Runtime::new().unwrap();
    let mut map = std::collections::HashMap::new();
    map.insert("x".to_string(), 10i32);
    map.insert("y".to_string(), 20i32);
    let val = rusty_hermes::IntoJs::into_js(map, &rt).unwrap();
    let map2: std::collections::HashMap<String, i32> =
        rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(map2["x"], 10);
    assert_eq!(map2["y"], 20);
}

// -- BTreeMap ----------------------------------------------------------------

#[test]
fn roundtrip_btreemap() {
    let rt = Runtime::new().unwrap();
    let mut map = std::collections::BTreeMap::new();
    map.insert("a".to_string(), 1.0f64);
    map.insert("b".to_string(), 2.0);
    let val = rusty_hermes::IntoJs::into_js(map, &rt).unwrap();
    let map2: std::collections::BTreeMap<String, f64> =
        rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(map2["a"], 1.0);
    assert_eq!(map2["b"], 2.0);
}

// -- HashSet -----------------------------------------------------------------

#[test]
fn roundtrip_hashset() {
    let rt = Runtime::new().unwrap();
    let set: std::collections::HashSet<i32> = [1, 2, 3].into_iter().collect();
    let val = rusty_hermes::IntoJs::into_js(set, &rt).unwrap();
    let set2: std::collections::HashSet<i32> =
        rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(set2.len(), 3);
    assert!(set2.contains(&1));
    assert!(set2.contains(&2));
    assert!(set2.contains(&3));
}

// -- BTreeSet ----------------------------------------------------------------

#[test]
fn roundtrip_btreeset() {
    let rt = Runtime::new().unwrap();
    let set: std::collections::BTreeSet<i32> = [10, 20, 30].into_iter().collect();
    let val = rusty_hermes::IntoJs::into_js(set, &rt).unwrap();
    let set2: std::collections::BTreeSet<i32> =
        rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(set2, [10, 20, 30].into_iter().collect());
}

// -- Tuples ------------------------------------------------------------------

#[test]
fn roundtrip_tuple_1() {
    let rt = Runtime::new().unwrap();
    let t = (42i32,);
    let val = rusty_hermes::IntoJs::into_js(t, &rt).unwrap();
    let t2: (i32,) = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(t2, (42,));
}

#[test]
fn roundtrip_tuple_2() {
    let rt = Runtime::new().unwrap();
    let t = (3.14f64, "hello".to_string());
    let val = rusty_hermes::IntoJs::into_js(t, &rt).unwrap();
    let t2: (f64, String) = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(t2, (3.14, "hello".to_string()));
}

#[test]
fn roundtrip_tuple_3() {
    let rt = Runtime::new().unwrap();
    let t = (1i32, true, "hi".to_string());
    let val = rusty_hermes::IntoJs::into_js(t, &rt).unwrap();
    let t2: (i32, bool, String) = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(t2, (1, true, "hi".to_string()));
}

#[test]
fn tuple_from_js_array() {
    let rt = Runtime::new().unwrap();
    let val = rt.eval("[10, 'world']").unwrap();
    let t: (i32, String) = rusty_hermes::FromJs::from_js(&rt, &val).unwrap();
    assert_eq!(t, (10, "world".to_string()));
}
