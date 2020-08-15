use proc_macro2::Span;
use std::borrow::Cow;
use syn::Ident;
use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, DeriveInput},
};

mod de;
mod ser;

#[proc_macro_derive(seeded, attributes(seeded, seeded_ser, seed_args, seed_args_ser))]
pub fn seeded(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    ser::expand_derive(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(seed, attributes(seeded, seeded_de, seed_args, seed_args_de))]
pub fn seed(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    de::expand_derive(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn serde_seeded() -> proc_macro2::TokenStream {
    let name = proc_macro_crate::crate_name("serde-seeded")
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed("serde_seeded"));
    let ident = Ident::new(&name, Span::call_site());
    quote!(::#ident)
}
