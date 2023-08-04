// impl Allocator
pub mod allocator;

// types
pub mod types;

pub use types::{AllocError, Allocator, FreeBlock, MemRange, ProcBuilder, Process, Result};
