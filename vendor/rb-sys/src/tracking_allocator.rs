//! Support for reporting Rust memory usage to the Ruby GC.

use std::{
    fmt::Formatter,
    sync::{
        Arc,
        atomic::{AtomicIsize, Ordering},
    },
};

#[cfg(ruby_engine = "mri")]
mod mri {
    use crate::{rb_gc_adjust_memory_usage, utils::is_ruby_vm_started};
    use std::alloc::{GlobalAlloc, Layout, System};

    /// A simple wrapper over [`System`] which reports memory usage to
    /// the Ruby GC. This gives the GC a more accurate picture of the process'
    /// memory usage so it can make better decisions about when to run.
    #[derive(Debug)]
    pub struct TrackingAllocator;

    impl TrackingAllocator {
        /// Create a new [`TrackingAllocator`].
        #[allow(clippy::new_without_default)]
        pub const fn new() -> Self {
            Self
        }

        /// Create a new [`TrackingAllocator`] with default values.
        pub const fn default() -> Self {
            Self::new()
        }

        /// Adjust the memory usage reported to the Ruby GC by `delta`. Useful for
        /// tracking allocations invisible to the Rust allocator, such as `mmap` or
        /// direct `malloc` calls.
        ///
        /// # Example
        /// ```
        /// use rb_sys::TrackingAllocator;
        ///
        /// // Allocate 1024 bytes of memory using `mmap` or `malloc`...
        /// TrackingAllocator::adjust_memory_usage(1024);
        ///
        /// // ...and then after the memory is freed, adjust the memory usage again.
        /// TrackingAllocator::adjust_memory_usage(-1024);
        /// ```
        #[inline]
        pub fn adjust_memory_usage(delta: isize) -> isize {
            if delta == 0 {
                return 0;
            }

            #[cfg(target_pointer_width = "32")]
            let delta = delta as i32;

            #[cfg(target_pointer_width = "64")]
            let delta = delta as i64;

            unsafe {
                if is_ruby_vm_started() {
                    #[cfg(all(target_pointer_width = "64", target_os = "windows"))]
                    rb_gc_adjust_memory_usage(delta as i32);

                    #[cfg(not(all(target_pointer_width = "64", target_os = "windows")))]
                    rb_gc_adjust_memory_usage(delta);

                    delta as isize
                } else {
                    0
                }
            }
        }
    }

    unsafe impl GlobalAlloc for TrackingAllocator {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            let delta = layout.size() as isize;

            if !ret.is_null() && delta != 0 {
                Self::adjust_memory_usage(delta);
            }

            ret
        }

        #[inline]
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc_zeroed(layout);
            let delta = layout.size() as isize;

            if !ret.is_null() && delta != 0 {
                Self::adjust_memory_usage(delta);
            }

            ret
        }

        #[inline]
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            let delta = -(layout.size() as isize);

            if delta != 0 {
                Self::adjust_memory_usage(delta);
            }
        }

        #[inline]
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            let ret = System.realloc(ptr, layout, new_size);
            let delta = new_size as isize - layout.size() as isize;

            if !ret.is_null() && delta != 0 {
                Self::adjust_memory_usage(delta);
            }

            ret
        }
    }
}

#[cfg(not(ruby_engine = "mri"))]
mod non_mri {
    use std::alloc::{GlobalAlloc, Layout, System};

    /// A simple wrapper over [`System`] as a fallback for non-MRI Ruby engines.
    pub struct TrackingAllocator;

    impl TrackingAllocator {
        #[allow(clippy::new_without_default)]
        pub const fn new() -> Self {
            Self
        }

        pub const fn default() -> Self {
            Self::new()
        }

        pub fn adjust_memory_usage(_delta: isize) -> isize {
            0
        }
    }

    unsafe impl GlobalAlloc for TrackingAllocator {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            System.alloc(layout)
        }

        #[inline]
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            System.alloc_zeroed(layout)
        }

        #[inline]
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }

        #[inline]
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            System.realloc(ptr, layout, new_size)
        }
    }
}

#[cfg(ruby_engine = "mri")]
pub use mri::*;

#[cfg(not(ruby_engine = "mri"))]
pub use non_mri::*;

