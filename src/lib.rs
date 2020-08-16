pub use serde_seeded_proc_macro_definitions::*;

use serde::{de, ser};

pub trait DeSeeder<'de, T> {
	type Seed: de::DeserializeSeed<'de, Value = T>;
	fn seed(self) -> Self::Seed;
}

pub trait SerSeeder<'s, T> {
	type Seeded: 's + ser::Serialize;
	fn seeded(&'s self, value: &'s T) -> Self::Seeded;
}

#[doc(hidden)]
pub use serde;

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
impl<'s, F: Fn(&'s T) -> Seeded, T: 's, Seeded: 's + ser::Serialize> SerSeeder<'s, T>
	for FnSerSeeder<F>
{
	type Seeded = Seeded;
	fn seeded(&'s self, value: &'s T) -> Self::Seeded {
		self.0(value)
	}
}
