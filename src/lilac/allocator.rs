use std::collections::HashMap;
use std::ops::Range;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use super::{AllocError, Allocator, FreeBlock, MemRange, Process, Result};

impl Allocator {
    /// Create a new `Allocator`.
    pub fn new() -> Self {
        Self {
            heap: vec![],
            allocated: HashMap::new(),
            free: vec![],
        }
    }

    pub fn register_process(&mut self, process_id: Process) -> Result<()> {
        if self.allocated.contains_key(&process_id) {
            return Err(AllocError::AlreadyRegistered);
        }

        if self.allocated.insert(process_id, vec![]).is_none() {
            Ok(())
        } else {
            unreachable!();
        }
    }

    fn alloc_new(&mut self, process_id: Process, size: u32) -> Range<u32> {
        let last_elem = self.heap.len() as u32;
        for _ in 0..size {
            self.heap.push(0);
        }
        let new_last_elem = self.heap.len() as u32;

        let range = last_elem..(new_last_elem - 1);

        let entry = self.allocated.entry(process_id).or_insert(vec![]);
        entry.push(MemRange::new(Arc::new(AtomicU32::new(1)), range));

        last_elem..(new_last_elem - 1)
    }

    fn alloc_free(
        &mut self,
        process_id: Process,
        size: u32,
        free: (u32, Range<u32>),
    ) -> Range<u32> {
        // the start will be the start of the free block, but the end will be the start plus the
        // size but subtracting one, because of how vectors are indexed, for example a 4 element
        // range is 0..3, not 0..4, if we were to not subtract it would treat a 4 element range as
        // 0..4 which is actually 5 elements
        //
        // NOTE: in alloc_new() this was done when initializing the range, however here we do it
        // beforehand.
        let start = free.1.start;
        let end = free.1.start + size - 1;

        let new_cap = free.0 - size;
        if new_cap != 0 {
            // if there is still free memory left that we don't need to allocate, we'll just start
            // from the end of the last used block and declare the rest as free.
            let start_of_rest = end + 1;
            let end_of_rest = free.1.end;

            self.free.push((new_cap, start_of_rest..end_of_rest));
        }

        let range = start..end;
        let entry = self.allocated.entry(process_id).or_insert(vec![]);
        entry.push(MemRange::new(Arc::new(AtomicU32::new(1)), range));

        start..end
    }

    /// Allocates a certain `size` of bytes on the heap of the `Allocator` under a process id; if
    /// there aren't enough free bytes it will add more space on the heap.
    ///
    /// It will return a `Range<u32>` where you can later use the start index of that range as the
    /// value to free this memory later, using the `free()` function.
    ///
    /// This function will error if the process id hasn't been registered before.
    pub fn alloc(&mut self, process_id: Process, size: u32) -> Result<Range<u32>> {
        if !self.allocated.contains_key(&process_id) {
            return Err(AllocError::NoSuchProcess);
        }

        let has_free = self.free.iter().enumerate().find(|x| x.1 .0 >= size);
        if let Some(free) = has_free {
            let free = self.free.swap_remove(free.0);
            Ok(self.alloc_free(process_id, size, free))
        } else {
            Ok(self.alloc_new(process_id, size))
        }
    }

