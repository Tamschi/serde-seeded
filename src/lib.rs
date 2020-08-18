pub use serde_seeded_proc_macro_definitions::*;

use erased_serde as eser;
use serde::de;

pub trait DeSeeder<'de, T> {
	type Seed: de::DeserializeSeed<'de, Value = T>;
	fn seed(self) -> Self::Seed;
}

pub trait SerSeeder<T> {
	// This would be nicer with a generic associated type, but that being ready seems a while off.
	fn seeded<'s>(&'s self, value: &'s T) -> Seeded<'s>;
}
pub type Seeded<'s> = Box<dyn 's + eser::Serialize>;

impl<S: SerSeeder<T>, T> SerSeeder<T> for &S {
	fn seeded<'s>(&'s self, value: &'s T) -> Seeded<'s> {
		S::seeded(self, value)
	}
}

#[doc(hidden)]
pub use {erased_serde, log, serde};

#[derive(Debug, Copy, Clone)]
pub struct FnDeSeeder<F>(pub F);
impl<'de, Seed: de::DeserializeSeed<'de>, F: Fn() -> Seed> DeSeeder<'de, Seed::Value>
	for FnDeSeeder<F>
{
	type Seed = Seed;
	fn seed(self) -> Self::Seed {
		self.0()
	}
}

#[derive(Debug, Copy, Clone)]
pub struct FnSerSeeder<F>(pub F);
/// The struct constructor doesn't always coerce closures correctly, but this here does.
impl<F> FnSerSeeder<F> {
	pub fn new<T>(f: F) -> Self
	where
		F: for<'a> Fn(&'a T) -> Seeded<'a>,
	{
		Self(f)
	}
}
impl<F: for<'a> Fn(&'a T) -> Seeded<'a>, T> SerSeeder<T> for FnSerSeeder<F> {
	fn seeded<'s>(&'s self, value: &'s T) -> Box<dyn 's + eser::Serialize> {
		self.0(value)
	}
}
