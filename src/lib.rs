pub use serde_seeded_proc_macro_definitions::*;

use {
    fn_t::Function,
    serde::{de, ser},
};

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
pub struct FunctionDeSeeder<F>(pub F);
impl<'de, F: Function<Args = ()>, T> DeSeeder<'de, T> for FunctionSerSeeder<F>
where
    F::Output: de::DeserializeSeed<'de, Value = T>,
{
    type Seed = F::Output;
    fn seed(self) -> Self::Seed {
        self.0.call(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FunctionSerSeeder<F>(pub F);
impl<'s: 'a, 'a, F: Function<Args = (&'a T,)>, T: 'a> SerSeeder<'s, T> for FunctionSerSeeder<F>
where
    F::Output: 's + ser::Serialize,
{
    type Seeded = F::Output;
    fn seeded(&'s self, value: &'s T) -> Self::Seeded {
        self.0.call((value,))
    }
}
