use ars_ds::err::StringError;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::value::Value;

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct RctlType {
    pub s: String,
}

impl RctlType {
    pub fn of<T>() -> RctlType {
        RctlType {
            s: std::any::type_name::<T>().to_string(),
        }
    }
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct RctlArgMetadata {
    pub name: String,
    pub ty: RctlType,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct RctlMethodMetadata {
    pub args: Vec<RctlArgMetadata>,
    pub ret: RctlType,
}

pub struct RctlLog<'a>(pub Box<dyn FnMut(String) + 'a>);

impl<'a> RctlLog<'a> {
    pub fn ignore() -> RctlLog<'static> {
        RctlLog(Box::new(|_line| { }))
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        (self.0)(msg.into())
    }
}

pub trait RctlEp: Send + Sync {
    fn metadata() -> Vec<(String, RctlMethodMetadata)>;
    fn invoke(&self, log: RctlLog, method: impl AsRef<str>, args: &[Value]) -> Result<Value, StringError>;
}

pub struct RctlArgsBag<'a, 'l> {
    args: &'a [Value],
    log: Option<RctlLog<'l>>,
}

impl<'a, 'l> RctlArgsBag<'a, 'l> {
    pub fn new(args: &'a [Value], log: RctlLog<'l>) -> Self {
        RctlArgsBag {
            args: args,
            log: Some(log),
        }
    }

    pub fn is_done(&self) -> bool {
        self.args.len() == 0
    }
}

pub trait RctlArgTrait<'l>: Sized {
    fn take_arg<'a>(bag: &mut RctlArgsBag<'a, 'l>) -> Result<Self, StringError>;
    fn add_metadata(args: &mut Vec<RctlArgMetadata>, name: impl Into<String>);
}

impl<'l, T: DeserializeOwned> RctlArgTrait<'l> for T {
    fn take_arg<'a>(bag: &mut RctlArgsBag<'a, 'l>) -> Result<T, StringError> {
        match bag.args.split_first() {
            None => Err(StringError::new("Not enough arguments")),
            Some((first, rest)) => {
                bag.args = rest;
                let t = serde_json::from_value(first.clone())?;
                Ok(t)
            },
        }
    }

    fn add_metadata(args: &mut Vec<RctlArgMetadata>, name: impl Into<String>) {
        args.push(RctlArgMetadata {
            name: name.into(),
            ty: RctlType::of::<T>(),
        });
    }
}

impl<'l, 'l2> RctlArgTrait<'l> for RctlLog<'l2> where 'l: 'l2 {
    fn take_arg<'a>(bag: &mut RctlArgsBag<'a, 'l>) -> Result<RctlLog<'l2>, StringError> {
        match bag.log.take() {
            // could arguably be a derive error but my sanity is worth something too
            None => Err(StringError::new("Multiple log arguments?")),
            Some(log) => Ok(log),
        }
    }

    fn add_metadata(_args: &mut Vec<RctlArgMetadata>, _name: impl Into<String>) {
        // nope
    }
}
