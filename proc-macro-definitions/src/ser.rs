use proc_macro2::Span;
use {
    crate::serde_seeded,
    call2_for_syn::call2,
    proc_macro2::TokenStream,
    quote::{quote, quote_spanned, ToTokens as _},
    std::borrow::Cow,
    syn::{
        parenthesized,
        punctuated::{Pair, Punctuated},
        spanned::Spanned as _,
        Data, DeriveInput, Error, FnArg, Ident, PatType, Token,
    },
};

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let serde_seeded = serde_seeded();
    let mut errors = vec![];

    let args = input
        .attrs
        .iter()
        .filter(|a| a.path.is_ident("seed_args") || a.path.is_ident("seed_args_ser"))
        .map(|a| {
            call2(a.tokens.clone(), |input| {
                let args;
                parenthesized!(args in input);
                let args = Punctuated::<FnArg, Token![,]>::parse_terminated(&args)?
                    .into_pairs()
                    .map(|pair| match pair {
                        punctuated @ Pair::Punctuated(_, _) => punctuated,
                        Pair::End(arg) => {
                            let comma = Token![,](arg.span());
                            Pair::Punctuated(arg, comma)
                        }
                    });
                Ok(args)
            })
        })
        .filter_map(|r| r.map_err(|e| errors.push(e.to_compile_error())).ok())
        .flatten()
        .collect::<Vec<_>>();

    let arg_names = args
        .iter()
        .map(|arg| match arg.clone().into_value() {
            FnArg::Receiver(r) => r.self_token.into_token_stream(),
            FnArg::Typed(PatType { pat, .. }) => pat.into_token_stream(),
        })
        .collect::<Vec<_>>();

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

                if attrs.len() > 1 {
                    let mut attrs = attrs.split_off(1).into_iter().map(|a| {
                        Error::new_spanned(
                            a,
                            "Multiple #[seeded] or #[seeded_ser] attributes on the same field",
                        )
                    });
                    let mut first = attrs.next().unwrap();
                    for next in attrs {
                        first.combine(next);
                    }
                    errors.push(first.to_compile_error())
                }

                let attr = attrs.drain(..).next();
                assert_eq!(attrs.into_iter().count(), 0);

                let mut serialize =
                    quote_spanned!(ident.span().resolved_at(Span::mixed_site())=> &this.#ident);
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
                            |error| errors.push(error.to_compile_error()),
                            |(paren, custom_seeder): (_, TokenStream)| {
                                serialize =
                                    quote_spanned!(paren.span=> &#custom_seeder.seeded(#serialize))
                            },
                        );
                    }
                }

                serialize_fields.push(
                    quote_spanned! {ident.span().resolved_at(Span::mixed_site())=>
                        serialize_struct.serialize_field(stringify!(#ident), #serialize)?;
                    },
                )
            }

            Ok(quote_spanned! {Span::mixed_site()=>
                #(#errors)*
                #[automatically_derived]
                impl #name {
                    pub fn seeded<'a>(&'a self, #(#args)*) -> impl 'a + #serde_seeded::serde::Serialize {
                        use #serde_seeded::{
                            DeSeeder as _,
                            SerSeeder as _,
                            serde::{
                                ser::{self, SerializeStruct as _},
                                export::Result,
                            },
                        };

                        struct Seeded<'a>{
                            this: &'a #name,
                            #(#args)*
                        };
                        impl<'a> ser::Serialize for Seeded<'a> {
                            fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                                let mut serialize_struct = serializer.serialize_struct(stringify!(#name), #field_count)?;
                                let Seeded {
                                    this,
                                    #(#arg_names,)*
                                } = self;

                                #(#serialize_fields)*

                                serialize_struct.end()
                            }
                        }
                        Seeded {
                            this: self,
                            #(#arg_names,)*
                        }
                    }
                }
            })
        }
        Data::Enum(_) => todo!("Data::Enum"),
        Data::Union(_) => todo!("Data::Union"),
    }
}
