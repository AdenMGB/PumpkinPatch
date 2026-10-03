use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use thiserror::Error as ThisError;

pub mod analytics;
pub mod catalog;
pub mod console;
pub mod network;
pub mod ping;
pub mod server;
pub mod settings;

pub type Result<T> = std::result::Result<T, PatchApiError>;

#[derive(ThisError, Debug)]
pub enum PatchApiError {
    #[error("{0}")]
    Core(#[from] patch_core::Error),
    #[error("state not initialized")]
    NotInitialized,
}

impl Serialize for PatchApiError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let message = self.to_string();
        let mut s = serializer.serialize_struct("PatchApiError", 1)?;
        s.serialize_field("message", &message)?;
        s.end()
    }
}
