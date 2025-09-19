mod atomic;
pub mod hi;
pub mod ho;
pub mod homod;
pub mod li;
mod seeder;

use atomic::DEVStoneAtomic;
pub use hi::HI;
pub use ho::HO;
pub use homod::HOmod;
pub use li::LI;
use seeder::DEVStoneSeeder;
#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[derive(Debug, Default, Copy, Clone)]
pub struct TestProbe {
    n_atomics: usize,
    n_eics: usize,
    n_ics: usize,
    n_eocs: usize,
    n_internals: usize,
    n_externals: usize,
    n_events: usize,
}

#[cfg(test)]
type SharedProbe = Arc<Mutex<TestProbe>>;
