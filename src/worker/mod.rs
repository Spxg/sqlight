mod raw_file;
mod sqlitend;

use std::collections::HashSet;

use crate::{
    DatabaseFile, DownloadDbResponse, LoadDbOptions, LoadDbsOptions, OpenOptions, RunOptions,
    SQLiteRunResult, WorkerError, WorkerRequest, WorkerResponse,
};
use js_sys::Uint8Array;
use sqlite_wasm_rs::{
    WasmOsCallback,
    vfs::{VfsFilesManager, transfer::DbTransfer},
};
use sqlite_wasm_vfs::sahpool::{OpfsSAHError, OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil};
use tokio::sync::mpsc;
use wasm_bindgen::{JsCast, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

use raw_file::ReadOnlyFile;
use sqlitend::SQLiteDb;

type Result<T> = std::result::Result<T, WorkerError>;

#[cfg(not(feature = "sqlite3mc"))]
const OPFS_VFS: &str = "opfs";
#[cfg(feature = "sqlite3mc")]
const OPFS_VFS: &str = "multipleciphers-opfs";

const OPFS_VFS_DIR: &str = if cfg!(feature = "sqlite3mc") {
    "sqlight-sahpool-mc"
} else if cfg!(feature = "sqlcipher") {
    "sqlight-sahpool-sqlcipher"
} else {
    "sqlight-sahpool"
};

fn uri(options: &OpenOptions) -> String {
    let filename = String::from(js_sys::encode_uri_component(&options.filename));
    format!("file:{filename}?vfs={OPFS_VFS}")
}

fn initialize_empty_db(options: &OpenOptions) -> Result<()> {
    let db = SQLiteDb::open(&uri(options), false)?;
    db.prepare("PRAGMA user_version = 0;")?
        .results(&mut Vec::new())?;
    Ok(())
}

struct SQLiteWorker {
    options: Option<OpenOptions>,
    db: Option<SQLiteDb>,
    opfs: Option<OpfsSAHPoolUtil>,
}

impl SQLiteWorker {
    fn new() -> Self {
        Self {
            options: None,
            db: None,
            opfs: None,
        }
    }

    async fn open(&mut self, options: OpenOptions) -> Result<()> {
        validate_filename(&options.filename).map_err(WorkerError::LoadDb)?;
        if self.options.as_ref().is_some_and(|current| {
            current.filename == options.filename && current.read_only == options.read_only
        }) && self.db.is_some()
        {
            self.options = Some(options);
            return Ok(());
        }
        self.ensure_autocommit()?;
        self.db = None;
        self.options = None;
        if self.opfs.is_none() {
            self.opfs = Some(
                sqlite_wasm_vfs::sahpool::install::<WasmOsCallback>(
                    &OpfsSAHPoolCfgBuilder::new()
                        .directory(OPFS_VFS_DIR)
                        .vfs_name("opfs")
                        .build(),
                    false,
                )
                .await
                .map_err(|_| WorkerError::OpfsSAHPoolOpened)?,
            );
        }
        self.opfs
            .as_ref()
            .unwrap()
            .resume()
            .await
            .map_err(|_| WorkerError::OpfsSAHPoolOpened)?;
        self.reserve_opfs(3).await?;
        if options.read_only {
            let exists = self
                .opfs
                .as_ref()
                .unwrap()
                .contains(&options.filename)
                .map_err(|_| WorkerError::Unexpected)?;
            if !exists {
                initialize_empty_db(&options)?;
            }
        }
        self.db = Some(SQLiteDb::open(&uri(&options), options.read_only)?);
        self.options = Some(options);
        Ok(())
    }

    fn reopen(&mut self) -> Result<()> {
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        self.db = Some(SQLiteDb::open(&uri(options), options.read_only)?);
        Ok(())
    }

    fn ensure_autocommit(&self) -> Result<()> {
        if self.db.as_ref().is_some_and(|db| !db.is_autocommit()) {
            return Err(WorkerError::ActiveTransaction);
        }
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.db = None;
        self.options = None;
        if let Some(opfs) = &self.opfs {
            opfs.pause().map_err(|_| WorkerError::Unexpected)?;
        }
        Ok(())
    }

    fn close_workspace(&mut self, workspace: &str) -> Result<()> {
        if self
            .options
            .as_ref()
            .is_some_and(|options| options.workspace == workspace)
        {
            self.close()?;
        }
        Ok(())
    }

    fn contains(&self, filename: &str) -> Result<bool> {
        self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        self.opfs
            .as_ref()
            .unwrap()
            .contains(filename)
            .map_err(|error| WorkerError::LoadDb(error.to_string()))
    }

    async fn reserve_opfs(&self, additional: usize) -> Result<()> {
        let opfs = self.opfs.as_ref().unwrap();
        let count = opfs
            .len()
            .map_err(|error| WorkerError::LoadDb(error.to_string()))?;
        opfs.ensure_capacity(count + additional)
            .await
            .map_err(|error| WorkerError::LoadDb(error.to_string()))
    }

    fn remove_files(&self, filename: &str) -> Result<()> {
        self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        for suffix in ["-journal", "-wal", "-shm", ""] {
            let name = format!("{filename}{suffix}");
            self.opfs
                .as_ref()
                .unwrap()
                .remove(&name)
                .map_err(|error| WorkerError::LoadDb(error.to_string()))?;
        }
        Ok(())
    }

    fn remove_db(&mut self) -> Result<()> {
        // SQLite must finish rollback/checkpoint work before its files are removed.
        self.db = None;
        let filename = self
            .options
            .as_ref()
            .ok_or(WorkerError::NotOpened)?
            .filename
            .clone();
        // PERSIST/TRUNCATE journals can survive closing the connection.
        self.remove_files(&filename)
    }

    async fn run(&mut self, options: RunOptions) -> Result<SQLiteRunResult> {
        let limit = self
            .db
            .as_ref()
            .ok_or(WorkerError::InvalidState)?
            .attachment_limit();
        // A script can create every attached database, their journals and
        // a super-journal before control returns to the worker.
        self.reserve_opfs(2 * limit + 3).await?;
        if options.clear_on_prepare {
            self.remove_db()?;
            let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
            if options.read_only {
                initialize_empty_db(options)?;
            }
            self.reopen()?;
        }
        let db = self.db.as_ref().ok_or(WorkerError::InvalidState)?;
        let mut result = Vec::new();
        let error = db.prepare(&options.sql)?.results(&mut result).err();
        Ok(SQLiteRunResult {
            embed: options.embed,
            result,
            error,
        })
    }

    async fn load_db(&mut self, data: LoadDbOptions) -> Result<()> {
        let filename = self
            .options
            .as_ref()
            .ok_or(WorkerError::NotOpened)?
            .filename
            .clone();
        self.load_dbs(LoadDbsOptions {
            files: vec![DatabaseFile {
                filename,
                data: data.data,
            }],
            replace: true,
        })
        .await
        .map(|_| ())
    }

    async fn load_dbs(&mut self, options: LoadDbsOptions) -> Result<Vec<String>> {
        self.ensure_autocommit()?;
        self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        let mut names = HashSet::new();
        let mut files = Vec::new();
        for file in options.files {
            validate_filename(&file.filename).map_err(WorkerError::LoadDb)?;
            if is_sidecar(&file.filename) {
                return Err(WorkerError::LoadDb(
                    "Upload main database files, without journal or WAL suffixes".into(),
                ));
            }
            if !names.insert(file.filename.clone()) {
                return Err(WorkerError::LoadDb(format!(
                    "Duplicate database name: {}",
                    file.filename
                )));
            }
            let data = file.data.to_vec();
            validate_import_db(&data)
                .map_err(|error| WorkerError::LoadDb(format!("{}: {error}", file.filename)))?;
            if !options.replace {
                for suffix in ["", "-journal", "-wal", "-shm"] {
                    if self.contains(&format!("{}{suffix}", file.filename))? {
                        return Err(WorkerError::LoadDb(format!(
                            "Database already exists: {}",
                            file.filename
                        )));
                    }
                }
            }
            files.push((file.filename, data));
        }
        self.reserve_opfs(files.len() + 3).await?;
        let reconnect = self.db.as_ref().is_some_and(|db| {
            files
                .iter()
                .any(|(filename, _)| db.uses_file(filename, OPFS_VFS))
        });
        if reconnect {
            self.db = None;
        }
        let result = (|| {
            for (filename, data) in &files {
                if options.replace {
                    self.remove_files(filename)?;
                }
                import_db(self.opfs.as_ref().unwrap(), filename, data)
                    .map_err(|error| WorkerError::LoadDb(error.to_string()))?;
            }
            Ok(files.into_iter().map(|(filename, _)| filename).collect())
        })();
        if reconnect {
            self.reopen()?;
        }
        result
    }

    async fn create_db(&self, filename: String) -> Result<String> {
        self.ensure_autocommit()?;
        validate_filename(&filename).map_err(WorkerError::CreateDb)?;
        if is_sidecar(&filename) {
            return Err(WorkerError::CreateDb(
                "Choose a main database filename, without journal or WAL suffixes".into(),
            ));
        }
        for suffix in ["", "-journal", "-wal", "-shm"] {
            if self.contains(&format!("{filename}{suffix}"))? {
                return Err(WorkerError::CreateDb(format!(
                    "Database already exists: {filename}"
                )));
            }
        }
        self.reserve_opfs(3).await?;
        let mut options = self.options.as_ref().ok_or(WorkerError::NotOpened)?.clone();
        options.filename = filename.clone();
        // The active connection keeps its key and SQL state while only the new file is opened.
        if let Err(error) = initialize_empty_db(&options) {
            self.remove_files(&filename)?;
            return Err(WorkerError::CreateDb(error.to_string()));
        }
        Ok(filename)
    }

    fn list_dbs(&self) -> Result<Vec<String>> {
        self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        let mut names = self
            .opfs
            .as_ref()
            .unwrap()
            .names()
            .map_err(|error| WorkerError::LoadDb(error.to_string()))?;
        names.retain(|name| !is_sidecar(name));
        names.sort();
        Ok(names)
    }

    fn download_db(&mut self) -> Result<DownloadDbResponse> {
        if let Some(db) = &self.db
            && !db.is_autocommit()
        {
            return Err(WorkerError::DownloadDb(
                "Commit or roll back the current transaction before downloading".into(),
            ));
        }
        if self
            .options
            .as_ref()
            .is_some_and(|options| options.read_only)
        {
            let filename = &self.options.as_ref().unwrap().filename;
            return self.download_raw(filename);
        }
        // VFS transfers require a closed database.
        self.db = None;
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        let filename = options.filename.clone();
        let data = export_db(self.opfs.as_ref().unwrap(), &filename);
        self.reopen()?;
        Ok(DownloadDbResponse {
            filename,
            data: Uint8Array::new_from_slice(&data?),
        })
    }

    fn download_file(&self, filename: &str) -> Result<DownloadDbResponse> {
        validate_filename(filename).map_err(WorkerError::DownloadDb)?;
        self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        if let Some(db) = &self.db
            && let Some((schema, read_only)) = db.file_schema(filename, OPFS_VFS)
        {
            if !db.is_autocommit() {
                return Err(WorkerError::DownloadDb(
                    "Commit or roll back the current transaction before downloading this database"
                        .into(),
                ));
            }
            if !read_only
                && self.contains(&format!("{filename}-wal"))?
                && ReadOnlyFile::open(&format!("{filename}-wal"))?.len()? != 0
            {
                db.checkpoint(&schema)?;
            }
        }
        self.download_raw(filename)
    }

    fn download_raw(&self, filename: &str) -> Result<DownloadDbResponse> {
        self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        // This worker executes one request at a time. Reading the base VFS keeps
        // the current SQLite connection and cipher state intact.
        for suffix in ["-journal", "-wal"] {
            let filename = format!("{filename}{suffix}");
            let exists = self
                .opfs
                .as_ref()
                .unwrap()
                .contains(&filename)
                .map_err(|error| WorkerError::DownloadDb(error.to_string()))?;
            if !exists {
                continue;
            }
            let file = ReadOnlyFile::open(&filename)?;
            if file.len()? != 0 && (suffix == "-wal" || file.first_byte()? != 0) {
                return Err(WorkerError::DownloadDb(
                    "The database has a journal or WAL that needs recovery before downloading a single file".into(),
                ));
            }
        }
        let file = ReadOnlyFile::open(filename)?;
        Ok(DownloadDbResponse {
            filename: filename.into(),
            data: Uint8Array::new_from_slice(&file.read_all()?),
        })
    }
}

fn export_db(vfs: &OpfsSAHPoolUtil, filename: &str) -> Result<Vec<u8>> {
    fn error(error: impl std::fmt::Display) -> WorkerError {
        WorkerError::DownloadDb(error.to_string())
    }

    for suffix in ["-journal", "-wal", "-shm"] {
        let sidecar = format!("{filename}{suffix}");
        if !vfs.contains(&sidecar).map_err(error)? {
            continue;
        }
        let mut source = vfs.begin_export(&sidecar).map_err(error)?;
        let mut first = [0];
        let length = source.read(&mut first).map_err(error)?;
        drop(source);
        // SQLite treats a zero journal header as cold. Preserve WAL data and
        // potentially hot journals instead of exporting an incomplete database.
        if length == 0 || (suffix == "-journal" && first[0] == 0) {
            vfs.remove(&sidecar).map_err(error)?;
        } else if suffix != "-shm" {
            return Err(WorkerError::DownloadDb(format!(
                "Database has a journal or WAL that needs recovery: {filename}"
            )));
        }
    }
    vfs.export_db(filename).map_err(error)
}

fn import_db(
    vfs: &OpfsSAHPoolUtil,
    filename: &str,
    db: &[u8],
) -> std::result::Result<(), OpfsSAHError> {
    #[cfg(not(any(feature = "sqlite3mc", feature = "sqlcipher")))]
    {
        vfs.import_db(filename, db)
    }
    #[cfg(any(feature = "sqlite3mc", feature = "sqlcipher"))]
    {
        vfs.import_db_unchecked(filename, db)
    }
}

fn validate_import_db(
    db: &[u8],
) -> std::result::Result<(), sqlite_wasm_rs::vfs::transfer::ImportDbError> {
    use sqlite_wasm_rs::vfs::transfer::ImportDbError;

    // Validate before replacing the current database.
    if db.len() < 512 || !db.len().is_multiple_of(512) {
        return Err(ImportDbError::InvalidDbSize);
    }
    #[cfg(not(any(feature = "sqlite3mc", feature = "sqlcipher")))]
    {
        if !db.starts_with(b"SQLite format 3\0") {
            return Err(ImportDbError::InvalidHeader);
        }
        let page_size = u16::from_be_bytes([db[16], db[17]]);
        let page_size = if page_size == 1 {
            65536
        } else {
            usize::from(page_size)
        };
        if !(512..=65536).contains(&page_size) || !page_size.is_power_of_two() {
            return Err(ImportDbError::InvalidPageSize);
        }
        if !db.len().is_multiple_of(page_size) {
            return Err(ImportDbError::InvalidDbSize);
        }
    }
    Ok(())
}

fn validate_filename(filename: &str) -> std::result::Result<(), String> {
    if filename.is_empty() || matches!(filename, "." | "..") || filename.contains(['\0', '/', '\\'])
    {
        return Err("Choose a nonempty database filename without directory separators".into());
    }
    Ok(())
}

fn is_sidecar(filename: &str) -> bool {
    ["-journal", "-wal", "-shm"]
        .iter()
        .any(|suffix| filename.ends_with(suffix))
}

async fn execute_tasks(
    scope: DedicatedWorkerGlobalScope,
    mut requests: mpsc::UnboundedReceiver<WorkerRequest>,
) {
    let mut worker = SQLiteWorker::new();
    post_response(&scope, WorkerResponse::Ready);
    while let Some(request) = requests.recv().await {
        let response = match request {
            WorkerRequest::Open(options) => WorkerResponse::Open(worker.open(options).await),
            WorkerRequest::Close => WorkerResponse::Close(worker.close()),
            WorkerRequest::CloseWorkspace(workspace) => {
                WorkerResponse::CloseWorkspace(worker.close_workspace(&workspace))
            }
            WorkerRequest::Run(options) => WorkerResponse::Run(worker.run(options).await),
            WorkerRequest::LoadDb(options) => WorkerResponse::LoadDb(worker.load_db(options).await),
            WorkerRequest::LoadDbs(options) => {
                WorkerResponse::LoadDbs(worker.load_dbs(options).await)
            }
            WorkerRequest::CreateDb(filename) => {
                WorkerResponse::CreateDb(worker.create_db(filename).await)
            }
            WorkerRequest::ListDbs => WorkerResponse::ListDbs(worker.list_dbs()),
            WorkerRequest::DownloadDb => WorkerResponse::DownloadDb(worker.download_db()),
            WorkerRequest::DownloadFile(filename) => {
                WorkerResponse::DownloadDb(worker.download_file(&filename))
            }
        };
        post_response(&scope, response);
    }
}

fn post_response(scope: &DedicatedWorkerGlobalScope, response: WorkerResponse) {
    let message = serde_wasm_bindgen::to_value(&response).unwrap();
    if let Err(err) = scope.post_message(&message) {
        log::error!("Failed to send worker response: {err:?}");
    }
}

pub fn entry() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();
    let scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    let (tx, rx) = mpsc::unbounded_channel();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        match serde_wasm_bindgen::from_value(event.data()) {
            Ok(request) => {
                let _ = tx.send(request);
            }
            Err(err) => log::error!("Invalid worker request: {err}"),
        }
    });
    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    // The dedicated worker owns this callback for its entire lifetime.
    on_message.forget();
    spawn_local(execute_tasks(scope, rx));
}
