use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Helper type to treat a [`NonNull`] as a `&T` in terms of variance,
/// auto-traits, and retaining a lifetime.
///
/// This catches potential mistakes with lifetimes, as well as when
/// implementing [`Send`] and [`Sync`] for the containing type.
///
/// Note that this _does not_ imply that it's safe to dereference.
/// It is just a helper for non-null pointers to immutable data.
#[repr(transparent)]
pub struct RawRef<'a, T: ?Sized> {
    pub ptr: NonNull<T>,
    _lifetime: PhantomData<&'a T>,
}

// SAFETY: treat `RawRef` as if it was a regular reference for send/sync
unsafe impl<'a, T: ?Sized> Send for RawRef<'a, T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized> Sync for RawRef<'a, T> where &'a T: Sync {}

impl<T: ?Sized> Copy for RawRef<'_, T> {}
impl<T: ?Sized> Clone for RawRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ?Sized> RawRef<'a, T> {
    /// Casts a pointer to a different type.
    pub fn cast<U>(self) -> RawRef<'a, U> {
        RawRef::from(self.ptr.cast())
    }

    /// Returns a shared reference to the value.
    ///
    /// See documentation for [`NonNull::as_ref`].
    pub unsafe fn as_ref(self) -> &'a T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a, T> RawRef<'a, [T]> {
    /// Casts a slice pointer to a pointer to where its first element would be.
    pub fn cast_element(self) -> RawRef<'a, T> {
        self.cast()
    }
}

impl<T: Sized> RawRef<'_, T> {
    /// See documentation for [`NonNull::add`].
    ///
    /// Retains the lifetime.
    pub unsafe fn add(self, offset: usize) -> Self {
        Self::from(unsafe { self.ptr.add(offset) })
    }
}

impl<T: ?Sized> From<NonNull<T>> for RawRef<'_, T> {
    fn from(value: NonNull<T>) -> Self {
        Self {
            ptr: value,
            _lifetime: PhantomData,
        }
    }
}

impl<'a, T: ?Sized> From<&'a T> for RawRef<'a, T> {
    fn from(value: &'a T) -> Self {
        NonNull::from(value).into()
    }
}

impl<T: ?Sized> fmt::Debug for RawRef<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.ptr, f)
    }
}

impl<T: ?Sized> fmt::Pointer for RawRef<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}
