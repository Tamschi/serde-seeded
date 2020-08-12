pub use serde_seeded_proc_macro_definitions::*;

use serde::{de, ser};

pub trait Seeder<'a, T> {
    type Seed: for<'de> de::DeserializeSeed<'de, Value = T>;
    type Seeded: 'a + ser::Serialize;
    fn seed(self) -> Self::Seed;
    fn seeded(self, value: &'a T) -> Self::Seeded;
}

#[doc(hidden)]
pub use serde;
