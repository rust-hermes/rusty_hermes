use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, FnArg, ItemFn, LitStr, Pat, Token};

pub struct OpArgs {
    pub name: Option<String>,
}

impl Parse for OpArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(OpArgs { name: None });
        }
        let ident: syn::Ident = input.parse()?;
        if ident != "name" {
            return Err(syn::Error::new_spanned(ident, "expected `name`"));
        }
        input.parse::<Token![=]>()?;
        let lit: LitStr = input.parse()?;
        Ok(OpArgs {
            name: Some(lit.value()),
        })
    }
}

pub fn expand(args: &OpArgs, func: &ItemFn) -> TokenStream {
    let fn_name = &func.sig.ident;
    let js_name = args
        .name
        .as_deref()
        .unwrap_or(&fn_name.to_string())
        .to_string();
    let struct_name = fn_name.clone();
    let inner_name = syn::Ident::new(
        &format!("__{fn_name}_inner"),
        proc_macro2::Span::call_site(),
    );

    let vis = &func.vis;
    let sig = &func.sig;
    let block = &func.block;
    let attrs = &func.attrs;

    // Build the inner function with original signature but renamed
    let inner_sig = {
        let mut s = sig.clone();
        s.ident = inner_name.clone();
        s
    };

    // Extract parameter names and types from the function signature.
    let mut param_names = Vec::new();
    let mut param_types = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                param_names.push(pat_ident.ident.clone());
                param_types.push((*pat_type.ty).clone());
            }
        }
    }

    let param_count = param_names.len() as u32;

    // Generate per-arg extraction code.
    let arg_extractions: Vec<TokenStream> = param_names
        .iter()
        .zip(param_types.iter())
        .enumerate()
        .map(|(i, (name, ty))| {
            quote! {
                let #name = match <#ty as rusty_hermes::__private::FromJsArg>::from_arg(
                    __rt,
                    __args_slice.get(#i).unwrap_or(&__undef),
                ) {
                    Ok(v) => v,
                    Err(e) => return rusty_hermes::__private::set_error_and_return_undefined(__rt, &e),
                };
            }
        })
        .collect();

    let call_args = &param_names;

    quote! {
        #(#attrs)*
        #vis #inner_sig #block

        #[allow(non_camel_case_types)]
        #vis struct #struct_name;

        impl #struct_name {
            pub fn register(rt: &rusty_hermes::Runtime) -> rusty_hermes::Result<()> {
                unsafe extern "C" fn __trampoline(
                    __rt: *mut rusty_hermes::__private::HermesRt,
                    __this: *const rusty_hermes::__private::HermesValue,
                    __args: *const rusty_hermes::__private::HermesValue,
                    __argc: usize,
                    __user_data: *mut ::std::ffi::c_void,
                ) -> rusty_hermes::__private::HermesValue {
                    let __args_slice: &[rusty_hermes::__private::HermesValue] = if __argc > 0 {
                        ::std::slice::from_raw_parts(__args, __argc)
                    } else {
                        &[]
                    };
                    let __undef = rusty_hermes::__private::undefined_value();
                    #(#arg_extractions)*
                    match rusty_hermes::__private::IntoJsRet::into_ret(
                        #inner_name(#(#call_args),*),
                        __rt,
                    ) {
                        Ok(v) => v,
                        Err(e) => rusty_hermes::__private::set_error_and_return_undefined(__rt, &e),
                    }
                }
                rt.__register_op(#js_name, #param_count, __trampoline)
            }
        }
    }
}
