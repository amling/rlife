use ars_validates::Validates;
use ars_validates::ValidationError;
use ars_validates::ValidationResult;
use std::sync::Arc;

#[derive(Default)]
pub struct BooleanOption(bool);

impl Validates for BooleanOption {
    type Target = bool;

    fn validate(self) -> ValidationResult<bool> {
        Result::Ok(self.0)
    }
}

impl BooleanOption {
    pub fn set(&mut self) -> ValidationResult<()> {
        self.0 = true;
        Result::Ok(())
    }

    pub fn clear(&mut self) -> ValidationResult<()> {
        self.0 = false;
        Result::Ok(())
    }
}

pub trait OptionDefaulter<T> {
    fn default() -> ValidationResult<T>;
}

#[macro_export]
macro_rules! option_defaulters {
    {$($id:ident: $r:ty => $e:expr,)*} => {
        $(
            // arggh, can't derive Default on stuff templated on this
            // otherwise...
            #[derive(Default)]
            pub struct $id();

            impl $crate::vals::OptionDefaulter<$r> for $id {
                fn default() -> ::ars_validates::ValidationResult<$r> {
                    Result::Ok($e)
                }
            }
        )*
    }
}

pub struct DefaultedOption<T, P>(Option<T>, std::marker::PhantomData<P>);

impl<T, P> Default for DefaultedOption<T, P> {
    fn default() -> Self {
        DefaultedOption(None, std::marker::PhantomData::default())
    }
}

impl<T, P: OptionDefaulter<T>> Validates for DefaultedOption<T, P> {
    type Target = T;

    fn validate(self) -> ValidationResult<T> {
        match self.0 {
            Some(t) => Result::Ok(t),
            None => P::default(),
        }
    }
}

impl<T, P> DefaultedOption<T, P> {
    pub fn set(&mut self, t: impl Into<T>) -> ValidationResult<()> {
        if self.0.is_some() {
            return ValidationError::message("DefaultedOption specified multiple times".to_string());
        }
        self.0 = Some(t.into());
        Result::Ok(())
    }

    pub fn maybe_set(&mut self, t: impl Into<T>) -> ValidationResult<bool> {
        if self.0.is_some() {
            return Result::Ok(false);
        }
        self.0 = Some(t.into());
        Result::Ok(true)
    }

    pub fn maybe_set_with(&mut self, f: impl FnOnce() -> T) -> ValidationResult<bool> {
        if self.0.is_some() {
            return Result::Ok(false);
        }
        self.0 = Some(f());
        Result::Ok(true)
    }
}

impl<T> OptionDefaulter<T> for ErrDefaulter {
    fn default() -> ValidationResult<T> {
        ValidationError::message("Missing option".to_string())
    }
}

pub type DefaultedStringOption<P> = DefaultedOption<String, P>;

impl<P> DefaultedStringOption<P> {
    pub fn set_str(&mut self, a: impl Into<String>) -> ValidationResult<()> {
        self.set(a)
    }

    pub fn maybe_set_str(&mut self, a: impl Into<String>) -> ValidationResult<bool> {
        self.maybe_set(a)
    }
}

pub enum ErrDefaulter {
}

pub type RequiredOption<T> = DefaultedOption<T, ErrDefaulter>;

pub type RequiredStringOption = DefaultedStringOption<ErrDefaulter>;

pub type OptionalOption<T> = UnvalidatedOption<Option<T>>;

impl<T> OptionalOption<T> {
    pub fn set(&mut self, t: impl Into<T>) -> ValidationResult<()> {
        if self.0.is_some() {
            return ValidationError::message("OptionalOption specified multiple times".to_string());
        }
        self.0 = Some(t.into());
        Result::Ok(())
    }

    pub fn maybe_set(&mut self, t: impl Into<T>) -> ValidationResult<bool> {
        if self.0.is_some() {
            return Result::Ok(false);
        }
        self.0 = Some(t.into());
        Result::Ok(true)
    }
}

pub type OptionalStringOption = OptionalOption<String>;

// now pointless but kept for backcompat
impl OptionalStringOption {
    pub fn set_str(&mut self, a: impl Into<String>) -> ValidationResult<()> {
        self.set(a)
    }

    pub fn maybe_set_str(&mut self, a: impl Into<String>) -> ValidationResult<bool> {
        self.maybe_set(a)
    }
}

#[derive(Default)]
pub struct UnvalidatedOption<T>(pub T);

impl<T> Validates for UnvalidatedOption<T> {
    type Target = T;

    fn validate(self) -> ValidationResult<T> {
        Result::Ok(self.0)
    }
}

pub type StringVecOption = UnvalidatedOption<Vec<String>>;

impl StringVecOption {
    pub fn push(&mut self, s: impl Into<String>) -> ValidationResult<()> {
        self.0.push(s.into());
        Result::Ok(())
    }

    pub fn push_split(&mut self, s: impl AsRef<str>) -> ValidationResult<()> {
        for a in s.as_ref().split(',') {
            self.push(a)?;
        }
        Result::Ok(())
    }

    pub fn push_all(&mut self, a: &[impl Into<String> + Clone]) -> ValidationResult<()> {
        for a in a {
            self.0.push(a.clone().into());
        }
        Result::Ok(())
    }

    pub fn maybe_push(&mut self, a: impl Into<String>) -> ValidationResult<bool> {
        self.push(a).map(|_| true)
    }
}

pub type OptionalUsizeOption = OptionalOption<usize>;

impl OptionalUsizeOption {
    pub fn parse(&mut self, a: impl AsRef<str>) -> ValidationResult<()> {
        let n: usize = a.as_ref().parse()?;
        self.set(n)
    }
}

#[derive(Default)]
pub struct IntoArcOption<P>(pub P);

impl<P: Validates> Validates for IntoArcOption<P> {
    type Target = Arc<P::Target>;

    fn validate(self) -> ValidationResult<Arc<P::Target>> {
        self.0.validate().map(Arc::new)
    }
}

#[derive(Default)]
pub struct EmptyOption();

impl Validates for EmptyOption {
    type Target = ();

    fn validate(self) -> ValidationResult<()> {
        Result::Ok(())
    }
}
