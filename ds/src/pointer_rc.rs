use std::rc::Rc;

pub struct PointerRc<T: ?Sized>(pub Rc<T>);

impl<T: ?Sized> PartialEq for PointerRc<T> {
    fn eq(&self, rhs: &PointerRc<T>) -> bool {
        return Rc::ptr_eq(&self.0, &rhs.0);
    }
}

impl<T: ?Sized> Eq for PointerRc<T> {
}

impl<T: ?Sized> Clone for PointerRc<T> {
    fn clone(&self) -> PointerRc<T> {
        return PointerRc(self.0.clone());
    }
}
