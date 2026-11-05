#![no_main]
//! Fuzz target `image` (docs/specs/PLATFORM.md §9.1): no panic, bounded allocation.

use forge_fuzz_harness::alloc::{CountingAlloc, DEFAULT_ALLOC_LIMIT, assert_bounded, reset};
use libfuzzer_sys::fuzz_target;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

fuzz_target!(|data: &[u8]| {
    reset();
    forge_fuzz_harness::fuzz_image(data);
    assert_bounded(DEFAULT_ALLOC_LIMIT);
});
