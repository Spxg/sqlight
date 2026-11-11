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
    #[error("Commit or roll back the current transaction before switching database settings")]
    ActiveTransaction,
    #[error("Failed to load db: {0}")]
    LoadDb(String),
    #[error("Failed to create db: {0}")]
    CreateDb(String),
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
    Close,
    CloseWorkspace(String),
    Run(RunOptions),
    LoadDb(LoadDbOptions),
    LoadDbs(LoadDbsOptions),
    CreateDb(String),
    ListDbs,
    DownloadDb,
    DownloadFile(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    Ready,
    Open(Result<()>),
    Close(Result<()>),
    CloseWorkspace(Result<()>),
    Run(Result<SQLiteRunResult>),
    LoadDb(Result<()>),
    LoadDbs(Result<Vec<String>>),
    CreateDb(Result<String>),
    ListDbs(Result<Vec<String>>),
    DownloadDb(Result<DownloadDbResponse>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadDbResponse {
    pub(crate) filename: String,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub(crate) data: Uint8Array,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenOptions {
    pub filename: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_workspace() -> String {
    "default".into()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadDbOptions {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub data: Uint8Array,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseFile {
    pub filename: String,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub data: Uint8Array,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadDbsOptions {
    pub files: Vec<DatabaseFile>,
    pub replace: bool,
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
    pub(crate) error: Option<SQLitendError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SQLiteStatementResult {
    Finish,
    Step(SQLiteStatementTable),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SQLiteStatementTable {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_open_requests_use_the_default_writable_workspace() {
        let options: OpenOptions =
            serde_json::from_str(r#"{"filename":"test.db","persist":false}"#).unwrap();
        assert!(!options.read_only);
        assert_eq!(options.workspace, "default");
    }
}
