use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use bloom_core::Identify;
use futures::task::SpawnError;
use toapi_client::Error as ToapiError;

#[derive(Debug)]
pub enum Error {
    ToapiError(ToapiError),
    SpawnError(SpawnError),
}

impl From<SpawnError> for Error {
    fn from(v: SpawnError) -> Self {
        Self::SpawnError(v)
    }
}

impl From<ToapiError> for Error {
    fn from(v: ToapiError) -> Self {
        Self::ToapiError(v)
    }
}

impl Identify for Error {
    fn id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        match self {
            Self::ToapiError(error) => {
                0.hash(&mut hasher);
                error.hash(&mut hasher);
            }
            Self::SpawnError(error) => {
                1.hash(&mut hasher);
                format!("{:?}", error).hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}
