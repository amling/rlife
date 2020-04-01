use std::hash::Hash;
use std::hash::Hasher;
use std::rc::Rc;

pub struct PointerRc<T: ?Sized>(pub Rc<T>);

impl<T: ?Sized> PointerRc<T> {
    // This works inline but I'd rather be very clear about what's going on.
    fn ptr(&self) -> &T {
        &*self.0
    }
}

impl<T: ?Sized> PartialEq for PointerRc<T> {
    fn eq(&self, rhs: &PointerRc<T>) -> bool {
        std::ptr::eq(self.ptr(), rhs.ptr())
    }
}

impl<T: ?Sized> Eq for PointerRc<T> {
}

impl<T: ?Sized> Clone for PointerRc<T> {
    fn clone(&self) -> PointerRc<T> {
        PointerRc(self.0.clone())
    }
}

impl<T: ?Sized> Hash for PointerRc<T> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        std::ptr::hash(self.ptr(), h);
    }
}
