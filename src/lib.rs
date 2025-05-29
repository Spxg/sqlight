pub mod app;
#[cfg(any(feature = "sqlite3", feature = "sqlite3mc"))]
pub mod worker;

use aceditor::EditorError;
use app::{Exported, GlobalState, GlobalStateStoreFields, Vfs};
use fragile::Fragile;
use js_sys::Uint8Array;
use leptos::prelude::*;
use reactive_stores::Store;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Once},
};
use tokio::sync::{OnceCell, mpsc::UnboundedReceiver};
use wasm_bindgen::{JsCast, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
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
    #[error("DB is not opened")]
    NotOpened,
    #[error("Execute sqlite with invaild state")]
    InvaildState,
    #[error("OPFS already opened")]
    OpfsSAHPoolOpened,
    #[error("Failed to load db: {0}")]
    LoadDb(String),
    #[error("Failed to download db: {0}")]
    DownloadDb(String),
    #[error("Unexpected error")]
    Unexpected,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerRequest {
    Open(OpenOptions),
    Run(RunOptions),
    LoadDb(LoadDbOptions),
    DownloadDb,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    Ready,
    Open(Result<()>),
    Run(Result<SQLiteRunResult>),
    LoadDb(Result<()>),
    DownloadDb(Result<DownloadDbResponse>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadDbResponse {
    filename: String,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    data: Uint8Array,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenOptions {
    pub filename: String,
    pub persist: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadDbOptions {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub data: Uint8Array,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunOptions {
    pub sql: String,
    pub embed: bool,
    pub clear_on_prepare: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SQLiteRunResult {
    embed: bool,
    result: Vec<SQLiteStatementResult>,
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

fn send_request(state: Store<GlobalState>, req: WorkerRequest) {
    spawn_local(async move {
        if state.multiple_ciphers().get_untracked() {
            sqlite3mc(state).await
        } else {
            sqlite3(state).await
        }
        .send_task(req);
    });
}

async fn sqlite3mc(state: Store<GlobalState>) -> &'static WorkerHandle {
    static ONCE: Once = Once::new();
    static WORKER: OnceCell<WorkerHandle> = OnceCell::const_new();

    let worker = WORKER
        .get_or_init(|| async { setup_worker(state, "./sqlite3mc_loader.js").await })
        .await;

    ONCE.call_once(|| {
        connect_db(state, worker);
        Effect::new(move || connect_db(state, worker));
    });
    worker
}

async fn sqlite3(state: Store<GlobalState>) -> &'static WorkerHandle {
    static ONCE: Once = Once::new();
    static WORKER: OnceCell<WorkerHandle> = OnceCell::const_new();

    let worker = WORKER
        .get_or_init(|| async { setup_worker(state, "./sqlite3_loader.js").await })
        .await;

    ONCE.call_once(|| {
        connect_db(state, worker);
        Effect::new(move || connect_db(state, worker));
    });
    worker
}

fn connect_db(state: Store<GlobalState>, handle: &'static WorkerHandle) {
    handle.send_task(crate::WorkerRequest::Open(crate::OpenOptions {
        filename: "test.db".into(),
        persist: *state.vfs().read() == Vfs::OPFS,
    }));
}

async fn setup_worker(state: Store<GlobalState>, uri: &str) -> WorkerHandle {
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

    spawn_local(handle_state(state, rx));

    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    wait.notified().await;

    WorkerHandle(worker)
}

async fn handle_state(state: Store<GlobalState>, mut rx: UnboundedReceiver<WorkerResponse>) {
    while let Some(resp) = rx.recv().await {
        state.last_error().set(None);

        match resp {
            WorkerResponse::Ready => unreachable!(),
            WorkerResponse::Open(result) => {
                if let Err(err) = result {
                    state.last_error().set(Some(SQLightError::new_worker(err)));
                }
            }
            WorkerResponse::Run(result) => match result {
                Ok(SQLiteRunResult { embed, result }) => {
                    if embed {
                        state.embed().set(result);
                    } else {
                        state.output().set(result);
                    }
                }
                Err(err) => state.last_error().set(Some(SQLightError::new_worker(err))),
            },
            WorkerResponse::LoadDb(result) => {
                let keep_ctx = result.is_ok();
                if let Some(progress) = &mut *state.import_progress().write() {
                    progress.opened = Some(keep_ctx);
                }
                if let Err(err) = result {
                    state.last_error().set(Some(SQLightError::new_worker(err)));
                }
                state
                    .keep_ctx()
                    .maybe_update(|keep| std::mem::replace(keep, keep_ctx) != keep_ctx);
            }
            WorkerResponse::DownloadDb(result) => match result {
                Ok(resp) => {
                    state.exported().set(Some(Exported {
                        filename: resp.filename,
                        data: FragileComfirmed::new(resp.data),
                    }));
                }
                Err(err) => {
                    state.last_error().set(Some(SQLightError::new_worker(err)));
                }
            },
        }
    }
}