/// Set the global allocator to [`TrackingAllocator`].
///
/// # Example
/// ```
/// // File: ext/my_gem/src/lib.rs
/// use rb_sys::set_global_tracking_allocator;
///
/// set_global_tracking_allocator!();
/// ```
#[macro_export]
macro_rules! set_global_tracking_allocator {
    () => {
        #[global_allocator]
        static RUBY_GLOBAL_TRACKING_ALLOCATOR: $crate::tracking_allocator::TrackingAllocator =
            $crate::tracking_allocator::TrackingAllocator;
    };
}

#[derive(Debug)]
#[repr(transparent)]
struct MemsizeDelta(Arc<AtomicIsize>);

impl MemsizeDelta {
    fn new(delta: isize) -> Self {
        let delta = TrackingAllocator::adjust_memory_usage(delta);
        Self(Arc::new(AtomicIsize::new(delta)))
    }

    fn add(&self, delta: usize) {
        if delta == 0 {
            return;
        }

        let delta = TrackingAllocator::adjust_memory_usage(delta as _);
        self.0.fetch_add(delta as _, Ordering::SeqCst);
    }

    fn sub(&self, delta: usize) {
        if delta == 0 {
            return;
        }

        let delta = TrackingAllocator::adjust_memory_usage(-(delta as isize));
        self.0.fetch_add(delta, Ordering::SeqCst);
    }

    fn get(&self) -> isize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Clone for MemsizeDelta {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl Drop for MemsizeDelta {
    fn drop(&mut self) {
        let memsize = self.0.swap(0, Ordering::SeqCst);
        TrackingAllocator::adjust_memory_usage(0 - memsize);
    }
}

/// A guard which adjusts the memory usage reported to the Ruby GC by `delta`.
/// This allows you to track resources which are invisible to the Rust
/// allocator, such as items that are known to internally use `mmap` or direct
/// `malloc` in their implementation.
///
/// Internally, it uses an [`Arc<AtomicIsize>`] to track the memory usage delta,
/// and is safe to clone when `T` is [`Clone`].
///
/// # Example
/// ```
/// use rb_sys::tracking_allocator::ManuallyTracked;
///
/// type SomethingThatUsedMmap = ();
///
/// // Will tell the Ruby GC that 1024 bytes were allocated.
/// let item = ManuallyTracked::new(SomethingThatUsedMmap, 1024);
///
/// // Will tell the Ruby GC that 1024 bytes were freed.
/// std::mem::drop(item);
/// ```
pub struct ManuallyTracked<T> {
    item: T,
    memsize_delta: MemsizeDelta,
}

impl<T> ManuallyTracked<T> {
    /// Create a new `ManuallyTracked<T>`, and immediately report that `memsize`
    /// bytes were allocated.
    pub fn wrap(item: T, memsize: usize) -> Self {
        Self {
            item,
            memsize_delta: MemsizeDelta::new(memsize as _),
        }
    }

    /// Increase the memory usage reported to the Ruby GC by `memsize` bytes.
    pub fn increase_memory_usage(&self, memsize: usize) {
        self.memsize_delta.add(memsize);
    }

    /// Decrease the memory usage reported to the Ruby GC by `memsize` bytes.
    pub fn decrease_memory_usage(&self, memsize: usize) {
        self.memsize_delta.sub(memsize);
    }

    /// Get the current memory usage delta.
    pub fn memsize_delta(&self) -> isize {
        self.memsize_delta.get()
    }

    /// Get a shared reference to the inner `T`.
    pub fn get(&self) -> &T {
        &self.item
    }

    /// Get a mutable reference to the inner `T`.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.item
    }
}

impl ManuallyTracked<()> {
    /// Create a new `ManuallyTracked<()>`, and immediately report that
    /// `memsize` bytes were allocated.
    pub fn new(memsize: usize) -> Self {
        Self::wrap((), memsize)
    }
}

impl Default for ManuallyTracked<()> {
    fn default() -> Self {
        Self::wrap((), 0)
    }
}

impl<T: Clone> Clone for ManuallyTracked<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            memsize_delta: self.memsize_delta.clone(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ManuallyTracked<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManuallyTracked")
            .field("item", &self.item)
            .field("memsize_delta", &self.memsize_delta)
            .finish()
    }
}
