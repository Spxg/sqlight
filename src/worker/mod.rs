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

#[cfg(not(feature = "sqlite3mc"))]
const OPFS_VFS_DIR: &str = "sqlight-sahpool";
#[cfg(feature = "sqlite3mc")]
const OPFS_VFS_DIR: &str = "sqlight-sahpool-mc";

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
        self.db = Some(SQLiteDb::open(&uri(&options))?);
        self.options = Some(options);
        Ok(())
    }

    fn reopen(&mut self) -> Result<()> {
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        self.db = Some(SQLiteDb::open(&uri(options))?);
        Ok(())
    }

    fn remove_db(&self) -> Result<()> {
        let options = self.options.as_ref().ok_or(WorkerError::NotOpened)?;
        if options.persist {
            self.opfs
                .as_ref()
                .unwrap()
                .remove(&options.filename)
                .map_err(|_| WorkerError::Unexpected)?;
        } else {
            self.memory.remove(&options.filename).unwrap();
        }
        Ok(())
    }

    fn run(&mut self, options: RunOptions) -> Result<SQLiteRunResult> {
        if options.clear_on_prepare {
            self.db = None;
            self.remove_db()?;
            self.reopen()?;
        }
        let db = self.db.as_ref().ok_or(WorkerError::InvalidState)?;
        let result = db.prepare(&options.sql)?.results()?;
        Ok(SQLiteRunResult {
            embed: options.embed,
            result,
        })
    }

    fn load_db(&mut self, data: LoadDbOptions) -> Result<()> {
        let data = data.data.to_vec();
        #[cfg(not(feature = "sqlite3mc"))]
        validate_import_db(&data).map_err(|err| WorkerError::LoadDb(err.to_string()))?;

        self.db = None;
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
            self.opfs
                .as_ref()
                .unwrap()
                .export_db(&filename)
                .map_err(|err| WorkerError::DownloadDb(err.to_string()))
        } else {
            self.memory
                .export_db(&filename)
                .map_err(|err| WorkerError::DownloadDb(err.to_string()))
        };
        self.reopen()?;
        Ok(DownloadDbResponse {
            filename,
            data: Uint8Array::new_from_slice(&data?),
        })
    }
}

fn import_db<T: DbTransfer>(
    vfs: &T,
    filename: &str,
    db: &[u8],
) -> std::result::Result<(), T::Error> {
    #[cfg(not(feature = "sqlite3mc"))]
    {
        vfs.import_db(filename, db)
    }
    #[cfg(feature = "sqlite3mc")]
    {
        vfs.import_db_unchecked(filename, db)
    }
}

#[cfg(not(feature = "sqlite3mc"))]
fn validate_import_db(
    db: &[u8],
) -> std::result::Result<(), sqlite_wasm_rs::vfs::transfer::ImportDbError> {
    use sqlite_wasm_rs::vfs::transfer::ImportDbError;

    // Validate before replacing the current database.
    if db.len() < 512 || db.len() % 512 != 0 {
        return Err(ImportDbError::InvalidDbSize);
    }
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
    if db.len() % page_size != 0 {
        return Err(ImportDbError::InvalidDbSize);
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
