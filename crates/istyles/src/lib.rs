use proc_macro::TokenStream;
use quote::quote;
use serde_json::Value;
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn istyles(input: TokenStream) -> TokenStream {
    let (css_module_map, mod_name) = parse_macro_input!(input with parse_input);

    let value: Value = match serde_json::from_str(&css_module_map) {
        Ok(v) => v,
        Err(e) => {
            return syn::Error::new_spanned(css_module_map, format!("Invalid JSON: {}", e))
                .to_compile_error()
                .into();
        }
    };

    let consts = generate_consts(&value);
    let expanded = quote! {
        pub mod #mod_name {
            #consts
        }
    };

    TokenStream::from(expanded)
}

fn parse_input(input: syn::parse::ParseStream) -> syn::Result<(String, syn::Ident)> {
    let mod_name: syn::Ident = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let path: LitStr = input.parse()?;
    let path = path.value();
    let json = std::fs::read_to_string(&path)
        .map_err(|e| syn::Error::new_spanned(&path, format!("Failed to read JSON: {:?}", e)))?;
    Ok((json, mod_name))
}

fn generate_consts(value: &Value) -> proc_macro2::TokenStream {
    match value {
        Value::Object(map) => {
            let const_decls = map.iter().map(|(k, v)| {
                let key = k.replace('-', "_");
                let key_ident = syn::Ident::new(&key, proc_macro2::Span::call_site());
                let value_str = v.as_str().unwrap_or_default();

                quote! {
                    pub const #key_ident: &str = #value_str;
                }
            });
            quote! { #(#const_decls)* }
        }
        _ => quote! {},
    }
}
