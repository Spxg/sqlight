mod sqlitend;

use crate::{
    DownloadDbOptions, DownloadDbResponse, LoadDbOptions, OpenOptions, PERSIST_VFS, PrepareOptions,
    SQLiteStatementResult, WorkerError,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use sqlite_wasm_rs::{
    export::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil},
    mem_vfs::MemVfsUtil,
    utils::{copy_to_uint8_array, copy_to_vec},
};
use sqlitend::{SQLiteDb, SQLitePreparedStatement, SQLiteStatements};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::OnceCell;

type Result<T> = std::result::Result<T, WorkerError>;

static DB_POOL: Lazy<Mutex<HashMap<String, SQLiteWorker>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static FS_UTIL: Lazy<FSUtil> = Lazy::new(|| FSUtil {
    mem: MemVfsUtil::new(),
    opfs: OnceCell::new(),
});

struct FSUtil {
    mem: MemVfsUtil,
    opfs: OnceCell<OpfsSAHPoolUtil>,
}

struct SQLiteWorker {
    id: String,
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

fn with_worker<F, T>(id: &str, mut f: F) -> Result<T>
where
    F: FnMut(&mut SQLiteWorker) -> Result<T>,
{
    f(DB_POOL.lock().get_mut(id).ok_or(WorkerError::NotFound)?)
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

pub async fn download_db(options: DownloadDbOptions) -> Result<DownloadDbResponse> {
    with_worker(&options.id, |worker| {
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
}

pub async fn load_db(options: LoadDbOptions) -> Result<()> {
    let db = copy_to_vec(&options.data);

    with_worker(&options.id, |worker| {
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
}

pub async fn open(options: OpenOptions) -> Result<String> {
    if let Some(worker) = DB_POOL.lock().get(&options.filename) {
        return Ok(worker.id.clone());
    }
    if options.persist {
        let util = init_opfs_util().await?;
        if util.get_capacity() - util.get_file_count() * 3 < 3 {
            util.add_capacity(3)
                .await
                .map_err(|_| WorkerError::Unexpected)?;
        }
    }
    // FIXME: multi db support
    let id = String::new();
    let db = SQLiteDb::open(&options.uri())?;
    let worker = SQLiteWorker {
        id: id.clone(),
        db: Some(db),
        open_options: options,
        state: SQLiteState::Idie,
    };
    DB_POOL.lock().insert(id.clone(), worker);
    Ok(id)
}

pub async fn prepare(options: PrepareOptions) -> Result<()> {
    with_worker(&options.id, |worker| {
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
}

pub fn r#continue(id: &str) -> Result<Vec<SQLiteStatementResult>> {
    with_worker(id, |worker| {
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
}

pub fn step_over(id: &str) -> Result<SQLiteStatementResult> {
    with_worker(id, |worker| match &mut worker.state {
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
}

pub fn step_in(id: &str) -> Result<()> {
    with_worker(id, |worker| {
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
}

pub fn step_out(id: &str) -> Result<SQLiteStatementResult> {
    with_worker(id, |worker| match &mut worker.state {
        SQLiteState::Idie => Err(WorkerError::InvaildState),
        SQLiteState::Prepared(prepared_state) => {
            if let Some(prepared) = prepared_state.prepared.take() {
                Ok(prepared.pack(prepared.get_all()?))
            } else {
                Err(WorkerError::InvaildState)
            }
        }
    })
}
