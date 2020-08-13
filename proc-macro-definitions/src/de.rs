use crate::serde_seeded;
use call2_for_syn::call2;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::borrow::Cow;
use syn::{parenthesized, spanned::Spanned as _, Data, DeriveInput, Error, Ident};

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let serde_seeded = serde_seeded();
    match &input.data {
        Data::Struct(data) => {
            let mut field_idents = vec![];
            let mut nexts = vec![];
            let mut errors = vec![];
            for (i, field) in data.fields.iter().enumerate() {
                field_idents.push(
                    field
                        .ident
                        .as_ref()
                        .map(Cow::Borrowed)
                        .unwrap_or_else(|| Cow::Owned(Ident::new(&i.to_string(), field.ty.span()))),
                );

                let mut attrs: Vec<_> = field
                    .attrs
                    .iter()
                    .filter(|a| a.path.is_ident("seeded"))
                    .collect();

                errors.push(if attrs.len() > 1 {
                    let mut attrs = attrs.split_off(1).into_iter().map(|a| {
                        Error::new_spanned(a, "Repeated #[seeded] attribute on the same field")
                    });
                    let mut first = attrs.next().unwrap();
                    for next in attrs {
                        first.combine(next);
                    }
                    Some(first.to_compile_error())
                } else {
                    None
                });

                let attr = attrs.drain(..).next();
                assert_eq!(attrs.into_iter().count(), 0);

                nexts.push(if let Some(attr) = attr {
                    if attr.tokens.is_empty() {
                        let ty = &field.ty;
                        quote_spanned!(attr.path.span()=> next_element_seed(#ty::seed()))
                    } else {
                        let tokens = &attr.tokens;

                        call2(quote!(#tokens), |tokens| {
                            let content;
                            let paren = parenthesized!(content in tokens);
                            Ok((paren, content.parse()?))
                        })
                        .map_or_else(
                            |error| error.to_compile_error(),
                            |(paren, custom_seeder): (_, TokenStream)| {
                                    quote_spanned!(paren.span=> next_element_seed(#custom_seeder.seed()))
                            },
                        )
                    }
                } else {
                    quote_spanned!(field.ty.span()=> next_element())
                });
            }
            let len = field_idents.len();

            Ok(quote! {
                #[automatically_derived]
                impl #name {
                    pub fn seed<'de>() -> impl #serde_seeded::serde::de::DeserializeSeed<'de, Value = Self> {
                        use #serde_seeded::{
                            Seeder,
                            serde::de,
                        };

                        struct Seed();
                        impl<'de> de::DeserializeSeed<'de> for Seed {
                            type Value = #name;
                            fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> ::std::result::Result<Self::Value, D::Error> {
                                struct Visitor;
                                impl<'de> de::Visitor<'de> for Visitor {
                                    type Value = #name;
                                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                                        write!(f, stringify!(#name))
                                    }

                                    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                                        Ok(#name {
                                            #(#field_idents: seq.#nexts?.ok_or_else(|| de::Error::invalid_length(0, &self))?,)*
                                        })
                                    }
                                }

                                const FIELD_NAMES: [&'static str; #len] = [#(stringify!(#field_idents), )*];
                                deserializer.deserialize_struct(stringify!(#name), FIELD_NAMES.as_ref(), Visitor)
                            }
                        }

                        Seed()
                    }
                }
            })
        }
        Data::Enum(_) => todo!("Data::Enum"),
        Data::Union(_) => todo!("Data::Union"),
    }
}
