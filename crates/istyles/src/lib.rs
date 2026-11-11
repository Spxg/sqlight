use std::{collections::BTreeMap, path::PathBuf};

use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn istyles(input: TokenStream) -> TokenStream {
    let (module, map) = parse_macro_input!(input with parse_input);
    expand(module, map)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn parse_input(input: syn::parse::ParseStream) -> syn::Result<(syn::Ident, LitStr)> {
    let module = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let map = input.parse()?;
    Ok((module, map))
}

fn expand(module: syn::Ident, map: LitStr) -> syn::Result<proc_macro2::TokenStream> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|error| syn::Error::new_spanned(&map, error))?;
    let path = PathBuf::from(manifest_dir).join(map.value());
    let source = std::fs::read_to_string(&path).map_err(|error| {
        syn::Error::new_spanned(
            &map,
            format!(
                "cannot read CSS module map {}: {error}; run `npm run assets` first",
                path.display()
            ),
        )
    })?;
    let classes: BTreeMap<String, String> = serde_json::from_str(&source).map_err(|error| {
        syn::Error::new_spanned(&map, format!("invalid CSS module map: {error}"))
    })?;
    let constants = classes
        .into_iter()
        .map(|(class, value)| {
            let name = syn::parse_str::<syn::Ident>(&class.replace('-', "_")).map_err(|_| {
                syn::Error::new_spanned(
                    &map,
                    format!("CSS class `{class}` cannot be used as a Rust constant"),
                )
            })?;
            Ok(quote! { pub const #name: &str = #value; })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let path = LitStr::new(&path.to_string_lossy(), map.span());

    Ok(quote! {
        #[allow(dead_code, non_upper_case_globals)]
        pub mod #module {
            // Register the map with rustc so a CSS change invalidates this macro.
            const _: &str = include_str!(#path);
            #(#constants)*
        }
    })
}
