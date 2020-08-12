use crate::serde_seeded;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let serde_seeded = serde_seeded();
    Ok(quote! {
        #[automatically_derived]
        impl #name {
            pub fn seed<'de>() -> impl #serde_seeded::serde::de::DeserializeSeed<'de, Value = Self> {
                use #serde_seeded::serde::de;

                struct Seed();
                impl<'de> de::DeserializeSeed<'de> for Seed {
                    type Value = #name;
                    fn deserialize<D: de::Deserializer<'de>>(self, _: D) -> ::std::result::Result<Self::Value, D::Error> {
                        todo!("seed()")
                    }
                }

                Seed()
            }
        }
    })
}
