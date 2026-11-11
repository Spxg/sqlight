mod sqlitend;

use crate::{
    DownloadDbResponse, LoadDbOptions, OpenOptions, RunOptions, SQLiteRunResult, WorkerError,
    WorkerRequest, WorkerResponse,
};
use js_sys::Uint8Array;
use sqlite_wasm_rs::{
    WasmOsCallback,
    vfs::{VfsFilesManager, memvfs::MemVfsUtil, transfer::DbTransfer},
};
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil};
use tokio::sync::mpsc;
use wasm_bindgen::{JsCast, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

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

#[cfg(not(feature = "sqlite3mc"))]
const MEM_VFS: &str = "memvfs";
#[cfg(feature = "sqlite3mc")]
const MEM_VFS: &str = "multipleciphers-memvfs";

fn uri(options: &OpenOptions) -> String {
    let vfs = if options.persist { OPFS_VFS } else { MEM_VFS };
    format!("file:{}?vfs={vfs}", options.filename)
}

struct SQLiteWorker {
    options: Option<OpenOptions>,
    db: Option<SQLiteDb>,
    memory: MemVfsUtil,
    opfs: Option<OpfsSAHPoolUtil>,
}

impl SQLiteWorker {
    fn new() -> Self {
        Self {
            options: None,
            db: None,
            // This worker serializes all access to the SQLite VFS.
            memory: unsafe { MemVfsUtil::get().expect("Failed to initialize memory VFS") },
            opfs: None,
        }
    }

    async fn open(&mut self, options: OpenOptions) -> Result<()> {
        self.db = None;
        self.options = None;
        if options.persist && self.opfs.is_none() {
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
        if let Some(opfs) = &self.opfs {
            if options.persist {
                opfs.resume()
                    .await
                    .map_err(|_| WorkerError::OpfsSAHPoolOpened)?;
            } else {
                opfs.pause().map_err(|_| WorkerError::Unexpected)?;
            }
        }
        self.db = Some(SQLiteDb::open(&uri(&options))?);
        self.options = Some(options);
        Ok(())
    }

    fn reopen(&mut self) -> Result<()> {
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        self.db = Some(SQLiteDb::open(&uri(options))?);
        Ok(())
    }

    fn remove_db(&mut self) -> Result<()> {
        // SQLite must finish rollback/checkpoint work before its files are removed.
        self.db = None;
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        // PERSIST/TRUNCATE journals can survive closing the connection.
        for suffix in ["-journal", "-wal", "-shm", ""] {
            let filename = format!("{}{suffix}", options.filename);
            if options.persist {
                self.opfs
                    .as_ref()
                    .unwrap()
                    .remove(&filename)
                    .map_err(|_| WorkerError::Unexpected)?;
            } else {
                self.memory.remove(&filename).unwrap();
            }
        }
        Ok(())
    }

    fn run(&mut self, options: RunOptions) -> Result<SQLiteRunResult> {
        if options.clear_on_prepare {
            self.remove_db()?;
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

    fn load_db(&mut self, data: LoadDbOptions) -> Result<()> {
        let data = data.data.to_vec();
        validate_import_db(&data).map_err(|err| WorkerError::LoadDb(err.to_string()))?;

        self.remove_db()?;
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        let result = if options.persist {
            import_db(self.opfs.as_ref().unwrap(), &options.filename, &data)
                .map_err(|err| WorkerError::LoadDb(err.to_string()))
        } else {
            import_db(&self.memory, &options.filename, &data)
                .map_err(|err| WorkerError::LoadDb(err.to_string()))
        };
        // Keep the worker usable even if an import fails after validation.
        self.reopen()?;
        result
    }

    fn download_db(&mut self) -> Result<DownloadDbResponse> {
        if let Some(db) = &self.db
            && !db.is_autocommit()
        {
            return Err(WorkerError::DownloadDb(
                "Commit or roll back the current transaction before downloading".into(),
            ));
        }
        // VFS transfers require a closed database.
        self.db = None;
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        let filename = options.filename.clone();
        let data = if options.persist {
            export_db(self.opfs.as_ref().unwrap(), &filename)
        } else {
            export_db(&self.memory, &filename)
        };
        self.reopen()?;
        Ok(DownloadDbResponse {
            filename,
            data: Uint8Array::new_from_slice(&data?),
        })
    }
}

fn export_db<T>(vfs: &T, filename: &str) -> Result<Vec<u8>>
where
    T: DbTransfer + VfsFilesManager,
    <T as DbTransfer>::Error: std::fmt::Display,
    <T as VfsFilesManager>::Error: std::fmt::Display,
{
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

fn import_db<T: DbTransfer>(
    vfs: &T,
    filename: &str,
    db: &[u8],
) -> std::result::Result<(), T::Error> {
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

async fn execute_tasks(
    scope: DedicatedWorkerGlobalScope,
    mut requests: mpsc::UnboundedReceiver<WorkerRequest>,
) {
    let mut worker = SQLiteWorker::new();
    post_response(&scope, WorkerResponse::Ready);
    while let Some(request) = requests.recv().await {
        let response = match request {
            WorkerRequest::Open(options) => WorkerResponse::Open(worker.open(options).await),
            WorkerRequest::Run(options) => WorkerResponse::Run(worker.run(options)),
            WorkerRequest::LoadDb(options) => WorkerResponse::LoadDb(worker.load_db(options)),
            WorkerRequest::DownloadDb => WorkerResponse::DownloadDb(worker.download_db()),
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
