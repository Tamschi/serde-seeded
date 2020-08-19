pub use serde_seeded_proc_macro_definitions::*;

use serde::{de, ser};

pub trait DeSeeder<'de, T> {
	type Seed: de::DeserializeSeed<'de, Value = T>;
	fn seed(self) -> Self::Seed;
}

pub trait SerSeeder<'ser, T, Seeded: 'ser + ser::Serialize> {
	type Seeded: 'ser + ser::Serialize;
	fn seeded(self, value: &'ser T) -> Seeded;
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
	pub fn new<T, S>(f: F) -> Self
	where
		F: Fn(&T) -> S,
		S: ser::Serialize,
	{
		Self(f)
	}
}
impl<'ser, F: Fn(&T) -> S, S: 'ser + ser::Serialize, T> SerSeeder<'ser, T, S> for FnSerSeeder<F> {
	type Seeded = S;
	fn seeded(self, value: &'ser T) -> S {
		self.0(value)
	}
}
