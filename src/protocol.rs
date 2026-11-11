use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, WorkerError>;

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub enum WorkerError {
    #[error(transparent)]
    SQLite(#[from] SQLitendError),
    #[error("DB is not opened")]
    NotOpened,
    #[error("Execute sqlite with invalid state")]
    InvalidState,
    #[error("OPFS already opened")]
    OpfsSAHPoolOpened,
    #[error("Failed to load db: {0}")]
    LoadDb(String),
    #[error("Failed to download db: {0}")]
    DownloadDb(String),
    #[error("Worker communication failed: {0}")]
    Transport(String),
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
    pub(crate) filename: String,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub(crate) data: Uint8Array,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub(crate) embed: bool,
    pub(crate) result: Vec<SQLiteStatementResult>,
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
    #[error("Unsupported column type: {0}")]
    UnsupportedColumnType(i32),
}
