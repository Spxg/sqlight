mod sqlitend;

use crate::{
    DownloadDbResponse, LoadDbOptions, OpenOptions, RunOptions, SQLiteRunResult, WorkerError,
    WorkerRequest, WorkerResponse,
};
use js_sys::Uint8Array;
use once_cell::sync::Lazy;
use sqlite_wasm_rs::WasmOsCallback;
use sqlite_wasm_rs::vfs::{VfsFilesManager, memvfs::MemVfsUtil, transfer::DbTransfer};
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil};

use sqlitend::SQLiteDb;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;
use tokio::sync::mpsc::UnboundedReceiver;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

type Result<T> = std::result::Result<T, WorkerError>;

static DB: Lazy<Mutex<Option<SQLiteWorker>>> = Lazy::new(|| Mutex::new(None));

static FS_UTIL: Lazy<FSUtil> = Lazy::new(|| FSUtil {
    // All SQLite access is serialized on this dedicated worker.
    mem: unsafe { MemVfsUtil::get().expect("Failed to initialize memory VFS") },
    opfs: OnceCell::new(),
});

#[cfg(feature = "sqlite3")]
const OPFS_VFS: &str = "opfs";
#[cfg(feature = "sqlite3mc")]
const OPFS_VFS: &str = "multipleciphers-opfs";

#[cfg(feature = "sqlite3")]
const OPFS_VFS_DIR: &str = "sqlight-sahpool";
#[cfg(feature = "sqlite3mc")]
const OPFS_VFS_DIR: &str = "sqlight-sahpool-mc";

#[cfg(feature = "sqlite3")]
const MEM_VFS: &str = "memvfs";
#[cfg(feature = "sqlite3mc")]
const MEM_VFS: &str = "multipleciphers-memvfs";

fn uri(filename: &str, persist: bool) -> String {
    format!(
        "file:{}?vfs={}",
        filename,
        if persist { OPFS_VFS } else { MEM_VFS }
    )
}

struct FSUtil {
    mem: MemVfsUtil,
    opfs: OnceCell<OpfsSAHPoolUtil>,
}

unsafe impl Send for FSUtil {}
unsafe impl Sync for FSUtil {}

struct SQLiteWorker {
    open_options: OpenOptions,
    state: SQLiteState,
}

enum SQLiteState {
    NotOpened,
    Opened(Arc<SQLiteDb>),
}

async fn with_worker<F, T>(mut f: F) -> Result<T>
where
    F: FnMut(&mut SQLiteWorker) -> Result<T>,
{
    f(DB.lock().await.as_mut().ok_or(WorkerError::NotOpened)?)
}

async fn init_opfs_util() -> Result<&'static OpfsSAHPoolUtil> {
    FS_UTIL
        .opfs
        .get_or_try_init(|| async {
            sqlite_wasm_vfs::sahpool::install::<WasmOsCallback>(
                &OpfsSAHPoolCfgBuilder::new()
                    .directory(OPFS_VFS_DIR)
                    .vfs_name("opfs")
                    .build(),
                false,
            )
            .await
            .map_err(|_| WorkerError::OpfsSAHPoolOpened)
        })
        .await
}

fn get_opfs_util() -> Result<&'static OpfsSAHPoolUtil> {
    FS_UTIL.opfs.get().ok_or(WorkerError::Unexpected)
}

async fn download_db() -> Result<DownloadDbResponse> {
    with_worker(|worker| {
        if let SQLiteState::Opened(db) = &worker.state {
            if !db.is_autocommit() {
                return Err(WorkerError::DownloadDb(
                    "Commit or roll back the current transaction before downloading".into(),
                ));
            }
        }
        // VFS transfers require a closed database. Reopen even if export fails.
        let was_open = matches!(worker.state, SQLiteState::Opened(_));
        drop(std::mem::replace(&mut worker.state, SQLiteState::NotOpened));
        let filename = &worker.open_options.filename;
        let db = if worker.open_options.persist {
            get_opfs_util()?
                .export_db(filename)
                .map_err(|err| WorkerError::DownloadDb(format!("{err}")))
        } else {
            let mem_vfs = &FS_UTIL.mem;
            mem_vfs
                .export_db(filename)
                .map_err(|err| WorkerError::DownloadDb(format!("{err}")))
        };
        if was_open {
            worker.state =
                SQLiteState::Opened(SQLiteDb::open(&uri(filename, worker.open_options.persist))?);
        }
        Ok(DownloadDbResponse {
            filename: worker.open_options.filename.clone(),
            data: Uint8Array::new_from_slice(&db?),
        })
    })
    .await
}

