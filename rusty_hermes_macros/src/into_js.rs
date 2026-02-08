use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

pub fn expand(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => expand_struct(name, &data.fields),
        Data::Enum(data) => expand_enum(name, data),
        Data::Union(_) => {
            return syn::Error::new_spanned(input, "IntoJs cannot be derived for unions")
                .to_compile_error();
        }
    };

    quote! {
        impl<'rt> #impl_generics rusty_hermes::IntoJs<'rt> for #name #ty_generics #where_clause {
            fn into_js(self, rt: &'rt rusty_hermes::Runtime) -> rusty_hermes::Result<rusty_hermes::Value<'rt>> {
                #body
            }
        }

        impl #impl_generics rusty_hermes::__private::IntoJsRet for #name #ty_generics #where_clause {
            unsafe fn into_ret(self, rt: *mut libhermes_sys::HermesRt) -> rusty_hermes::Result<libhermes_sys::HermesValue> {
                let rt_ref = unsafe { rusty_hermes::Runtime::borrow_raw(rt) };
                let val = rusty_hermes::IntoJs::into_js(self, &rt_ref)?;
                Ok(val.into_raw())
            }
        }
    }
}

fn expand_struct(name: &syn::Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(named) => {
            let field_sets: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let key = ident.to_string();
                    quote! {
                        obj.set(#key, rusty_hermes::IntoJs::into_js(self.#ident, rt)?)?;
                    }
                })
                .collect();
            quote! {
                let obj = rusty_hermes::Object::new(rt);
                #(#field_sets)*
                Ok(obj.into())
            }
        }
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() == 1 {
                // Newtype struct: transparent
                quote! {
                    rusty_hermes::IntoJs::into_js(self.0, rt)
                }
            } else {
                // Tuple struct: array
                let sets: Vec<_> = unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let idx = syn::Index::from(i);
                        quote! {
                            arr.set(#i, rusty_hermes::IntoJs::into_js(self.#idx, rt)?)?;
                        }
                    })
                    .collect();
                let len = unnamed.unnamed.len();
                quote! {
                    let arr = rusty_hermes::Array::new(rt, #len);
                    #(#sets)*
                    Ok(arr.into())
                }
            }
        }
        Fields::Unit => {
            let _ = name;
            quote! { Ok(rusty_hermes::Value::null()) }
        }
    }
}

fn expand_enum(name: &syn::Ident, data: &syn::DataEnum) -> TokenStream {
    let _ = name;
    let arms: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let vname = &variant.ident;
            let vname_str = vname.to_string();
            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#vname => {
                            Ok(rusty_hermes::IntoJs::into_js(#vname_str.to_string(), rt)?)
                        }
                    }
                }
                Fields::Named(named) => {
                    let field_idents: Vec<_> = named
                        .named
                        .iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();
                    let field_sets: Vec<_> = field_idents
                        .iter()
                        .map(|ident| {
                            let key = ident.to_string();
                            quote! {
                                inner.set(#key, rusty_hermes::IntoJs::into_js(#ident, rt)?)?;
                            }
                        })
                        .collect();
                    quote! {
                        Self::#vname { #(#field_idents),* } => {
                            let inner = rusty_hermes::Object::new(rt);
                            #(#field_sets)*
                            let outer = rusty_hermes::Object::new(rt);
                            outer.set(#vname_str, inner.into())?;
                            Ok(outer.into())
                        }
                    }
                }
                Fields::Unnamed(unnamed) => {
                    if unnamed.unnamed.len() == 1 {
                        // Newtype variant: {"Variant": value}
                        quote! {
                            Self::#vname(v) => {
                                let payload = rusty_hermes::IntoJs::into_js(v, rt)?;
                                let outer = rusty_hermes::Object::new(rt);
                                outer.set(#vname_str, payload)?;
                                Ok(outer.into())
                            }
                        }
                    } else {
                        // Tuple variant: {"Variant": [a, b, ...]}
                        let field_names: Vec<_> = (0..unnamed.unnamed.len())
                            .map(|i| {
                                syn::Ident::new(&format!("f{i}"), proc_macro2::Span::call_site())
                            })
                            .collect();
                        let sets: Vec<_> = field_names
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                quote! {
                                    arr.set(#i, rusty_hermes::IntoJs::into_js(#f, rt)?)?;
                                }
                            })
                            .collect();
                        let len = unnamed.unnamed.len();
                        quote! {
                            Self::#vname(#(#field_names),*) => {
                                let arr = rusty_hermes::Array::new(rt, #len);
                                #(#sets)*
                                let outer = rusty_hermes::Object::new(rt);
                                outer.set(#vname_str, arr.into())?;
                                Ok(outer.into())
                            }
                        }
                    }
                }
            }
        })
        .collect();

    quote! {
        match self {
            #(#arms)*
        }
    }
}
