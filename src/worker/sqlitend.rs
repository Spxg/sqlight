use std::ffi::{CStr, CString};

use sqlite_wasm_rs::*;
use sqlite_wasm_vec::sqlite3_vec_init;

use crate::{
    InnerError, SQLiteStatementResult, SQLiteStatementTable, SQLiteStatementValues, SQLitendError,
};

type Result<T> = std::result::Result<T, SQLitendError>;

fn sqlite_error(code: i32, db: *mut sqlite3) -> InnerError {
    let message = unsafe { CStr::from_ptr(sqlite3_errmsg(db)) }
        .to_string_lossy()
        .into_owned();
    InnerError { code, message }
}

pub struct SQLiteDb {
    handle: *mut sqlite3,
}

impl SQLiteDb {
    pub fn is_autocommit(&self) -> bool {
        unsafe { sqlite3_get_autocommit(self.handle) != 0 }
    }

    pub fn attachment_limit(&self) -> usize {
        unsafe { sqlite3_limit(self.handle, SQLITE_LIMIT_ATTACHED, -1) }.max(0) as usize
    }

    pub fn uses_file(&self, filename: &str, vfs_name: &str) -> bool {
        self.file_schema(filename, vfs_name).is_some()
    }

    pub fn file_schema(&self, filename: &str, vfs_name: &str) -> Option<(String, bool)> {
        let mut index = 0;
        loop {
            let schema = unsafe { sqlite3_db_name(self.handle, index) };
            if schema.is_null() {
                return None;
            }
            let path = unsafe { sqlite3_db_filename(self.handle, schema) };
            if !path.is_null() && unsafe { CStr::from_ptr(path) }.to_bytes() == filename.as_bytes()
            {
                let mut vfs: *mut sqlite3_vfs = std::ptr::null_mut();
                let code = unsafe {
                    sqlite3_file_control(
                        self.handle,
                        schema,
                        SQLITE_FCNTL_VFS_POINTER,
                        (&mut vfs as *mut *mut sqlite3_vfs).cast(),
                    )
                };
                if code != SQLITE_OK
                    || vfs.is_null()
                    || unsafe { CStr::from_ptr((*vfs).zName) }.to_bytes() == vfs_name.as_bytes()
                {
                    let read_only = unsafe { sqlite3_db_readonly(self.handle, schema) != 0 };
                    let name = unsafe { CStr::from_ptr(schema) }
                        .to_string_lossy()
                        .into_owned();
                    return Some((name, read_only));
                }
            }
            index += 1;
        }
    }

    pub fn checkpoint(&self, schema: &str) -> Result<()> {
        let schema = CString::new(schema).map_err(|_| SQLitendError::ToCStr)?;
        let code = unsafe {
            sqlite3_wal_checkpoint_v2(
                self.handle,
                schema.as_ptr(),
                SQLITE_CHECKPOINT_TRUNCATE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if code != SQLITE_OK {
            return Err(SQLitendError::Step(sqlite_error(code, self.handle)));
        }
        Ok(())
    }

    pub fn open(filename: &str, read_only: bool) -> Result<Self> {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute::<
                *const (),
                unsafe extern "C" fn(
                    *mut sqlite3,
                    *mut *mut i8,
                    *const sqlite3_api_routines,
                ) -> i32,
            >(sqlite3_vec_init as *const ())));
        }
        let filename = CString::new(filename).map_err(|_| SQLitendError::ToCStr)?;
        let mut handle = std::ptr::null_mut();
        let code = unsafe {
            sqlite3_open_v2(
                filename.as_ptr(),
                &mut handle,
                SQLITE_OPEN_URI
                    | if read_only {
                        SQLITE_OPEN_READONLY
                    } else {
                        SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
                    },
                std::ptr::null(),
            )
        };
        if code != SQLITE_OK {
            let error = sqlite_error(code, handle);
            unsafe { sqlite3_close(handle) };
            return Err(SQLitendError::OpenDb(error));
        }
        let db = Self { handle };
        // OPFS has no shared-memory methods. Exclusive mode keeps WAL's index
        // in this worker's memory, including when the database is read-only.
        db.prepare("PRAGMA locking_mode=EXCLUSIVE;")?
            .results(&mut Vec::new())?;
        Ok(db)
    }

    pub fn prepare(&self, sql: &str) -> Result<SQLiteStatements<'_>> {
        Ok(SQLiteStatements {
            sql: CString::new(sql).map_err(|_| SQLitendError::ToCStr)?,
            db: self,
            offset: 0,
        })
    }
}

