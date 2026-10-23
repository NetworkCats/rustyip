//! Global allocator: mimalloc for request-path allocations, the system
//! allocator for very large blocks.
//!
//! The production image is built against musl, whose allocator is slow and
//! takes a global lock; it was the largest user-space cost of serving `/json`.
//! mimalloc, however, keeps a freed ~100 MB MMDB buffer resident after a
//! database update, while musl unmaps it immediately. Routing large blocks to
//! the system allocator keeps the speed without holding a dead database.

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;

use mimalloc::MiMalloc;

/// Blocks at least this large go to the system allocator. Far above anything
/// allocated per request, far below the MMDB file contents.
const LARGE_ALLOC_BYTES: usize = 1 << 20;

pub struct Allocator;

fn is_large(size: usize) -> bool {
    size >= LARGE_ALLOC_BYTES
}

// SAFETY: every block is freed or resized by the allocator that produced it,
// since the choice depends only on the block size, which `dealloc` and
// `realloc` receive unchanged from the caller.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            if is_large(layout.size()) {
                System.alloc(layout)
            } else {
                MiMalloc.alloc(layout)
            }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe {
            if is_large(layout.size()) {
                System.alloc_zeroed(layout)
            } else {
                MiMalloc.alloc_zeroed(layout)
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            if is_large(layout.size()) {
                System.dealloc(ptr, layout);
            } else {
                MiMalloc.dealloc(ptr, layout);
            }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            match (is_large(layout.size()), is_large(new_size)) {
                (false, false) => MiMalloc.realloc(ptr, layout, new_size),
                (true, true) => System.realloc(ptr, layout, new_size),
                // Crossing the threshold moves the block to the other allocator.
                _ => {
                    let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
                    let new_ptr = self.alloc(new_layout);
                    if !new_ptr.is_null() {
                        ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
                        self.dealloc(ptr, layout);
                    }
                    new_ptr
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realloc_across_threshold_preserves_contents() {
        let small = 64 * 1024;
        let large = 4 * LARGE_ALLOC_BYTES;
        let layout = Layout::from_size_align(small, 16).unwrap();
        unsafe {
            let p = Allocator.alloc(layout);
            assert!(!p.is_null());
            for i in 0..small {
                *p.add(i) = i as u8;
            }

            let grown = Allocator.realloc(p, layout, large);
            assert!(!grown.is_null());
            assert_eq!(grown as usize % 16, 0);
            for i in 0..small {
                assert_eq!(*grown.add(i), i as u8);
            }

            let large_layout = Layout::from_size_align(large, 16).unwrap();
            let shrunk = Allocator.realloc(grown, large_layout, small);
            assert!(!shrunk.is_null());
            for i in 0..small {
                assert_eq!(*shrunk.add(i), i as u8);
            }
            Allocator.dealloc(shrunk, layout);
        }
    }

    #[test]
    fn alloc_zeroed_large_block_is_zeroed() {
        let layout = Layout::from_size_align(2 * LARGE_ALLOC_BYTES, 8).unwrap();
        unsafe {
            let p = Allocator.alloc_zeroed(layout);
            assert!(!p.is_null());
            assert!((0..layout.size()).step_by(4096).all(|i| *p.add(i) == 0));
            Allocator.dealloc(p, layout);
        }
    }
}
