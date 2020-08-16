use proc_macro2::Span;
use syn::{punctuated::Pair, spanned::Spanned};

pub fn ensure_second<First: Spanned, Second>(
	pair: Pair<First, Second>,
	second: impl FnOnce(Span) -> Second,
) -> Pair<First, Second> {
	match pair {
		punctuated @ Pair::Punctuated(_, _) => punctuated,
		Pair::End(first) => {
			let second = second(first.span());
			Pair::Punctuated(first, second)
		}
	}
}
