pub use serde_seeded_proc_macro_definitions::*;

use serde::{de, ser};

pub trait DeSeeder<T> {
    type Seed: for<'de> de::DeserializeSeed<'de, Value = T>;
    fn seed(self) -> Self::Seed;
}

pub trait SerSeeder<'a, T> {
    type Seeded: 'a + ser::Serialize;
    fn seeded(&'a self, value: &'a T) -> Self::Seeded;
}

#[doc(hidden)]
pub use serde;
