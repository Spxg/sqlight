mod raw_file;
mod sqlitend;

use std::collections::HashSet;

use crate::{
    DatabaseFile, DownloadBundleResponse, DownloadDbResponse, FileInfo, FileSnapshot,
    LoadDbOptions, LoadDbsOptions, OpenOptions, RunOptions, SQLiteRunResult, WorkerError,
    WorkerRequest, WorkerResponse,
};
use js_sys::Uint8Array;
use sqlite_wasm_rs::{
    WasmOsCallback,
    vfs::{VfsFilesManager, transfer::DbTransfer},
};
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil};
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

const OPFS_VFS_DIR: &str = "sqlight-sahpool";

const MAX_DATABASE_FILENAME_BYTES: usize = 499;
const MAX_FILE_FILENAME_BYTES: usize = 511;

fn uri(filename: &str) -> String {
    let filename = String::from(js_sys::encode_uri_component(filename));
    format!("file:{filename}?vfs={OPFS_VFS}")
}

fn initialize_empty_db(filename: &str) -> Result<()> {
    let db = SQLiteDb::open(&uri(filename), false)?;
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
        validate_filename(&options.filename, MAX_DATABASE_FILENAME_BYTES)
            .map_err(WorkerError::LoadDb)?;
        if self.options.as_ref().is_some_and(|current| {
            current.filename == options.filename && current.read_only == options.read_only
        }) && self.db.is_some()
        {
            self.options = Some(options);
            return Ok(());
        }
        self.ensure_autocommit()?;
        self.ensure_fs().await?;
        self.reserve_opfs(3).await?;
        if !self.contains(&options.filename)? {
            for suffix in ["-journal", "-wal", "-shm"] {
                if self.contains(&format!("{}{suffix}", options.filename))? {
                    return Err(WorkerError::CreateDb(format!(
                        "A log file already exists for {}; restore its main database before opening it",
                        options.filename
                    )));
                }
            }
            self.initialize_main(&options)?;
        }
        self.db = None;
        self.options = None;
        self.db = Some(SQLiteDb::open(&uri(&options.filename), options.read_only)?);
        self.options = Some(options);
        Ok(())
    }

    async fn ensure_fs(&mut self) -> Result<()> {
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
        Ok(())
    }

    fn reopen(&mut self) -> Result<()> {
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        self.db = Some(SQLiteDb::open(&uri(&options.filename), options.read_only)?);
        Ok(())
    }

    fn initialize_main(&self, options: &OpenOptions) -> Result<()> {
        if options.read_only {
            initialize_empty_db(&options.filename)
        } else {
            self.opfs
                .as_ref()
                .unwrap()
                .import_db_unchecked(&options.filename, &[])
                .map_err(|error| WorkerError::Filesystem(error.to_string()))
        }
    }

    fn ensure_autocommit(&self) -> Result<()> {
        if self.db.as_ref().is_some_and(|db| !db.is_autocommit()) {
            return Err(WorkerError::ActiveTransaction);
        }
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.ensure_autocommit()?;
        self.db = None;
        self.options = None;
        if let Some(opfs) = &self.opfs {
            opfs.pause().map_err(|_| WorkerError::Unexpected)?;
        }
        Ok(())
    }

    fn contains(&self, filename: &str) -> Result<bool> {
        self.opfs
            .as_ref()
            .unwrap()
            .contains(filename)
            .map_err(|error| WorkerError::Filesystem(error.to_string()))
    }

    async fn reserve_opfs(&self, additional: usize) -> Result<()> {
        let opfs = self.opfs.as_ref().unwrap();
        let count = opfs
            .len()
            .map_err(|error| WorkerError::Filesystem(error.to_string()))?;
        opfs.ensure_capacity(count + additional)
            .await
            .map_err(|error| WorkerError::Filesystem(error.to_string()))
    }

    fn remove_files(&self, filename: &str) -> Result<()> {
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
            self.initialize_main(options)?;
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
        self.ensure_fs().await?;
        self.ensure_autocommit()?;
        let mut names = HashSet::new();
        let mut databases = Vec::new();
        let mut files = Vec::new();
        for file in options.files {
            // The transfer API applies the main-database name limit to every
            // imported file, including journals and WAL files.
            validate_filename(&file.filename, MAX_DATABASE_FILENAME_BYTES)
                .map_err(WorkerError::LoadDb)?;
            if !names.insert(file.filename.clone()) {
                return Err(WorkerError::LoadDb(format!(
                    "Duplicate database name: {}",
                    file.filename
                )));
            }
            let data = file.data.to_vec();
            if !is_sidecar(&file.filename) {
                validate_import_db(&data)
                    .map_err(|error| WorkerError::LoadDb(format!("{}: {error}", file.filename)))?;
                databases.push(file.filename.clone());
            } else if file.filename.ends_with("-journal") && has_super_journal(&data) {
                return Err(WorkerError::LoadDb(
                    "This journal references a super-journal from a multi-database transaction; \
                    import a recovered database instead"
                        .into(),
                ));
            }
            files.push((file.filename, data));
        }
        for (filename, _) in &files {
            if let Some(database) = sidecar_database(filename)
                && !databases.iter().any(|name| name == database)
            {
                return Err(WorkerError::LoadDb(format!(
                    "Missing main database for sidecar: {filename}"
                )));
            }
        }
        for database in &databases {
            if !options.replace {
                for suffix in ["", "-journal", "-wal", "-shm"] {
                    if self.contains(&format!("{database}{suffix}"))? {
                        return Err(WorkerError::LoadDb(format!(
                            "Database already exists: {database}"
                        )));
                    }
                }
            }
        }
        self.reserve_opfs(files.len() + 3).await?;
        let reconnect = self.db.as_ref().is_some_and(|db| {
            options.replace
                && databases.iter().any(|database| {
                    ["", "-journal", "-wal", "-shm"]
                        .iter()
                        .any(|suffix| db.uses_file(&format!("{database}{suffix}"), OPFS_VFS))
                })
        });
        if reconnect {
            self.db = None;
        }
        let result = (|| {
            if options.replace {
                for database in &databases {
                    self.remove_files(database)?;
                }
            }
            // Importing a main file after its sidecars is rejected by the VFS.
            files.sort_by_key(|(filename, _)| is_sidecar(filename));
            for (filename, data) in &files {
                self.opfs
                    .as_ref()
                    .unwrap()
                    .import_db_unchecked(filename, data)
                    .map_err(|error| WorkerError::LoadDb(error.to_string()))?;
            }
            Ok(databases)
        })();
        if reconnect {
            self.reopen()?;
        }
        result
    }

    async fn create_db(&mut self, filename: String) -> Result<String> {
        self.ensure_fs().await?;
        validate_filename(&filename, MAX_DATABASE_FILENAME_BYTES).map_err(WorkerError::CreateDb)?;
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
        // Leave the first page for the selected backend's SQL (and optional key).
        // Creating this separate file does not change the active connection.
        self.opfs
            .as_ref()
            .unwrap()
            .import_db_unchecked(&filename, &[])
            .map_err(|error| WorkerError::CreateDb(error.to_string()))?;
        Ok(filename)
    }

    fn file_in_use(&self, filename: &str) -> bool {
        self.db.as_ref().is_some_and(|db| {
            db.uses_file(filename, OPFS_VFS)
                || sidecar_database(filename).is_some_and(|main| db.uses_file(main, OPFS_VFS))
        })
    }

    async fn list_files(&mut self) -> Result<FileSnapshot> {
        self.ensure_fs().await?;
        let mut names = self
            .opfs
            .as_ref()
            .unwrap()
            .names()
            .map_err(|error| WorkerError::Filesystem(error.to_string()))?;
        names.sort();
        let mut files = Vec::new();
        for filename in names {
            files.push(FileInfo {
                size: ReadOnlyFile::open(&filename)?.len()?,
                in_use: self.file_in_use(&filename),
                filename,
            });
        }
        Ok(FileSnapshot {
            files,
            connection: if self.db.is_some() {
                self.options.clone()
            } else {
                None
            },
            in_transaction: self.db.as_ref().is_some_and(|db| !db.is_autocommit()),
        })
    }

    async fn delete_files(&mut self, filenames: Vec<String>) -> Result<Vec<String>> {
        self.ensure_fs().await?;
        self.ensure_autocommit()?;
        let mut selected = HashSet::new();
        for filename in filenames {
            validate_filename(&filename, MAX_FILE_FILENAME_BYTES)
                .map_err(WorkerError::Filesystem)?;
            if !self.contains(&filename)? {
                return Err(WorkerError::MissingDatabase(filename));
            }
            if let Some(main) = sidecar_database(&filename)
                && !self.contains(main)?
            {
                selected.insert(filename);
                continue;
            }
            let main = sidecar_database(&filename).unwrap_or(&filename);
            for suffix in ["", "-journal", "-wal", "-shm"] {
                let name = format!("{main}{suffix}");
                if self.contains(&name)? {
                    selected.insert(name);
                }
            }
        }
        let mut selected: Vec<_> = selected.into_iter().collect();
        selected.sort();
        let in_use: Vec<_> = selected
            .iter()
            .filter(|name| self.file_in_use(name))
            .cloned()
            .collect();
        if !in_use.is_empty() {
            return Err(WorkerError::FilesInUse(in_use));
        }
        // Remove main files first, leaving only orphan logs if cleanup fails.
        selected.sort_by_key(|name| is_sidecar(name));
        for filename in &selected {
            self.opfs
                .as_ref()
                .unwrap()
                .remove(filename)
                .map_err(|error| WorkerError::Filesystem(error.to_string()))?;
        }
        Ok(selected)
    }

    async fn list_dbs(&mut self) -> Result<Vec<String>> {
        self.ensure_fs().await?;
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

    async fn download_db(&mut self) -> Result<DownloadDbResponse> {
        let filename = self
            .options
            .as_ref()
            .ok_or(WorkerError::NotOpened)?
            .filename
            .clone();
        self.download_file(&filename).await
    }

    async fn download_file(&mut self, filename: &str) -> Result<DownloadDbResponse> {
        self.ensure_fs().await?;
        validate_filename(filename, MAX_FILE_FILENAME_BYTES).map_err(WorkerError::DownloadDb)?;
        if !self.contains(filename)? {
            return Err(WorkerError::MissingDatabase(filename.into()));
        }
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

    async fn download_bundle(&mut self, databases: Vec<String>) -> Result<DownloadBundleResponse> {
        self.ensure_fs().await?;
        if databases.is_empty() {
            return Err(WorkerError::DownloadDb(
                "Choose at least one database".into(),
            ));
        }
        let mut selected = HashSet::new();
        let mut names = Vec::new();
        for database in &databases {
            validate_filename(database, MAX_FILE_FILENAME_BYTES)
                .map_err(WorkerError::DownloadDb)?;
            if !self.contains(database)? {
                return Err(WorkerError::MissingDatabase(database.clone()));
            }
            if !selected.insert(database) {
                return Err(WorkerError::DownloadDb("Choose each file once".into()));
            }
            if let Some(main) = sidecar_database(database) {
                if !self.contains(main)? {
                    names.push(database.clone());
                    continue;
                }
                return Err(WorkerError::DownloadDb(
                    "Choose the main database to include its log files".into(),
                ));
            }
            names.push(database.clone());
            for suffix in ["-journal", "-wal", "-shm"] {
                let filename = format!("{database}{suffix}");
                if self.contains(&filename)? {
                    names.push(filename);
                }
            }
        }
        if let Some(db) = &self.db
            && !db.is_autocommit()
            && names.iter().any(|name| db.uses_file(name, OPFS_VFS))
        {
            return Err(WorkerError::DownloadDb(
                "Commit or roll back the current transaction before downloading these databases"
                    .into(),
            ));
        }
        let mut files = Vec::new();
        // The worker runs requests serially and owns the OPFS pool exclusively.
        // Copy every file synchronously so no transaction can change this bundle.
        for filename in names {
            let data = ReadOnlyFile::open(&filename)?.read_all()?;
            if filename.ends_with("-journal") && has_super_journal(&data) {
                return Err(WorkerError::DownloadDb(
                    "This journal references a super-journal from a multi-database transaction; \
                    recover the database before downloading"
                        .into(),
                ));
            }
            files.push(DatabaseFile {
                filename,
                data: Uint8Array::new_from_slice(&data),
            });
        }
        Ok(DownloadBundleResponse {
            filename: if databases.len() == 1 {
                format!("{}.zip", databases[0])
            } else {
                "databases.zip".into()
            },
            files,
        })
    }
}

fn validate_import_db(
    db: &[u8],
) -> std::result::Result<(), sqlite_wasm_rs::vfs::transfer::ImportDbError> {
    use sqlite_wasm_rs::vfs::transfer::ImportDbError;

    // SQLite creates a zero-byte file until its first database write.
    if db.is_empty() {
        return Ok(());
    }
    // Files may use another backend's encryption, so only check their size.
    if db.len() < 512 || !db.len().is_multiple_of(512) {
        return Err(ImportDbError::InvalidDbSize);
    }
    Ok(())
}

fn validate_filename(filename: &str, max_bytes: usize) -> std::result::Result<(), String> {
    if filename.is_empty() || matches!(filename, "." | "..") || filename.contains(['\0', '/', '\\'])
    {
        return Err("Choose a nonempty database filename without directory separators".into());
    }
    if filename.len() > max_bytes {
        return Err(format!("Filename exceeds {max_bytes} UTF-8 bytes"));
    }
    Ok(())
}

fn is_sidecar(filename: &str) -> bool {
    sidecar_database(filename).is_some()
}

fn sidecar_database(filename: &str) -> Option<&str> {
    ["-journal", "-wal", "-shm"]
        .iter()
        .find_map(|suffix| filename.strip_suffix(suffix))
}

fn has_super_journal(data: &[u8]) -> bool {
    const MAGIC: &[u8] = &[0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];
    if data.len() < 16 || data.first() == Some(&0) || !data.ends_with(MAGIC) {
        return false;
    }
    let footer = data.len() - 16;
    let length = u32::from_be_bytes(data[footer..footer + 4].try_into().unwrap()) as usize;
    if length == 0 || length > footer {
        return false;
    }
    let checksum = u32::from_be_bytes(data[footer + 4..footer + 8].try_into().unwrap());
    data[footer - length..footer]
        .iter()
        // SQLite's journal checksum treats filename bytes as signed octets.
        .fold(0u32, |sum, byte| {
            sum.wrapping_add(i32::from(*byte as i8) as u32)
        })
        == checksum
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
            WorkerRequest::Run(options) => WorkerResponse::Run(worker.run(options).await),
            WorkerRequest::LoadDb(options) => WorkerResponse::LoadDb(worker.load_db(options).await),
            WorkerRequest::LoadDbs(options) => {
                WorkerResponse::LoadDbs(worker.load_dbs(options).await)
            }
            WorkerRequest::CreateDb(filename) => {
                WorkerResponse::CreateDb(worker.create_db(filename).await)
            }
            WorkerRequest::ListDbs => WorkerResponse::ListDbs(worker.list_dbs().await),
            WorkerRequest::ListFiles => WorkerResponse::ListFiles(worker.list_files().await),
            WorkerRequest::DeleteFiles(filenames) => {
                WorkerResponse::DeleteFiles(worker.delete_files(filenames).await)
            }
            WorkerRequest::DownloadDb => WorkerResponse::DownloadDb(worker.download_db().await),
            WorkerRequest::DownloadFile(filename) => {
                WorkerResponse::DownloadDb(worker.download_file(&filename).await)
            }
            WorkerRequest::DownloadBundle(filenames) => {
                WorkerResponse::DownloadBundle(worker.download_bundle(filenames).await)
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
