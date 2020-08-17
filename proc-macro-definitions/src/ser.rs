use crate::serde_seeded;
use call2_for_syn::call2;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens as _};
use std::borrow::Cow;
use syn::{
	parenthesized, parse2,
	punctuated::{Pair, Punctuated},
	spanned::Spanned as _,
	Data, DeriveInput, Error, FnArg, GenericParam, Generics, Ident, Lifetime, PatType, Token,
};
use wyz::TapOption;

pub fn expand_derive(input: &DeriveInput) -> syn::Result<TokenStream> {
	let name = &input.ident;
	let serde_seeded = serde_seeded();
	let mut errors = vec![];

	let mut type_generics_lifetimes = vec![];
	let mut type_generics_types = vec![];
	for generic in input.generics.params.iter() {
		match generic{
		    syn::GenericParam::Type(ty) => type_generics_types.push(ty),
		    syn::GenericParam::Lifetime(l) => type_generics_lifetimes.push(l),
		    syn::GenericParam::Const(c) => {errors.push(Error::new_spanned(c, "serde-seeded::seeded: Const parameters are currently not supported here. You can request or help out with implementation at <https://github.com/Tamschi/serde-seeded/issues/2>.").to_compile_error())}
		}
	}

	let type_generics_lifetime_lifetimes = type_generics_lifetimes
		.iter()
		.map(|l| &l.lifetime)
		.collect::<Vec<_>>();
	let type_generics_type_idents = type_generics_types
		.iter()
		.map(|t| &t.ident)
		.collect::<Vec<_>>();
	let type_generics_where = &input.generics.where_clause;

	let fn_generics = input
		.attrs
		.iter()
		.filter(|a| a.path.is_ident("seed_generics") || a.path.is_ident("seed_generics_ser"))
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

	let mut fn_generics_lifetimes = vec![];
	let mut fn_generics_types = vec![];
	for generic in fn_generics.iter().flat_map(|g| g.params.iter()) {
		match generic {
				GenericParam::Type(ty) => fn_generics_types.push(ty),
				GenericParam::Lifetime(l) => fn_generics_lifetimes.push(l),
				GenericParam::Const(c) => {errors.push(Error::new_spanned(c, "serde-seeded::seed: Const parameters are currently not supported here. You can request or help out with implementation at <https://github.com/Tamschi/serde-seeded/issues/3>.").to_compile_error())}
			}
	}

	let fn_generics_lifetime_lifetimes = fn_generics_lifetimes
		.iter()
		.map(|l| &l.lifetime)
		.collect::<Vec<_>>();
	let fn_generics_type_idents = fn_generics_types
		.iter()
		.map(|t| &t.ident)
		.collect::<Vec<_>>();
	// Where clauses on derived functions are missing too but don't have a specific error since there's no syntax to specify them yet. The GitHub issue is <https://github.com/Tamschi/serde-seeded/issues/4>.

	let mut default_ser = vec![Lifetime::new("'ser", Span::mixed_site())];
	let ser = fn_generics_lifetime_lifetimes
		.iter()
		.copied()
		.find(|l| l.ident == "ser")
		.tap_some(|_| default_ser.pop().unwrap())
		.unwrap_or_else(|| default_ser.first().unwrap());

	let args = input
		.attrs
		.iter()
		.filter(|a| a.path.is_ident("seed_args") || a.path.is_ident("seed_args_ser"))
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
			let field_count = data.fields.len();

			let mut field_idents = vec![];
			let mut serialize_fields = vec![];
			for (i, field) in data.fields.iter().enumerate() {
				let ident = field
					.ident
					.as_ref()
					.map(Cow::Borrowed)
					.unwrap_or_else(|| Cow::Owned(Ident::new(&i.to_string(), field.ty.span())));

				field_idents.push(ident.clone());

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

				let mut serialize =ident.to_token_stream();
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
									quote_spanned!(paren.span.resolved_at(Span::mixed_site())=> { // <-- No-field-shadowing!-brace.
										&#custom_seeder.seeded(#serialize)
									})
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
				impl<
					#(#type_generics_lifetimes,)*
				> #name<
					#(#type_generics_lifetime_lifetimes,)*
				> {
					pub fn seeded<#ser>(&#ser self, #(#args,)*) -> impl #ser + #serde_seeded::serde::Serialize {
						use #serde_seeded::{
							DeSeeder as _,
							SerSeeder as _,
							serde::{
								ser::{self, SerializeStruct as _},
								export::Result,
							},
						};

						struct Seeded<
							#ser,
							#(#type_generics_lifetimes,)*
						>{
							this: &#ser #name<
								#(#type_generics_lifetime_lifetimes,)*
							>,
							#(#args,)*
						};
						impl<
							#ser,
							#(#type_generics_lifetimes,)*
						> ser::Serialize for Seeded<
							#ser,
							#(#type_generics_lifetime_lifetimes,)*
						> {
							fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
								let mut serialize_struct = serializer.serialize_struct(stringify!(#name), #field_count)?;
								let Seeded {
									this: #name {
										#(#field_idents,)*
									},
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
		Data::Enum(e) => Err(Error::new_spanned(e.enum_token, "serde-seeded derive macros are not available on enums yet. You can request this feature at <https://github.com/Tamschi/serde-seeded/issues/5>.")),
		Data::Union(u) => Err(Error::new_spanned(u.union_token, "serde-seeded derive macros are not available on unions yet. You can request this feature at <https://github.com/Tamschi/serde-seeded/issues/6>.")),
	}
}
