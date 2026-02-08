use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

pub fn expand(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => expand_struct(&data.fields),
        Data::Enum(data) => expand_enum(name, data),
        Data::Union(_) => {
            return syn::Error::new_spanned(input, "FromJs cannot be derived for unions")
                .to_compile_error();
        }
    };

    // Also generate FromJsArg impl that bridges through FromJs
    let from_js_arg_body = match &input.data {
        Data::Struct(_) | Data::Enum(_) => expand_from_js_arg(),
        Data::Union(_) => TokenStream::new(),
    };

    quote! {
        impl<'rt> #impl_generics rusty_hermes::FromJs<'rt> for #name #ty_generics #where_clause {
            fn from_js(rt: &'rt rusty_hermes::Runtime, value: &rusty_hermes::Value<'rt>) -> rusty_hermes::Result<Self> {
                #body
            }
        }

        impl #impl_generics rusty_hermes::__private::FromJsArg for #name #ty_generics #where_clause {
            unsafe fn from_arg(
                rt: *mut libhermes_sys::HermesRt,
                raw: &libhermes_sys::HermesValue,
            ) -> rusty_hermes::Result<Self> {
                #from_js_arg_body
            }
        }
    }
}

fn expand_from_js_arg() -> TokenStream {
    quote! {
        let value = unsafe { rusty_hermes::Value::from_raw_clone(rt, raw) };
        let rt_ref = unsafe { rusty_hermes::Runtime::borrow_raw(rt) };
        rusty_hermes::FromJs::from_js(&rt_ref, &value)
    }
}

fn expand_struct(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(named) => {
            let field_inits: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let key = ident.to_string();
                    quote! {
                        #ident: rusty_hermes::FromJs::from_js(rt, &obj.get(#key)?)?,
                    }
                })
                .collect();
            quote! {
                let obj = value.duplicate().into_object()?;
                Ok(Self {
                    #(#field_inits)*
                })
            }
        }
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() == 1 {
                // Newtype struct: transparent
                quote! {
                    Ok(Self(rusty_hermes::FromJs::from_js(rt, value)?))
                }
            } else {
                // Tuple struct: array
                let field_inits: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|i| {
                        quote! {
                            rusty_hermes::FromJs::from_js(rt, &arr.get(#i)?)?,
                        }
                    })
                    .collect();
                quote! {
                    let arr = value.duplicate().into_array()?;
                    Ok(Self(#(#field_inits)*))
                }
            }
        }
        Fields::Unit => {
            quote! { Ok(Self) }
        }
    }
}

fn expand_enum(name: &syn::Ident, data: &syn::DataEnum) -> TokenStream {
    let _ = name;

    // Collect unit variants for string matching
    let unit_arms: Vec<_> = data
        .variants
        .iter()
        .filter(|v| matches!(v.fields, Fields::Unit))
        .map(|v| {
            let vname = &v.ident;
            let vname_str = vname.to_string();
            quote! {
                #vname_str => Ok(Self::#vname),
            }
        })
        .collect();

    // Collect non-unit variants for object matching
    let object_arms: Vec<_> =
        data.variants
            .iter()
            .filter(|v| !matches!(v.fields, Fields::Unit))
            .map(|v| {
                let vname = &v.ident;
                let vname_str = vname.to_string();
                match &v.fields {
                    Fields::Named(named) => {
                        let field_inits: Vec<_> = named.named.iter().map(|f| {
                        let ident = f.ident.as_ref().unwrap();
                        let key = ident.to_string();
                        quote! {
                            #ident: rusty_hermes::FromJs::from_js(rt, &inner_obj.get(#key)?)?,
                        }
                    }).collect();
                        quote! {
                            #vname_str => {
                                let inner_obj = payload.into_object()?;
                                Ok(Self::#vname { #(#field_inits)* })
                            }
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        if unnamed.unnamed.len() == 1 {
                            quote! {
                                #vname_str => {
                                    Ok(Self::#vname(rusty_hermes::FromJs::from_js(rt, &payload)?))
                                }
                            }
                        } else {
                            let field_inits: Vec<_> = (0..unnamed.unnamed.len())
                                .map(|i| {
                                    quote! {
                                        rusty_hermes::FromJs::from_js(rt, &arr.get(#i)?)?,
                                    }
                                })
                                .collect();
                            quote! {
                                #vname_str => {
                                    let arr = payload.into_array()?;
                                    Ok(Self::#vname(#(#field_inits)*))
                                }
                            }
                        }
                    }
                    Fields::Unit => unreachable!(),
                }
            })
            .collect();

    let has_unit = !unit_arms.is_empty();
    let has_object = !object_arms.is_empty();

    let string_branch = if has_unit {
        quote! {
            rusty_hermes::ValueKind::String => {
                let s = value.duplicate().into_string()?.to_rust_string()?;
                match s.as_str() {
                    #(#unit_arms)*
                    other => Err(rusty_hermes::Error::RuntimeError(
                        format!("unknown variant: {}", other)
                    )),
                }
            }
        }
    } else {
        TokenStream::new()
    };

    let object_branch = if has_object {
        quote! {
            rusty_hermes::ValueKind::Object => {
                let obj = value.duplicate().into_object()?;
                let names = obj.property_names()?;
                if names.len() != 1 {
                    return Err(rusty_hermes::Error::RuntimeError(
                        format!("expected object with exactly 1 key for enum, got {}", names.len())
                    ));
                }
                let variant_name = names.get(0)?.into_string()?.to_rust_string()?;
                let payload = obj.get(&variant_name)?;
                match variant_name.as_str() {
                    #(#object_arms)*
                    other => Err(rusty_hermes::Error::RuntimeError(
                        format!("unknown variant: {}", other)
                    )),
                }
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        match value.kind() {
            #string_branch
            #object_branch
            _ => Err(rusty_hermes::Error::TypeError {
                expected: "string or object (enum)",
                got: value.kind().name(),
            }),
        }
    }
}