async fn load_db(options: LoadDbOptions) -> Result<()> {
    let db = options.data.to_vec();

    #[cfg(feature = "sqlite3")]
    validate_import_db(&db).map_err(|err| WorkerError::LoadDb(format!("{err}")))?;

    with_worker(|worker| {
        drop(std::mem::replace(&mut worker.state, SQLiteState::NotOpened));

        let filename = &worker.open_options.filename;
        if worker.open_options.persist {
            let opfs = get_opfs_util()?;
            opfs.remove(filename).map_err(|_| WorkerError::Unexpected)?;

            if let Err(err) = import_db(opfs, filename, &db) {
                return Err(WorkerError::LoadDb(format!("{err}")));
            }
        } else {
            let mem_vfs = &FS_UTIL.mem;
            mem_vfs.remove(filename).unwrap();
            if let Err(err) = import_db(mem_vfs, filename, &db) {
                return Err(WorkerError::LoadDb(format!("{err}")));
            }
        }

        worker.state = SQLiteState::Opened(SQLiteDb::open(&uri(
            &worker.open_options.filename,
            worker.open_options.persist,
        ))?);
        Ok(())
    })
    .await
}

fn import_db<T: DbTransfer>(
    vfs: &T,
    filename: &str,
    db: &[u8],
) -> std::result::Result<(), T::Error> {
    #[cfg(feature = "sqlite3")]
    {
        vfs.import_db(filename, db)
    }

    #[cfg(feature = "sqlite3mc")]
    {
        vfs.import_db_unchecked(filename, db)
    }
}

#[cfg(feature = "sqlite3")]
fn validate_import_db(
    db: &[u8],
) -> std::result::Result<(), sqlite_wasm_rs::vfs::transfer::ImportDbError> {
    use sqlite_wasm_rs::vfs::transfer::ImportDbError;

    // Validate before removing the current database. The checked transfer also
    // validates, but only after we have freed the destination filename.
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

async fn open(options: OpenOptions) -> Result<()> {
    let mut locker = DB.lock().await;
    locker.take();

    if options.persist {
        init_opfs_util().await?;
    }

    let state = SQLiteState::Opened(SQLiteDb::open(&uri(&options.filename, options.persist))?);
    let worker = SQLiteWorker {
        open_options: options,
        state,
    };
    *locker = Some(worker);
    Ok(())
}

async fn run(options: RunOptions) -> Result<SQLiteRunResult> {
    with_worker(|worker| {
        if options.clear_on_prepare {
            drop(std::mem::replace(&mut worker.state, SQLiteState::NotOpened));

            let filename = &worker.open_options.filename;
            if worker.open_options.persist {
                get_opfs_util()?
                    .remove(filename)
                    .map_err(|_| WorkerError::Unexpected)?;
            } else {
                let mem_vfs = &FS_UTIL.mem;
                mem_vfs.remove(filename).unwrap();
            }

            worker.state = SQLiteState::Opened(SQLiteDb::open(&uri(
                &worker.open_options.filename,
                worker.open_options.persist,
            ))?);
        }
        match &worker.state {
            SQLiteState::NotOpened => Err(WorkerError::InvaildState),
            SQLiteState::Opened(sqlite_db) => {
                let stmts = sqlite_db.prepare(&options.sql)?;
                let result = stmts.stmts_result()?;
                Ok(SQLiteRunResult {
                    embed: options.embed,
                    result,
                })
            }
        }
    })
    .await
}

async fn execute_task(scope: DedicatedWorkerGlobalScope, mut rx: UnboundedReceiver<JsValue>) {
    while let Some(request) = rx.recv().await {
        let request = serde_wasm_bindgen::from_value::<WorkerRequest>(request).unwrap();
        let resp = match request {
            WorkerRequest::Open(options) => WorkerResponse::Open(open(options).await),
            WorkerRequest::Run(options) => WorkerResponse::Run(run(options).await),
            WorkerRequest::LoadDb(options) => WorkerResponse::LoadDb(load_db(options).await),
            WorkerRequest::DownloadDb => WorkerResponse::DownloadDb(download_db().await),
        };
        if let Err(err) = scope.post_message(&serde_wasm_bindgen::to_value(&resp).unwrap()) {
            log::error!("Failed to send task to window: {resp:?}, {err:?}");
        }
    }
}

pub fn entry() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<JsValue>();

    let scope: DedicatedWorkerGlobalScope = JsValue::from(js_sys::global()).into();
    spawn_local(execute_task(scope.clone(), rx));

    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |ev: MessageEvent| {
        tx.send(ev.data()).unwrap();
    });

    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    scope
        .post_message(&serde_wasm_bindgen::to_value(&WorkerResponse::Ready).unwrap())
        .expect("Faild to send ready to window");
    on_message.forget();
}