impl Drop for SQLiteDb {
    fn drop(&mut self) {
        unsafe { sqlite3_close(self.handle) };
    }
}

pub struct SQLiteStatements<'db> {
    sql: CString,
    db: &'db SQLiteDb,
    offset: usize,
}

impl SQLiteStatements<'_> {
    pub fn results(mut self, results: &mut Vec<SQLiteStatementResult>) -> Result<()> {
        while self.offset < self.sql.as_bytes().len() {
            let mut handle = std::ptr::null_mut();
            let mut tail = std::ptr::null();
            let start = self.offset;
            let code = unsafe {
                sqlite3_prepare_v3(
                    self.db.handle,
                    self.sql.as_ptr().add(start),
                    -1,
                    0,
                    &mut handle,
                    &mut tail,
                )
            };
            if code != SQLITE_OK {
                return Err(SQLitendError::Prepare(sqlite_error(code, self.db.handle)));
            }
            self.offset = tail as usize - self.sql.as_ptr() as usize;
            // Empty statements and comments do not produce a prepared statement.
            if handle.is_null() {
                continue;
            }
            let statement = PreparedStatement {
                db: self.db,
                handle,
            };
            results.push(SQLiteStatementResult::Step(SQLiteStatementTable {
                position: [start, self.offset],
                values: statement.values()?,
            }));
        }
        results.push(SQLiteStatementResult::Finish);
        Ok(())
    }
}

struct PreparedStatement<'db> {
    db: &'db SQLiteDb,
    handle: *mut sqlite3_stmt,
}

impl PreparedStatement<'_> {
    fn step(&self) -> Result<bool> {
        match unsafe { sqlite3_step(self.handle) } {
            SQLITE_DONE => Ok(false),
            SQLITE_ROW => Ok(true),
            code => Err(SQLitendError::Step(sqlite_error(code, self.db.handle))),
        }
    }

    fn values(&self) -> Result<Option<SQLiteStatementValues>> {
        let mut has_row = self.step()?;
        let count = unsafe { sqlite3_column_count(self.handle) };
        if count == 0 {
            return Ok(None);
        }
        let columns = (0..count)
            .map(|index| {
                let name = unsafe { sqlite3_column_name(self.handle, index) };
                if name.is_null() {
                    return Err(SQLitendError::GetColumnName("Missing column name".into()));
                }
                unsafe { CStr::from_ptr(name) }
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|err| SQLitendError::GetColumnName(err.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut rows = Vec::new();
        while has_row {
            let row = (0..count)
                .map(|index| self.column_value(index))
                .collect::<Result<Vec<_>>>()?;
            rows.push(row);
            has_row = self.step()?;
        }
        Ok(Some(SQLiteStatementValues { columns, rows }))
    }

    fn column_value(&self, index: i32) -> Result<String> {
        // SQLite owns these buffers until the statement advances or is finalized.
        unsafe {
            match sqlite3_column_type(self.handle, index) {
                SQLITE_NULL => Ok("NULL".into()),
                SQLITE_INTEGER => Ok(sqlite3_column_int64(self.handle, index).to_string()),
                SQLITE_FLOAT => Ok(sqlite3_column_double(self.handle, index).to_string()),
                SQLITE_TEXT => {
                    let ptr = sqlite3_column_text(self.handle, index);
                    let length = sqlite3_column_bytes(self.handle, index) as usize;
                    let bytes = if length == 0 {
                        &[]
                    } else {
                        std::slice::from_raw_parts(ptr, length)
                    };
                    let text = std::str::from_utf8(bytes).map_err(|_| SQLitendError::Utf8Text)?;
                    Ok(format!("'{}'", text.replace('\'', "''")))
                }
                SQLITE_BLOB => {
                    let ptr = sqlite3_column_blob(self.handle, index);
                    let length = sqlite3_column_bytes(self.handle, index) as usize;
                    let bytes = if length == 0 {
                        &[]
                    } else {
                        std::slice::from_raw_parts(ptr.cast::<u8>(), length)
                    };
                    Ok(format!("x'{}'", hex::encode(bytes)))
                }
                kind => Err(SQLitendError::UnsupportedColumnType(kind)),
            }
        }
    }
}

impl Drop for PreparedStatement<'_> {
    fn drop(&mut self) {
        unsafe { sqlite3_finalize(self.handle) };
    }
}
