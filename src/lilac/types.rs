use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use xorshift::{thread_rng, Rng, Xorshift1024};

#[derive(Debug, Clone)]
pub enum AllocError {
    AlreadyRegistered,
    NoSuchProcess,
    NotOwned,
    BlockNotFound,
}

impl std::error::Error for AllocError {}

pub type Result<T> = std::result::Result<T, AllocError>;

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AllocError::AlreadyRegistered => write!(f, "this process id is already registered"),
            AllocError::NoSuchProcess => write!(f, "the given process id does not exist"),
            AllocError::NotOwned => write!(f, "the memory range is not owned by the process"),
            AllocError::BlockNotFound => write!(
                f,
                "the block at the given start address was not found for this process"
            ),
        }
    }
}

#[derive(Debug)]
pub enum FreeBlock {
    Free(u32),
    FreeMerge(u32),
    RefcountDecreased,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Process(u32);

#[derive(Copy, Clone)]
pub struct ProcBuilder {
    xorshift: Xorshift1024,
    counter: u32,
}

impl ProcBuilder {
    pub fn new() -> Self {
        Self {
            xorshift: thread_rng(),
            counter: 0,
        }
    }

    pub fn count(&mut self) -> Process {
        self.counter += 1;
        Process(self.counter - 1)
    }

    pub fn xorshift(&mut self) -> Process {
        let mut num: [u8; 4] = [0; 4];
        self.xorshift.fill_bytes(&mut num);
        Process(u32::from_le_bytes(num))
    }
}

impl Default for ProcBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MemRange {
    pub(super) refcount: Arc<AtomicU32>,
    pub(super) range: Range<u32>,
}

impl MemRange {
    pub fn new(refcount: Arc<AtomicU32>, range: Range<u32>) -> Self {
        Self { refcount, range }
    }
}

#[derive(Debug)]
pub struct Allocator {
    pub(super) heap: Vec<u8>,
    // hashmap<pid, vec<(refcount, range)>>
    pub(super) allocated: HashMap<Process, Vec<MemRange>>,
    // (size, range)
    pub(super) free: Vec<(u32, Range<u32>)>,
}

impl Default for Allocator {
    fn default() -> Self {
        Self::new()
    }
}
