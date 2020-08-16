use crate::{serde_seeded, shared::ensure_second};
use call2_for_syn::call2;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::borrow::Cow;
use syn::{
	parenthesized, parse2,
	punctuated::{Pair, Punctuated},
	spanned::Spanned as _,
	Data, DeriveInput, Error, FnArg, GenericParam, Generics, Ident, Lifetime, PatType, Token,
};
use wyz::{Pipe as _, TapOption as _};

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
	let name = &input.ident;
	let serde_seeded = serde_seeded();
	let mut errors = vec![];

	// let type_generics_lifetimes = input.generics.lifetimes().collect::<Vec<_>>();
	// let type_generics_lifetime_lifetimes = type_generics_lifetimes
	// 	.iter()
	// 	.map(|l| &l.lifetime)
	// 	.collect::<Vec<_>>();
	let type_generics_types = input.generics.type_params().collect::<Vec<_>>();
	let type_generics_type_idents = type_generics_types
		.iter()
		.map(|t| &t.ident)
		.collect::<Vec<_>>();
	// let type_generics_consts = input.generics.const_params().collect::<Vec<_>>();
	// let type_generics_const_idents = type_generics_consts
	// 	.iter()
	// 	.map(|c| &c.ident)
	// 	.collect::<Vec<_>>();
	let type_generics_where = &input.generics.where_clause;

	let fn_generics = input
		.attrs
		.iter()
		.filter(|a| a.path.is_ident("seed_generics") || a.path.is_ident("seed_generics_de"))
		.filter_map(|a| {
			call2(a.tokens.clone(), |input| {
				let args;
				let parens = parenthesized!(args in input);
				let args: TokenStream = args.parse()?;
				parse2::<Generics>(quote_spanned!(parens.span=> <#args>))
			})
			.map_err(|e: syn::Error| errors.push(e.to_compile_error()))
			.ok()
		})
		.collect::<Vec<_>>();
	let fn_generics_lifetimes = fn_generics
		.iter()
		.flat_map(|g| g.lifetimes())
		.collect::<Vec<_>>();
	let fn_generics_lifetime_lifetimes = fn_generics_lifetimes
		.iter()
		.map(|l| &l.lifetime)
		.collect::<Vec<_>>();
	let fn_generics_types = fn_generics
		.iter()
		.flat_map(|g| g.type_params())
		.collect::<Vec<_>>();
	let fn_generics_type_idents = fn_generics_types
		.iter()
		.map(|t| &t.ident)
		.collect::<Vec<_>>();
	// fn_generics_consts

	let mut default_de = vec![Lifetime::new("'de", Span::mixed_site())];
	let de = fn_generics_lifetime_lifetimes
		.iter()
		.copied()
		.find(|l| l.ident == "de")
		.tap_some(|_| default_de.pop().unwrap())
		.unwrap_or_else(|| default_de.first().unwrap());

	let args = input
		.attrs
		.iter()
		.filter(|a| a.path.is_ident("seed_args") || a.path.is_ident("seed_args_de"))
		.filter_map(|a| {
			call2(a.tokens.clone(), |input| {
				let args;
				parenthesized!(args in input);
				let args = Punctuated::<FnArg, Token![,]>::parse_terminated(&args)?
					.into_pairs()
					.map(Pair::into_value);
				Ok(args)
			})
			.map_err(|e| errors.push(e.to_compile_error()))
			.ok()
		})
		.flatten()
		.collect::<Vec<_>>();

	let arg_names = args
		.iter()
		.map(|arg| match arg {
			FnArg::Receiver(r) => {
				Error::new_spanned(r, "self-parameters are not supported in this position")
					.to_compile_error()
			}
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

			Ok(quote_spanned! {Span::mixed_site()=>
				#(#errors)*
				#[automatically_derived]
				impl<
					#(#type_generics_types)*
				> #name<
					#(#type_generics_type_idents,)*
				> #type_generics_where {
					pub fn seed<
						#(#default_de,)*
						#(#fn_generics_lifetimes,)*
						#(#fn_generics_types,)*
					>(#(#args),*) -> impl #serde_seeded::serde::de::DeserializeSeed<#de, Value = Self> {
						use #serde_seeded::{
							DeSeeder as _,
							SerSeeder as _,
							serde::de,
						};

						struct Seed<
							#(#fn_generics_lifetimes,)*
							#(#type_generics_types,)*
							#(#fn_generics_types,)*
							> {#(#args),*};
						impl<
							#(#default_de,)*
							#(#fn_generics_lifetimes,)*
							#(#type_generics_types,)*
							#(#fn_generics_types,)*
							> de::DeserializeSeed<#de> for Seed<
								#(#fn_generics_lifetime_lifetimes,)*
								#(#type_generics_type_idents,)*
								#(#fn_generics_type_idents,)*
							> #type_generics_where {
							type Value = #name<#(#type_generics_type_idents)*>;
							fn deserialize<D: de::Deserializer<#de>>(self, deserializer: D) -> ::std::result::Result<Self::Value, D::Error> {
								struct Visitor<
									#(#fn_generics_lifetimes,)*
									#(#type_generics_types,)*
									#(#fn_generics_types,)*
								> {#(#args),*};
								impl<
									#(#default_de,)*
									#(#fn_generics_lifetimes,)*
									#(#type_generics_types,)*
									#(#fn_generics_types,)*
								> de::Visitor<#de> for Visitor<
									#(#fn_generics_lifetime_lifetimes,)*
									#(#type_generics_type_idents,)*
									#(#fn_generics_type_idents,)*
								> #type_generics_where {
									type Value = #name<
										#(#type_generics_type_idents,)*
									>;
									fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
										write!(f, stringify!(#name))
									}

									fn visit_seq<A: de::SeqAccess<#de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
										let Self {#(#arg_names),*} = self;

										#(let #field_idents = seq.#nexts?.ok_or_else(|| de::Error::invalid_length(0, &self))?;)*

										Ok(#name {
											#(#field_idents,)*
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
