mod sqlitend;

use crate::{
    DownloadDbResponse, LoadDbOptions, OpenOptions, PERSIST_VFS, RunOptions, SQLiteRunResult,
    WorkerError,
};
use once_cell::sync::Lazy;
use sqlite_wasm_rs::{
    export::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil},
    mem_vfs::MemVfsUtil,
    utils::{copy_to_uint8_array, copy_to_vec},
};
use sqlitend::SQLiteDb;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;

type Result<T> = std::result::Result<T, WorkerError>;

static DB: Lazy<Mutex<Option<SQLiteWorker>>> = Lazy::new(|| Mutex::new(None));

static FS_UTIL: Lazy<FSUtil> = Lazy::new(|| FSUtil {
    mem: MemVfsUtil::new(),
    opfs: OnceCell::new(),
});

struct FSUtil {
    mem: MemVfsUtil,
    opfs: OnceCell<OpfsSAHPoolUtil>,
}

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
            sqlite_wasm_rs::sahpool_vfs::install(
                Some(
                    &OpfsSAHPoolCfgBuilder::new()
                        .directory(PERSIST_VFS)
                        .vfs_name(PERSIST_VFS)
                        .build(),
                ),
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

pub async fn download_db() -> Result<DownloadDbResponse> {
    with_worker(|worker| {
        let filename = &worker.open_options.filename;
        let db = if worker.open_options.persist {
            get_opfs_util()?
                .export_file(filename)
                .map_err(|err| WorkerError::DownloadDb(format!("{err}")))?
        } else {
            let mem_vfs = &FS_UTIL.mem;
            mem_vfs
                .export_db(filename)
                .map_err(|err| WorkerError::DownloadDb(format!("{err}")))?
        };
        Ok(DownloadDbResponse {
            filename: worker.open_options.filename.clone(),
            data: copy_to_uint8_array(&db),
        })
    })
    .await
}

pub async fn load_db(options: LoadDbOptions) -> Result<()> {
    let db = copy_to_vec(&options.data);

    with_worker(|worker| {
        let _ = std::mem::replace(&mut worker.state, SQLiteState::NotOpened);

        let filename = &worker.open_options.filename;
        if worker.open_options.persist {
            let opfs = get_opfs_util()?;
            opfs.unlink(filename).map_err(|_| WorkerError::Unexpected)?;
            if let Err(err) = opfs.import_db(filename, &db) {
                return Err(WorkerError::LoadDb(format!("{err}")));
            }
        } else {
            let mem_vfs = &FS_UTIL.mem;
            mem_vfs.delete_db(filename);
            if let Err(err) = mem_vfs.import_db(filename, &db) {
                return Err(WorkerError::LoadDb(format!("{err}")));
            }
        }

        worker.state = SQLiteState::Opened(SQLiteDb::open(&worker.open_options.uri())?);
        Ok(())
    })
    .await
}

pub async fn open(options: OpenOptions) -> Result<()> {
    let mut locker = DB.lock().await;
    locker.take();

    if options.persist {
        init_opfs_util().await?;
    }

    let state = SQLiteState::Opened(SQLiteDb::open(&options.uri())?);
    let worker = SQLiteWorker {
        open_options: options,
        state,
    };
    *locker = Some(worker);
    Ok(())
}

pub async fn run(options: RunOptions) -> Result<SQLiteRunResult> {
    with_worker(|worker| {
        if options.clear_on_prepare {
            let _ = std::mem::replace(&mut worker.state, SQLiteState::NotOpened);

            let filename = &worker.open_options.filename;
            if worker.open_options.persist {
                get_opfs_util()?
                    .unlink(filename)
                    .map_err(|_| WorkerError::Unexpected)?;
            } else {
                let mem_vfs = &FS_UTIL.mem;
                mem_vfs.delete_db(filename);
            }

            worker.state = SQLiteState::Opened(SQLiteDb::open(&worker.open_options.uri())?);
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
