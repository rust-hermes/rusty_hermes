mod from_js;
mod into_js;
mod op;

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

/// Derive `IntoJs` for a struct or enum.
///
/// - Named structs become JS objects with field names as keys.
/// - Newtype structs are transparent (inner value is used directly).
/// - Tuple structs become JS arrays.
/// - Unit structs become `null`.
/// - Enum unit variants become JS strings (`"VariantName"`).
/// - Enum struct/tuple/newtype variants become `{"VariantName": payload}`.
#[proc_macro_derive(IntoJs)]
pub fn derive_into_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_js::expand(&input).into()
}

/// Derive `FromJs` for a struct or enum.
///
/// Also generates `FromJsArg` so the type can be used as a host function argument.
///
/// - Named structs are read from JS objects by field name.
/// - Newtype structs transparently convert the inner value.
/// - Tuple structs are read from JS arrays.
/// - Enum unit variants are read from JS strings.
/// - Enum struct/tuple/newtype variants are read from `{"VariantName": payload}`.
#[proc_macro_derive(FromJs)]
pub fn derive_from_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    from_js::expand(&input).into()
}

/// Mark a function as a Hermes op for easy registration.
///
/// ```ignore
/// #[hermes_op]
/// fn add(a: f64, b: f64) -> f64 {
///     a + b
/// }
///
/// // Register with:
/// add::register(&rt)?;
/// ```
///
/// The function's argument types must implement `FromJsArg` and the return
/// type must implement `IntoJsRet`. Use `#[hermes_op(name = "customName")]`
/// to override the JS function name.
#[proc_macro_attribute]
pub fn hermes_op(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as op::OpArgs);
    let func = parse_macro_input!(item as ItemFn);
    op::expand(&args, &func).into()
}
