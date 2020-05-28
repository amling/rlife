use ars_ds::err::StringError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::path::Path;

pub enum SerdeFormat {
    JSON,
    Bincode,
}

impl SerdeFormat {
    pub fn write(&self, path: impl AsRef<Path>, t: &impl Serialize) -> Result<(), StringError> {
        let f = File::create(path)?;
        let f = BufWriter::new(f);
        match self {
            SerdeFormat::JSON => serde_json::to_writer(f, t)?,
            SerdeFormat::Bincode => bincode::serialize_into(f, t)?,
        };
        Ok(())
    }

    pub fn read<T: DeserializeOwned>(&self, path: impl AsRef<Path>) -> Result<T, StringError> {
        let f = File::open(path)?;
        let f = BufReader::new(f);
        let t = match self {
            SerdeFormat::JSON => serde_json::from_reader(f)?,
            SerdeFormat::Bincode => bincode::deserialize_from(f)?,
        };
        Ok(t)
    }
}