    // this function frees the block if and only if the refcount becomes zero in this free, meaning
    // that it will only remove the memory block from the access list and not put it into the free
    // vector, this means that if a process just holds to a shared memory infinitely it will never
    // free and be a memory leak, very cool!
    fn free_inner(
        &mut self,
        process_id: Process,
        start_idx: u32,
        zeroize: bool,
    ) -> Result<FreeBlock> {
        let allocated = {
            if !self.allocated.contains_key(&process_id) {
                return Err(AllocError::NoSuchProcess);
            }

            self.allocated.entry(process_id).or_insert(vec![])
        };

        let block = allocated
            .iter()
            .enumerate()
            .find(|x| x.1.range.start == start_idx);

        // because of enumerate the index is .0 and the block is .1
        if let Some(block_real) = block {
            let refcount = (*(block_real.1.refcount)).load(Ordering::Relaxed);
            let block_idx = block_real.0;

            // decrease refcount by 1
            (*(allocated[block_idx].refcount)).fetch_sub(1, Ordering::SeqCst);
            let refcount = refcount - 1;

            // remove block from process' access list
            let block = allocated.swap_remove(block_idx);

            // if the refcount became zero (aka this was the last process holding a reference) then
            // move it into the free vec
            if refcount == 0 {
                if zeroize {
                    for i in block.range.start..=block.range.end {
                        self.heap[i as usize] = 0;
                    }
                }

                // add the freed block into the free vec
                let blocklen = block.range.len() as u32 + 1;
                self.free.push((blocklen, block.range));

                // sort the free vec before checking to merge
                self.free.sort_unstable_by(|a, b| a.1.start.cmp(&b.1.start));

                // NOTE: this, somehow in some arcane fucking way, checks all the ranges in this
                // vector to see if they connect (this is possible because we sorted the vector
                // beforehand, the sort was also unstable because our key would NEVER repeat as it
                // is the index of a vector) after checking if they connect it adds the indices to
                // a vector and deduplicates them because in my shitty implementation duplication
                // is a thing.
                let mut last_end = 0;
                let mut indices = vec![];
                for i in self.free.iter().enumerate() {
                    let old_last = last_end;
                    last_end = i.1 .1.start + i.1 .0;

                    if (old_last > 0) && (old_last == i.1 .1.start) {
                        indices.push(i.0 - 1);
                        indices.push(i.0);
                    }
                }
                indices.dedup();

                if !indices.is_empty() {
                    // safe to unwrap because we know indices is NOT empty, and we can do both first()
                    // and last() because we know if indices is NOT empty there are at least 2 elements
                    // because of the last code block which fills indices
                    let start = self.free[*indices.first().unwrap()].1.start;
                    let end = self.free[*indices.last().unwrap()].1.end;
                    let cap = end + 1;

                    // we dont swap remove because it will take the sorted free array and ruin it,
                    // instead we remove and keep the order, we can't use the indices because the array
                    // is shifted, so instead we remove the first index with the count of however many
                    // indices we had (3 works too!)
                    //
                    // example with 3 merged at the same time:
                    // alloc 4 bytes under 0
                    // alloc 4 bytes under 1
                    // alloc 4 bytes under 2
                    //
                    // [0][0][0][0][1][1][1][1][2][2][2][2]
                    //
                    // free 4 bytes under 0
                    // free 4 bytes under 2
                    //
                    // [/][/][/][/][1][1][1][1][/][/][/][/]
                    //
                    // free 4 bytes under 1
                    //
                    // (memory will be merged as they are all contiguous)
                    // [-][-][-][-][-][-][-][-][-][-][-][-]
                    //
                    // alloc 6 bytes under 0
                    //
                    // [0][0][0][0][0][0][-][-][-][-][-][-]
                    //
                    // ---
                    //
                    // i believe 3 is the most amount of contiguous blocks possible that we would have
                    // to merge, as this code is run on every free() call there can never be more than
                    // 3 mergable blocks together at the same time.
                    for _ in 0..indices.len() {
                        self.free.remove(indices[0]);
                    }

                    self.free.push((cap, start..end));

                    return Ok(FreeBlock::FreeMerge(cap));
                }

                Ok(FreeBlock::Free(blocklen))
            } else {
                Ok(FreeBlock::RefcountDecreased)
            }
        } else {
            Err(AllocError::BlockNotFound)
        }
    }

    /// Free a block of memory under a process id (but don't zeroize the underlying memory), this
    /// will need the starting index of the block.
    ///
    /// It errors if it couldn't find the block from the starting index (`AllocError::BlockNotFound`).
    pub fn free(&mut self, process_id: Process, start_idx: u32) -> Result<FreeBlock> {
        self.free_inner(process_id, start_idx, false)
    }

    /// Free a block of memory under a process id (and zeroize the underlying memory), this will
    /// need the starting index of the block.
    ///
    /// It errors if it couldn't find the block from the starting index (`AllocError::BlockNotFound`).
    pub fn free_clear(&mut self, process_id: Process, start_idx: u32) -> Result<FreeBlock> {
        self.free_inner(process_id, start_idx, true)
    }

