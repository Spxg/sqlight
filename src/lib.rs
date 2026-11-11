pub mod app;
mod protocol;
#[cfg(feature = "sqlite3")]
pub mod worker;
mod worker_client;

pub use protocol::*;
pub use send_wrapper::SendWrapper;
pub(crate) use worker_client::send_request;

use aceditor::EditorError;

#[derive(thiserror::Error, Debug)]
pub enum SQLightError {
    #[error(transparent)]
    Worker(#[from] WorkerError),
    #[error(transparent)]
    AceEditor(#[from] EditorError),
    #[error("Failed to import db: {0}")]
    ImportDb(String),
}

impl SQLightError {
    pub fn new_worker(err: WorkerError) -> SendWrapper<Self> {
        SendWrapper::new(Self::Worker(err))
    }

    pub fn new_ace_editor(err: EditorError) -> SendWrapper<Self> {
        SendWrapper::new(Self::AceEditor(err))
    }
}
