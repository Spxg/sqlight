pub mod app;
pub mod worker;

use aceditor::EditorError;
use app::{GlobalState, GlobalStateStoreFields};
use fragile::Fragile;
use leptos::prelude::*;
use reactive_stores::Store;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::mpsc::UnboundedReceiver;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, WorkerError>;

/// A [`FragileComfirmed<T>`] wraps a non sendable `T` to be safely send to other threads.
///
/// Once the value has been wrapped it can be sent to other threads but access
/// to the value on those threads will fail.
pub struct FragileComfirmed<T> {
    fragile: Fragile<T>,
}

unsafe impl<T> Send for FragileComfirmed<T> {}
unsafe impl<T> Sync for FragileComfirmed<T> {}

impl<T> FragileComfirmed<T> {
    pub fn new(t: T) -> Self {
        FragileComfirmed {
            fragile: Fragile::new(t),
        }
    }
}

impl<T> Deref for FragileComfirmed<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.fragile.get()
    }
}

impl<T> DerefMut for FragileComfirmed<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.fragile.get_mut()
    }
}

pub const PERSIST_VFS: &str = "sqlight-sahpool";

#[derive(thiserror::Error, Debug)]
pub enum SQLightError {
    #[error(transparent)]
    Worker(#[from] WorkerError),
    #[error(transparent)]
    AceEditor(#[from] EditorError),
}

impl SQLightError {
    pub fn new_worker(err: WorkerError) -> FragileComfirmed<Self> {
        FragileComfirmed::new(Self::Worker(err))
    }

    pub fn new_ace_editor(err: EditorError) -> FragileComfirmed<Self> {
        FragileComfirmed::new(Self::AceEditor(err))
    }
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub enum WorkerError {
    #[error(transparent)]
    SQLite(#[from] SQLitendError),
    #[error("Not found database by id")]
    NotFound,
    #[error("Execute sqlite with invaild state")]
    InvaildState,
    #[error("OPFS already opened")]
    OpfsSAHPoolOpened,
    #[error("OPFS unexpected error")]
    OpfsSAHError,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerRequest {
    Open(OpenOptions),
    Prepare(PrepareOptions),
    Continue(String),
    StepOver(String),
    StepIn(String),
    StepOut(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    Ready,
    Open(Result<String>),
    Prepare(Result<()>),
    Continue(Result<Vec<SQLiteStatementResult>>),
    StepOver(Result<SQLiteStatementResult>),
    StepIn(Result<()>),
    StepOut(Result<SQLiteStatementResult>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenOptions {
    pub filename: String,
    pub persist: bool,
}

impl OpenOptions {
    pub fn uri(&self) -> String {
        format!(
            "file:{}?vfs={}",
            self.filename,
            if self.persist { PERSIST_VFS } else { "memvfs" }
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrepareOptions {
    pub id: String,
    pub sql: String,
    pub clear_on_prepare: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SQLiteStatementResult {
    Finish,
    Step(SQLiteStatementTable),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SQLiteStatementTable {
    pub sql: String,
    pub position: [usize; 2],
    pub values: Option<SQLiteStatementValues>,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SQLiteStatementValues {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub enum SQLitendError {
    #[error("An error occurred while converting a string to a CString")]
    ToCStr,
    #[error("An error occurred while opening the DB: {0:#?}")]
    OpenDb(InnerError),
    #[error("An error occurred while preparing stmt: {0:#?}")]
    Prepare(InnerError),
    #[error("An error occurred while stepping to the next line: {0:#?}")]
    Step(InnerError),
    #[error("An error occurred while getting column name: {0}")]
    GetColumnName(String),
    #[error("The text is not a utf8 string")]
    Utf8Text,
    #[error("The column type is not support: {0}")]
    UnsupportColumnType(i32),
}

pub struct WorkerHandle(Worker);

impl WorkerHandle {
    pub fn send_task(&self, req: WorkerRequest) {
        if let Err(err) = self
            .0
            .post_message(&serde_wasm_bindgen::to_value(&req).unwrap())
        {
            log::error!("Failed to send task to worker: {req:?}, {err:?}");
        }
    }
}

unsafe impl Send for WorkerHandle {}
unsafe impl Sync for WorkerHandle {}

pub async fn setup_worker() -> (WorkerHandle, UnboundedReceiver<WorkerResponse>) {
    let uri = "./worker_loader.js";

    let opts = WorkerOptions::new();
    opts.set_type(WorkerType::Module);

    let worker = match Worker::new_with_options(uri, &opts) {
        Ok(worker) => worker,
        Err(err) => panic!("Failed to new setup worker: {err:?}"),
    };

    let notify = Arc::new(tokio::sync::Notify::new());
    let wait = Arc::clone(&notify);

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |ev: MessageEvent| {
        match serde_wasm_bindgen::from_value(ev.data()) {
            Ok(WorkerResponse::Ready) => notify.notify_one(),
            Ok(resp) => tx.send(resp).unwrap(),
            Err(err) => log::error!("Failed to parse message {:?}", err),
        }
    });

    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    wait.notified().await;

    (WorkerHandle(worker), rx)
}

pub async fn handle_state(state: Store<GlobalState>, mut rx: UnboundedReceiver<WorkerResponse>) {
    while let Some(resp) = rx.recv().await {
        match resp {
            WorkerResponse::Ready => unreachable!(),
            WorkerResponse::Open(result) => match result {
                Ok(_) => (),
                Err(err) => state.last_error().set(Some(SQLightError::new_worker(err))),
            },
            WorkerResponse::Prepare(result) => {
                if let Err(err) = result {
                    state.last_error().set(Some(SQLightError::new_worker(err)));
                }
            }
            WorkerResponse::Continue(result) => match result {
                Ok(results) => state.output().set(results),
                Err(err) => state.last_error().set(Some(SQLightError::new_worker(err))),
            },
            WorkerResponse::StepOver(_)
            | WorkerResponse::StepIn(_)
            | WorkerResponse::StepOut(_) => unimplemented!(),
        }
    }
}