    /// Immutably borrow a certain range of the heap from a process, the process must have already
    /// allocated memory beforehand and the range specified must also be within the allocated
    /// memory space of the process.
    ///
    /// It errors if the process doesn't exist (`AllocError::NoSuchProcess`) and if the specified
    /// range isn't owned by the process (`AllocError::NotOwned`).
    pub fn range_borrow(&mut self, process_id: Process, range: Range<u32>) -> Result<&[u8]> {
        let allocated = {
            if !self.allocated.contains_key(&process_id) {
                return Err(AllocError::NoSuchProcess);
            }

            self.allocated.entry(process_id).or_insert(vec![])
        };

        if let Some(_found_range) = allocated
            .iter()
            .find(|&x| (x.range.start <= range.start) && (x.range.end >= range.end))
        {
            // as range end is exclusive we have to add 1 to it, because
            // all indexable types start from 0 instead of 1
            Ok(&self.heap[range.start as usize..range.end as usize + 1])
        } else {
            Err(AllocError::NotOwned)
        }
    }

    /// Mutably borrow a certain range of the heap from a process, the process must have already
    /// allocated memory beforehand and the range specified must also be within the allocated
    /// memory space of the process.
    ///
    /// It errors if the process doesn't exist (`AllocError::NoSuchProcess`) and if the specified
    /// range isn't owned by the process (`AllocError::NotOwned`).
    ///
    /// NOTE: The given range **must** be within a single allocated block, be it shared or owned.
    /// If you would like to have one contiguous range, either free all the back to back blocks and
    /// allocate them again, or call `realloc`.
    pub fn range_borrow_mut(
        &mut self,
        process_id: Process,
        range: Range<u32>,
    ) -> Result<&mut [u8]> {
        let allocated = {
            if !self.allocated.contains_key(&process_id) {
                return Err(AllocError::NoSuchProcess);
            }

            self.allocated.entry(process_id).or_insert(vec![])
        };

        if let Some(_found_range) = allocated
            .iter()
            .find(|&x| (x.range.start <= range.start) && (x.range.end >= range.end))
        {
            // as range end is exclusive we have to add 1 to it, because
            // all indexable types start from 0 instead of 1
            Ok(&mut self.heap[range.start as usize..range.end as usize + 1])
        } else {
            Err(AllocError::NotOwned)
        }
    }

    pub fn share(
        &mut self,
        source_process: Process,
        target_process: Process,
        start_idx: u32,
    ) -> Result<()> {
        let allocated_source = {
            if !self.allocated.contains_key(&source_process) {
                return Err(AllocError::NoSuchProcess);
            }

            // safe to unwrap because we checked whether it exists
            self.allocated.get(&source_process).unwrap()
        };

        // instead of cloning the vec we clone the memrange, less overhead this way
        let memrange = {
            if let Some(found_range) = allocated_source
                .iter()
                .find(|&x| x.range.start <= start_idx)
            {
                found_range.clone()
            } else {
                return Err(AllocError::NotOwned);
            }
        };

        let allocated_target = {
            if !self.allocated.contains_key(&target_process) {
                return Err(AllocError::NoSuchProcess);
            }

            self.allocated.entry(target_process).or_insert(vec![])
        };

        (*memrange.refcount).fetch_add(1, Ordering::SeqCst);
        let refcount = Arc::clone(&memrange.refcount);

        allocated_target.push(MemRange::new(refcount, memrange.range));
        //Ok(&self.heap[range.start as usize..range.end as usize + 1])
        Ok(())
    }

    pub fn clean_process(&mut self, process_id: Process) -> Result<()> {
        if !self.allocated.contains_key(&process_id) {
            return Err(AllocError::NoSuchProcess);
        }

        let vec = self.allocated[&process_id].clone();

        for block in vec {
            self.free(process_id, block.range.start)?;
        }

        self.allocated.remove(&process_id);
        Ok(())
    }
}
