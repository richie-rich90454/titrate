//! Bump and region allocators for the freestanding image.
//!
//! Only the static instance at the bottom of this file is installed as the
//! global allocator, and only for `target_os = "none"`. On a hosted build the
//! same types stay inert so the standard allocator keeps the process heap.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Static heap size backing the freestanding global allocator.
pub const HEAP_SIZE: usize = 4 * 1024 * 1024;

/// Round `value` up to the next multiple of the power-of-two `align`.
fn align_up(value: usize, align: usize) -> Option<usize> {
    let mask = align - 1;
    value.checked_add(mask).map(|sum| sum & !mask)
}

/// A lock-free bump allocator over a fixed in-place byte array.
///
/// Reclaiming a block is deliberately impossible: a cursor compare-and-swap
/// is the whole allocation path, so an interrupt cannot deadlock on a lock.
pub struct BumpAllocator<const N: usize> {
    storage: UnsafeCell<[u8; N]>,
    cursor: AtomicUsize,
}

// SAFETY: the cursor is atomic and every allocation reserves a distinct range
// of `storage`. Sharing `&BumpAllocator` between CPUs cannot alias a block.
unsafe impl<const N: usize> Sync for BumpAllocator<N> {}

impl<const N: usize> BumpAllocator<N> {
    /// Create an empty allocator. Usable in a `static` initializer.
    pub const fn new() -> Self {
        Self {
            storage: UnsafeCell::new([0u8; N]),
            cursor: AtomicUsize::new(0),
        }
    }

    /// Reserve a block and return its address, or `None` when exhausted.
    pub fn alloc_layout(&self, layout: Layout) -> Option<NonNull<u8>> {
        let base = self.storage.get() as usize;
        loop {
            let current = self.cursor.load(Ordering::Relaxed);
            let aligned = align_up(current, layout.align())?;
            let next = aligned.checked_add(layout.size())?;
            if next > N {
                return None;
            }
            if self
                .cursor
                .compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_err()
            {
                continue;
            }
            // The winning CAS reserved `[aligned, next)` inside the live
            // `[u8; N]`, and `base + aligned` cannot be null.
            let ptr = (base + aligned) as *mut u8;
            return NonNull::new(ptr);
        }
    }

    /// Number of bytes handed out so far, including alignment padding.
    pub fn used(&self) -> usize {
        self.cursor.load(Ordering::Relaxed)
    }

    /// Rewind the cursor to zero.
    ///
    /// Only call this when every allocation from this arena is dead; there is
    /// no per-block lifetime tracking to enforce that for the caller.
    pub fn reset(&self) {
        self.cursor.store(0, Ordering::SeqCst);
    }

    fn watermark(&self) -> usize {
        self.cursor.load(Ordering::SeqCst)
    }

    fn rewind_to(&self, mark: usize) {
        // Never advance the cursor: a nested region must not resurrect blocks
        // reserved after the mark it was entered with.
        self.cursor.fetch_min(mark, Ordering::SeqCst);
    }
}

impl<const N: usize> Default for BumpAllocator<N> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<const N: usize> GlobalAlloc for BumpAllocator<N> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_layout(layout)
            .map_or(core::ptr::null_mut(), NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = match Layout::from_size_align(new_size, layout.align()) {
            Ok(value) => value,
            Err(_) => return core::ptr::null_mut(),
        };
        let new_ptr = self.alloc(new_layout);
        if new_ptr.is_null() {
            return new_ptr;
        }
        // SAFETY: both blocks are owned by this allocator and the new one is
        // at least `min(old, new)` bytes long. The old block stays reserved,
        // which is the bump allocator's defined deallocation behavior.
        unsafe {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, core::cmp::min(layout.size(), new_size));
        }
        new_ptr
    }
}

/// An allocator whose cursor can be marked and rewound for scoped lifetimes.
pub trait Resettable {
    /// Capture the current allocation watermark.
    fn mark(&self) -> usize;

    /// Rewind to a previously captured watermark, never past it.
    fn reset_to(&self, mark: usize);
}

