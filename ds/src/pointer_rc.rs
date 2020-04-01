use std::hash::Hash;
use std::hash::Hasher;
use std::rc::Rc;

pub struct PointerRc<T: ?Sized>(pub Rc<T>);

impl<T: ?Sized> PartialEq for PointerRc<T> {
    fn eq(&self, rhs: &PointerRc<T>) -> bool {
        Rc::ptr_eq(&self.0, &rhs.0)
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
        // This works inline but I'd rather be very sure the type of whatever we're throwing into
        // hashing.
        let p: &T = &*self.0;
        std::ptr::hash(p, h);
    }
}
