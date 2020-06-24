use ars_ds::err::StringError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;

pub trait SerializerFor<T> {
    fn to_writer(&self, w: impl Write, t: &T) -> Result<(), StringError>;

    fn to_file(&self, path: impl AsRef<Path>, t: &T) -> Result<(), StringError> {
        let f = File::create(path)?;
        let f = BufWriter::new(f);
        self.to_writer(f, t)
    }
}

pub trait DeserializerFor<T> {
    fn from_reader(&self, r: impl Read) -> Result<T, StringError>;

    fn from_file(&self, path: impl AsRef<Path>) -> Result<T, StringError> {
        let f = File::open(path)?;
        let f = BufReader::new(f);
        let t = self.from_reader(f)?;
        Ok(t)
    }
}

pub struct JsonSerializer();

impl<T: Serialize> SerializerFor<T> for JsonSerializer {
    fn to_writer(&self, w: impl Write, t: &T) -> Result<(), StringError> {
        serde_json::to_writer(w, t)?;
        Ok(())
    }
}

impl<T: DeserializeOwned> DeserializerFor<T> for JsonSerializer {
    fn from_reader(&self, r: impl Read) -> Result<T, StringError> {
        let t = serde_json::from_reader(r)?;
        Ok(t)
    }
}

pub struct BincodeSerializer();

impl<T: Serialize> SerializerFor<T> for BincodeSerializer {
    fn to_writer(&self, w: impl Write, t: &T) -> Result<(), StringError> {
        bincode::serialize_into(w, t)?;
        Ok(())
    }
}

impl<T: DeserializeOwned> DeserializerFor<T> for BincodeSerializer {
    fn from_reader(&self, r: impl Read) -> Result<T, StringError> {
        let t = bincode::deserialize_from(r)?;
        Ok(t)
    }
}
