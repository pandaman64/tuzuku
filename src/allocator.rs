use std::ptr::NonNull;

pub(crate) static LEAKING_ALLOCATOR: LeakingAllocator = LeakingAllocator;

pub(crate) struct LeakingAllocator;

impl LeakingAllocator {
    pub(crate) fn alloc<T>(&self, value: T) -> NonNull<T> {
        // SAFETY: Box::into_raw returns non-null pointer
        unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(value))) }
    }

    pub(crate) fn alloc_array<T: Clone>(&self, init: T, num: usize) -> NonNull<[T]> {
        // SAFETY: Box::into_raw returns non-null pointer
        unsafe { NonNull::new_unchecked(Box::into_raw(vec![init; num].into_boxed_slice())) }
    }

    pub(crate) fn alloc_empty_array<T>(&self) -> NonNull<[T]> {
        // SAFETY: Box::into_raw returns non-null pointer
        unsafe { NonNull::new_unchecked(Box::into_raw(vec![].into_boxed_slice())) }
    }
}
