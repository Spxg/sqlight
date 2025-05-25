mod sqlitend;

use crate::{
    DownloadDbResponse, LoadDbOptions, OpenOptions, PERSIST_VFS, PrepareOptions,
    SQLiteStatementResult, WorkerError,
};
use once_cell::sync::Lazy;
use sqlite_wasm_rs::{
    export::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil},
    mem_vfs::MemVfsUtil,
    utils::{copy_to_uint8_array, copy_to_vec},
};
use sqlitend::{SQLiteDb, SQLitePreparedStatement, SQLiteStatements};
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
    db: Option<Arc<SQLiteDb>>,
    open_options: OpenOptions,
    state: SQLiteState,
}

enum SQLiteState {
    Idie,
    Prepared(PreparedState),
}

struct PreparedState {
    stmts: SQLiteStatements,
    prepared: Option<SQLitePreparedStatement>,
}

async fn with_worker<F, T>(mut f: F) -> Result<T>
where
    F: FnMut(&mut SQLiteWorker) -> Result<T>,
{
    f(DB.lock().await.as_mut().ok_or(WorkerError::NotFound)?)
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
        worker.db.take();

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

        worker.db = Some(SQLiteDb::open(&worker.open_options.uri())?);
        worker.state = SQLiteState::Idie;
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

    let db = SQLiteDb::open(&options.uri())?;
    let worker = SQLiteWorker {
        db: Some(db),
        open_options: options,
        state: SQLiteState::Idie,
    };
    *locker = Some(worker);
    Ok(())
}

pub async fn prepare(options: PrepareOptions) -> Result<()> {
    with_worker(|worker| {
        if options.clear_on_prepare {
            worker.db.take();

            let filename = &worker.open_options.filename;
            if worker.open_options.persist {
                get_opfs_util()?
                    .unlink(filename)
                    .map_err(|_| WorkerError::Unexpected)?;
            } else {
                let mem_vfs = &FS_UTIL.mem;
                mem_vfs.delete_db(filename);
            }

            worker.db = Some(SQLiteDb::open(&worker.open_options.uri())?);
        }

        let stmts = worker
            .db
            .as_ref()
            .ok_or(WorkerError::InvaildState)?
            .prepare(&options.sql)?;
        worker.state = SQLiteState::Prepared(PreparedState {
            stmts,
            prepared: None,
        });
        Ok(())
    })
    .await
}

pub async fn r#continue() -> Result<Vec<SQLiteStatementResult>> {
    with_worker(|worker| {
        let state = std::mem::replace(&mut worker.state, SQLiteState::Idie);
        let mut result = match state {
            SQLiteState::Idie => return Err(WorkerError::InvaildState),
            SQLiteState::Prepared(prepared_state) => {
                let mut result = vec![];
                if let Some(stmt) = prepared_state.prepared {
                    result.push(stmt.pack(stmt.get_all()?));
                }
                result.extend(prepared_state.stmts.stmts_result()?);
                result
            }
        };
        result.push(SQLiteStatementResult::Finish);
        Ok(result)
    })
    .await
}

pub async fn step_over() -> Result<SQLiteStatementResult> {
    with_worker(|worker| match &mut worker.state {
        SQLiteState::Idie => Err(WorkerError::InvaildState),
        SQLiteState::Prepared(prepared_state) => {
            if let Some(prepared) = &mut prepared_state.prepared {
                if let Some(value) = prepared.get_one()? {
                    Ok(prepared.pack(Some(value)))
                } else {
                    let done = prepared.pack(None);
                    prepared_state.prepared = None;
                    Ok(done)
                }
            } else if let Some(prepared) = prepared_state.stmts.prepare_next()? {
                Ok(prepared.pack(prepared.get_all()?))
            } else {
                Ok(SQLiteStatementResult::Finish)
            }
        }
    })
    .await
}

pub async fn step_in() -> Result<()> {
    with_worker(|worker| {
        match &mut worker.state {
            SQLiteState::Idie => return Err(WorkerError::InvaildState),
            SQLiteState::Prepared(prepared_state) => {
                if prepared_state.prepared.is_some() {
                    return Err(WorkerError::InvaildState);
                }
                let prepared = prepared_state
                    .stmts
                    .prepare_next()?
                    .ok_or(WorkerError::InvaildState)?;
                prepared_state.prepared = Some(prepared);
            }
        };
        Ok(())
    })
    .await
}

pub async fn step_out() -> Result<SQLiteStatementResult> {
    with_worker(|worker| match &mut worker.state {
        SQLiteState::Idie => Err(WorkerError::InvaildState),
        SQLiteState::Prepared(prepared_state) => {
            if let Some(prepared) = prepared_state.prepared.take() {
                Ok(prepared.pack(prepared.get_all()?))
            } else {
                Err(WorkerError::InvaildState)
            }
        }
    })
    .await
}
