pub use serde_seeded_proc_macro_definitions::*;

use {
    fn_t::Function,
    serde::{de, ser},
};

pub trait DeSeeder<T> {
    type Seed: for<'de> de::DeserializeSeed<'de, Value = T>;
    fn seed(self) -> Self::Seed;
}

pub trait SerSeeder<'seeder, 'value, T> {
    type Seeded: 'value + ser::Serialize;
    fn seeded(&'seeder self, value: &'value T) -> Self::Seeded;
}

#[doc(hidden)]
pub use serde;

#[derive(Debug, Copy, Clone)]
pub struct FunctionDeSeeder<F: Function>(pub F);
impl<F: Function<Args = ()>, T> DeSeeder<T> for FunctionSerSeeder<F>
where
    F::Output: for<'de> de::DeserializeSeed<'de, Value = T>,
{
    type Seed = F::Output;
    fn seed(self) -> Self::Seed {
        self.0.call(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FunctionSerSeeder<F: Function>(pub F);
impl<'seeder: 'value, 'value, F: Function<Args = (&'value T,)>, T: 'value>
    SerSeeder<'seeder, 'value, T> for FunctionSerSeeder<F>
where
    F::Output: 'seeder + ser::Serialize,
{
    type Seeded = F::Output;
    fn seeded(&'seeder self, value: &'value T) -> Self::Seeded {
        self.0.call((value,))
    }
}
