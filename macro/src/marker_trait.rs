pub trait Always {
}

impl<T> Always for T {
}

#[macro_export]
macro_rules! marker_trait {
    {$tr:ident: $($bounds:tt)*} => {
        pub trait $tr: $crate::marker_trait::Always $($bounds)* { }
        impl<T: $crate::marker_trait::Always $($bounds)*> $tr for T { }
    }
}
