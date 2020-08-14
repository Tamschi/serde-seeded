use {
    crate::serde_seeded,
    call2_for_syn::call2,
    proc_macro2::TokenStream,
    quote::{quote, quote_spanned},
    std::borrow::Cow,
    syn::{parenthesized, spanned::Spanned as _, Data, DeriveInput, Error, Ident},
};

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let serde_seeded = serde_seeded();
    match &input.data {
        Data::Struct(data) => {
            let field_count = data.fields.len();

            let mut serialize_fields = vec![];
            for (i, field) in data.fields.iter().enumerate() {
                let ident = field
                    .ident
                    .as_ref()
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| Cow::Owned(Ident::new(&i.to_string(), field.ty.span())));

                let mut attrs: Vec<_> = field
                    .attrs
                    .iter()
                    .filter(|a| a.path.is_ident("seeded") || a.path.is_ident("seeded_ser"))
                    .collect();

                let mut errors = if attrs.len() > 1 {
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
                };

                let attr = attrs.drain(..).next();
                assert_eq!(attrs.into_iter().count(), 0);

                let mut serialize = quote_spanned!(ident.span()=> &self.0 .#ident);
                if let Some(attr) = attr {
                    if attr.tokens.is_empty() {
                        serialize = quote_spanned!(attr.path.span()=> #serialize.seeded());
                    } else {
                        let tokens = &attr.tokens;

                        call2(quote!(#tokens), |tokens| {
                            let content;
                            let paren = parenthesized!(content in tokens);
                            Ok((paren, content.parse()?))
                        })
                        .map_or_else(
                            |error| {
                                let error = error.to_compile_error();
                                errors = quote!(#errors #error).into()
                            },
                            |(paren, custom_seeder): (_, TokenStream)| {
                                serialize =
                                    quote_spanned!(paren.span=> &#custom_seeder.seeded(#serialize))
                            },
                        );
                    }
                }

                serialize_fields.push(quote_spanned! {ident.span()=>
                    serialize_struct.serialize_field(stringify!(#ident), #serialize)?;
                    #errors
                })
            }

            Ok(quote! {
                #[automatically_derived]
                impl #name {
                    pub fn seeded<'a>(&'a self) -> impl 'a + #serde_seeded::serde::Serialize {
                        use #serde_seeded::{
                            DeSeeder as _,
                            SerSeeder as _,
                            serde::{
                                ser::{self, SerializeStruct as _},
                                export::Result,
                            },
                        };

                        struct Seeded<'a>(&'a #name);
                        impl<'a> ser::Serialize for Seeded<'a> {
                            fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                                let mut serialize_struct = serializer.serialize_struct(stringify!(#name), #field_count)?;

                                #(#serialize_fields)*

                                serialize_struct.end()
                            }
                        }
                        Seeded(self)
                    }
                }
            })
        }
        Data::Enum(_) => todo!("Data::Enum"),
        Data::Union(_) => todo!("Data::Union"),
    }
}