impl<const N: usize> Resettable for BumpAllocator<N> {
    fn mark(&self) -> usize {
        self.watermark()
    }

    fn reset_to(&self, mark: usize) {
        self.rewind_to(mark);
    }
}

/// A scoped allocation region. Blocks reserved inside it are reclaimed when
/// the guard is dropped, so a request handler can free all its temporaries at
/// once without a per-block deallocator.
pub struct Region<'a, A: Resettable> {
    allocator: &'a A,
    mark: usize,
}

impl<'a, A: Resettable> Region<'a, A> {
    /// Enter a region at the allocator's current watermark.
    pub fn enter(allocator: &'a A) -> Self {
        let mark = allocator.mark();
        Self { allocator, mark }
    }

    /// Allocator backing this region.
    pub fn allocator(&self) -> &'a A {
        self.allocator
    }
}

impl<'a, A: Resettable> Drop for Region<'a, A> {
    fn drop(&mut self) {
        self.allocator.reset_to(self.mark);
    }
}

#[cfg(target_os = "none")]
#[global_allocator]
static GLOBAL_ALLOCATOR: BumpAllocator<HEAP_SIZE> = BumpAllocator::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_rounds_up_only_when_needed() {
        assert_eq!(align_up(0, 8), Some(0));
        assert_eq!(align_up(1, 8), Some(8));
        assert_eq!(align_up(8, 8), Some(8));
        assert_eq!(align_up(usize::MAX, 8), None);
    }

    #[test]
    fn allocations_are_aligned_and_distinct() {
        let arena = BumpAllocator::<64>::new();
        let a = arena
            .alloc_layout(Layout::from_size_align(3, 1).unwrap())
            .expect("first block fits");
        let b = arena
            .alloc_layout(Layout::from_size_align(4, 4).unwrap())
            .expect("second block fits");
        assert_ne!(a.as_ptr(), b.as_ptr());
        assert_eq!(b.as_ptr() as usize % 4, 0);
        assert_eq!(arena.used(), 8);
    }

    #[test]
    fn exhaustion_is_reported_instead_of_overwriting() {
        let arena = BumpAllocator::<8>::new();
        assert!(arena
            .alloc_layout(Layout::from_size_align(9, 1).unwrap())
            .is_none());
        assert_eq!(arena.used(), 0);
        assert!(arena
            .alloc_layout(Layout::from_size_align(8, 1).unwrap())
            .is_some());
        assert!(arena
            .alloc_layout(Layout::from_size_align(1, 1).unwrap())
            .is_none());
    }

    #[test]
    fn reset_reclaims_the_whole_arena() {
        let arena = BumpAllocator::<16>::new();
        assert!(arena
            .alloc_layout(Layout::from_size_align(16, 1).unwrap())
            .is_some());
        arena.reset();
        assert_eq!(arena.used(), 0);
        assert!(arena
            .alloc_layout(Layout::from_size_align(16, 1).unwrap())
            .is_some());
    }

    #[test]
    fn region_rewinds_on_drop_but_not_past_its_mark() {
        let arena = BumpAllocator::<32>::new();
        let outer = arena
            .alloc_layout(Layout::from_size_align(4, 1).unwrap())
            .expect("outer block fits");
        let watermark = arena.used();
        {
            let region = Region::enter(&arena);
            assert!(region
                .allocator()
                .alloc_layout(Layout::from_size_align(8, 1).unwrap())
                .is_some());
            assert_eq!(arena.used(), watermark + 8);
        }
        assert_eq!(arena.used(), watermark);
        assert_ne!(outer.as_ptr() as usize, 0);
    }

    #[test]
    fn nested_regions_do_not_rewind_a_parent() {
        let arena = BumpAllocator::<32>::new();
        let outer = Region::enter(&arena);
        {
            let inner = Region::enter(&arena);
            assert!(inner
                .allocator()
                .alloc_layout(Layout::from_size_align(8, 1).unwrap())
                .is_some());
        }
        assert_eq!(arena.used(), 0);
        assert_eq!(outer.allocator().used(), 0);
    }
}
