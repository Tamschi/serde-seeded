use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use crate::serde_seeded;

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let serde_seeded = serde_seeded();
    Ok(quote! {
        #[automatically_derived]
        impl #name {
            pub fn seeded(&self) -> impl #serde_seeded::serde::Serialize {
                todo!()
            }
        }
    })
}
