use std::ops::Deref;

pub trait Universe: Sized {
    fn universe() -> Vec<Self>;
}

pub trait Named {
    type S: Deref<Target=str>;

    fn name(&self) -> Self::S;
}

marker_trait! {
    NamedUniverseMarker:
    + Named
    + Universe
}

pub trait NamedUniverse: NamedUniverseMarker {
    fn named(s: impl Deref<Target=str>) -> Self {
        let s = s.deref();
        for f in Self::universe() {
            if f.name().deref() == s {
                return f;
            }
        }
        panic!("Inappropriate {} {}", std::any::type_name::<Self>(), s);
    }
}

impl<T: NamedUniverseMarker> NamedUniverse for T {
}
