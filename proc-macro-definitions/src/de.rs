use {
    crate::serde_seeded,
    call2_for_syn::call2,
    proc_macro2::TokenStream,
    quote::{quote, quote_spanned, ToTokens},
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
        .filter(|a| a.path.is_ident("seed_args") || a.path.is_ident("seed_args_de"))
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
            let mut field_idents = vec![];
            let mut nexts = vec![];

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
                    .filter(|a| a.path.is_ident("seeded") || a.path.is_ident("seeded_de"))
                    .collect();

                if attrs.len() > 1 {
                    let mut attrs = attrs.split_off(1).into_iter().map(|a| {
                        Error::new_spanned(
                            a,
                            "Multiple #[seeded] or #[seeded_de] attributes on the same field",
                        )
                    });
                    let mut first = attrs.next().unwrap();
                    for next in attrs {
                        first.combine(next);
                    }
                    errors.push(first.to_compile_error())
                }

                errors.extend(
                    field.attrs.iter()
                        .filter(|a| a.path.is_ident("seed_args"))
                        .map(|a| Error::new_spanned(a, "Misplaced #[seeded_args]: This attribute is only valid on the type's definition")
                            .to_compile_error()
                        )
                    );

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
                #(#errors)*
                #[automatically_derived]
                impl #name {
                    pub fn seed<'de>(#(#args)*) -> impl #serde_seeded::serde::de::DeserializeSeed<'de, Value = Self> {
                        use #serde_seeded::{
                            DeSeeder as _,
                            SerSeeder as _,
                            serde::de,
                        };

                        struct Seed {#(#args),*};
                        impl<'de> de::DeserializeSeed<'de> for Seed {
                            type Value = #name;
                            fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> ::std::result::Result<Self::Value, D::Error> {
                                struct Visitor {#(#args),*};
                                impl<'de> de::Visitor<'de> for Visitor {
                                    type Value = #name;
                                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                                        write!(f, stringify!(#name))
                                    }

                                    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                                        let Self {#(#arg_names),*} = self;
                                        Ok(#name {
                                            #(#field_idents: seq.#nexts?.ok_or_else(|| de::Error::invalid_length(0, &self))?,)*
                                        })
                                    }
                                }

                                let Self {#(#arg_names),*} = self;
                                const FIELD_NAMES: [&'static str; #len] = [#(stringify!(#field_idents), )*];
                                deserializer.deserialize_struct(
                                    stringify!(#name),
                                    FIELD_NAMES.as_ref(),
                                    Visitor {#(#arg_names),*},
                                )
                            }
                        }

                        Seed {#(#arg_names),*}
                    }
                }
            })
        }
        Data::Enum(_) => todo!("Data::Enum"),
        Data::Union(_) => todo!("Data::Union"),
    }
}
