use std::ptr::NonNull;
use std::marker::PhantomData;

/// Helper type to treat a [`NonNull`] as a `&T` in terms of variance,
/// auto-traits, and retaining a lifetime.
///
/// This catches potential mistakes with lifetimes, as well as when
/// implementing [`Send`] and [`Sync`] for the containing type.
///
/// Note that this _does not_ imply that it's safe to dereference.
/// It is just a helper for non-null pointers to immutable data.
#[repr(transparent)]
pub(crate) struct RawRef<'a, T: ?Sized> {
    pub ptr: NonNull<T>,
    _lifetime: PhantomData<&'a T>,
}

unsafe impl<'a, T: ?Sized> Send for RawRef<'a, T> where &'a T: Send {}
unsafe impl<'a, T: ?Sized> Sync for RawRef<'a, T> where &'a T: Sync {}

impl<'a, T: ?Sized> Copy for RawRef<'a, T> {}
impl<'a, T: ?Sized> Clone for RawRef<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ?Sized> RawRef<'a, T> {
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

impl<'a, T: Sized> RawRef<'a, T> {
    /// See documentation for [`NonNull::add`].
    ///
    /// Retains the lifetime.
    pub unsafe fn add(self, offset: usize) -> Self {
        Self::from(unsafe { self.ptr.add(offset) })
    }
}

impl<'a, T: ?Sized> From<NonNull<T>> for RawRef<'a, T> {
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

impl<'a, T: ?Sized> std::fmt::Debug for RawRef<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.ptr, f)
    }
}

impl<'a, T: ?Sized> std::fmt::Pointer for RawRef<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.ptr, f)
    }
}
