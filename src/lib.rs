pub mod lilac;

pub use lilac::Result as LilacResult;
pub use lilac::{AllocError, Allocator, FreeBlock, ProcBuilder, Process};

// <vivyir> for `lilac`:
//
// TODO: program `realloc`, it allocates a new block with the asked size and copies all the data
// from the old block to the new one, if the new one is smaller it'll result in a shrink and won't
// write the data past the limit and just truncate it.

// TODO: program `merge`, it will merge 2 consecutive blocks of allocated memory UNLESS the
// refcounter of one is more than 1, which means that block is shared and if it were to be combined
// it would cause the most MAJOR fuckups of the history in memory allocation, also if you FOR SOME
// GODS FORSAKEN REASON have a shared block between the ones you want to merge just ~~kill
// yourself~~ i mean, just call `realloc` and copy the blocks contiguously.
//
// TODO: write unit tests for all the functions, boring but someone's gotta do it, ugh.
//
// TODO: implement whatever the fuck the below code should be (parallel allocator):
// ```rs
// #[derive(Debug)]
// struct ParallelAlloc(Arc<RwLock<Allocator>>);
//
// impl ParallelAlloc {
//     fn new() -> Self {
//         Self(Arc::new(RwLock::new(Allocator::new())))
//     }
// }
// ```
