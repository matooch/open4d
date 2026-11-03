//! Bounded-allocation checking: a counting global allocator that tracks the live and peak
//! bytes of the **current thread** (docs/specs/PLATFORM.md §9.2).
//!
//! A test or fuzz binary installs it with `#[global_allocator] static A: CountingAlloc =
//! CountingAlloc;`, calls [`reset`] before an input and [`assert_bounded`] after it. Without
//! the allocator installed the counters stay zero and the check passes trivially.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Peak live bytes one input may use before a target fails (512 MiB).
pub const DEFAULT_ALLOC_LIMIT: usize = 512 << 20;

thread_local! {
    // `const` initialisers: no lazy allocation inside the allocator.
    static LIVE: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

fn add(n: usize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get().saturating_add(n);
        live.set(now);
        let _ = PEAK.try_with(|peak| {
            if now > peak.get() {
                peak.set(now);
            }
        });
    });
}

fn sub(n: usize) {
    let _ = LIVE.try_with(|live| live.set(live.get().saturating_sub(n)));
}

/// `System` plus per-thread live/peak counters.
pub struct CountingAlloc;

// SAFETY: every method forwards to `System` with the caller's layout/pointers unchanged; the
// counters are thread-local `Cell`s touched only for bookkeeping and never allocate.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: same contract as the caller's.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            add(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: same contract as the caller's.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            add(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        sub(layout.size());
        // SAFETY: `ptr` and `layout` come from a matching `alloc` call of the caller.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: `ptr`/`layout` come from the caller's earlier allocation; `new_size` is theirs.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            sub(layout.size());
            add(new_size);
        }
        p
    }
}

/// Starts a measurement: the peak becomes the current live size.
pub fn reset() {
    let live = LIVE.with(Cell::get);
    PEAK.with(|p| p.set(live));
}

/// Peak live bytes of this thread since [`reset`].
pub fn peak() -> usize {
    PEAK.with(Cell::get)
}

/// Panics when the peak live bytes since [`reset`] exceed `limit`.
pub fn assert_bounded(limit: usize) {
    let peak = peak();
    assert!(
        peak <= limit,
        "allocation not bounded: peak {peak} bytes exceeds {limit}"
    );
}
