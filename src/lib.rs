pub use serde_seeded_proc_macro_definitions::*;

use erased_serde as eser;
use serde::{de, ser};
use std::marker::PhantomData;

pub trait DeSeeder<'de, T> {
	type Seed: de::DeserializeSeed<'de, Value = T>;
	fn seed(self) -> Self::Seed;
}

pub trait SerSeeder<T> {
	// This would be nicer with a generic associated type, but that being ready seems a while off.
	fn seeded<'s>(&'s self, value: &'s T) -> Seeded<'s>;
}
pub type Seeded<'s> = Box<dyn 's + eser::Serialize>;

#[doc(hidden)]
pub use {erased_serde, serde};

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
pub struct FnSerSeeder<F, Seeded>(pub F, PhantomData<Seeded>);
impl<F: for<'a> Fn(&'a T) -> Seeded, T, Seeded: ser::Serialize> SerSeeder<T>
	for FnSerSeeder<F, Seeded>
{
	fn seeded<'s>(&'s self, value: &'s T) -> Box<dyn 's + eser::Serialize> {
		Box::new(self.0(value)) as _
	}
}
