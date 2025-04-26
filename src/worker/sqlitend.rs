use serde_json::Value as JsonValue;
use sqlite_wasm_rs::*;
use std::ffi::{CStr, CString};
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use crate::{
    InnerError, SQLiteStatementResult, SQLiteStatementTable, SQLiteStatementValues, SQLitendError,
};

type Result<T> = std::result::Result<T, SQLitendError>;

fn cstr(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| SQLitendError::ToCStr)
}

fn sqlite_err(code: i32, db: *mut sqlite3) -> InnerError {
    let message = unsafe {
        let ptr = sqlite3_errmsg(db);
        CStr::from_ptr(ptr).to_string_lossy().to_string()
    };
    InnerError { code, message }
}

pub struct SQLiteDb {
    sqlite3: *mut sqlite3,
}

unsafe impl Send for SQLiteDb {}
unsafe impl Sync for SQLiteDb {}

impl SQLiteDb {
    pub fn open(filename: &str) -> Result<Arc<Self>> {
        let mut sqlite3 = std::ptr::null_mut();
        let ret = unsafe {
            sqlite3_open_v2(
                cstr(filename)?.as_ptr().cast(),
                &mut sqlite3 as *mut _,
                SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
                std::ptr::null(),
            )
        };

        if ret != SQLITE_OK {
            return Err(SQLitendError::OpenDb(sqlite_err(ret, sqlite3)));
        }

        Ok(Arc::new(Self { sqlite3 }))
    }

    pub fn prepare(self: &Arc<Self>, sql: &str) -> Result<SQLiteStatements> {
        let sql = cstr(sql)?;
        let tail = sql.as_ptr();

        Ok(SQLiteStatements {
            sql,
            db: Arc::clone(self),
            tail,
        })
    }
}

impl Drop for SQLiteDb {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.sqlite3);
        }
    }
}

pub struct SQLiteStatements {
    sql: CString,
    db: Arc<SQLiteDb>,
    tail: *const i8,
}

unsafe impl Send for SQLiteStatements {}
unsafe impl Sync for SQLiteStatements {}

impl SQLiteStatements {
    pub fn prepare_next(&mut self) -> Result<Option<SQLitePreparedStatement>> {
        if self.tail.is_null() {
            return Ok(None);
        }

        let sqlite3 = self.db.sqlite3;
        let mut stmt: *mut sqlite3_stmt = std::ptr::null_mut();
        let mut tail = std::ptr::null();

        let ret = unsafe {
            sqlite3_prepare_v3(sqlite3, self.tail, -1, 0, &mut stmt as _, &mut tail as _)
        };

        if ret != SQLITE_OK {
            return Err(SQLitendError::Prepare(sqlite_err(ret, sqlite3)));
        }

        let sql = unsafe { sqlite3_sql(stmt) };
        if sql.is_null() {
            return Ok(None);
        }
        let sql = unsafe { CStr::from_ptr(sql).to_string_lossy().to_string() };

        let start_offset = self.tail as usize - self.sql.as_ptr() as usize;
        let end_offset = start_offset + sql.len();
        let position = [start_offset, end_offset];

        self.tail = tail;

        Ok(Some(SQLitePreparedStatement {
            sql,
            done: AtomicBool::new(false),
            position,
            sqlite3,
            stmt,
        }))
    }

    pub fn stmts_result(self) -> Result<Vec<SQLiteStatementResult>> {
        let mut result = vec![];
        for stmt in self {
            let stmt = stmt?;
            result.push(stmt.pack(stmt.get_all()?));
        }
        Ok(result)
    }
}

impl Iterator for SQLiteStatements {
    type Item = Result<SQLitePreparedStatement>;

    fn next(&mut self) -> Option<Self::Item> {
        self.prepare_next().transpose()
    }
}

pub struct SQLitePreparedStatement {
    sql: String,
    position: [usize; 2],
    done: AtomicBool,
    sqlite3: *mut sqlite3,
    stmt: *mut sqlite3_stmt,
}

unsafe impl Send for SQLitePreparedStatement {}
unsafe impl Sync for SQLitePreparedStatement {}

impl SQLitePreparedStatement {
    /// Stepping to the next line
    fn step(&self) -> Result<bool> {
        let ret = unsafe { sqlite3_step(self.stmt) };
        match ret {
            SQLITE_DONE => {
                self.done.store(true, atomic::Ordering::SeqCst);
                Ok(false)
            }
            SQLITE_ROW => Ok(true),
            code => Err(SQLitendError::Step(sqlite_err(code, self.sqlite3))),
        }
    }

    pub fn pack(&self, values: Option<SQLiteStatementValues>) -> SQLiteStatementResult {
        let values = SQLiteStatementTable {
            sql: self.sql.clone(),
            position: self.position,
            done: self.done.load(atomic::Ordering::SeqCst),
            values,
        };
        SQLiteStatementResult::Step(values)
    }

    pub fn get_all(&self) -> Result<Option<SQLiteStatementValues>> {
        let mut values = match self.get_one()? {
            Some(value) => value,
            None => return Ok(None),
        };

        while let Some(value) = self.get_one()? {
            for row in value.rows {
                values.rows.push(row);
            }
        }

        Ok(Some(values))
    }

    /// Get data for all columns of the current row
    pub fn get_one(&self) -> Result<Option<SQLiteStatementValues>> {
        if !self.step()? {
            return Ok(None);
        }

        let column_count = unsafe { sqlite3_column_count(self.stmt) };

        let mut column = Vec::with_capacity(column_count as usize);
        let mut row = Vec::with_capacity(column_count as usize);

        for col_ndx in 0..column_count {
            // column_name as key
            let (column_name, column_type) = unsafe {
                let ptr = sqlite3_column_name(self.stmt, col_ndx);
                if ptr.is_null() {
                    return Err(SQLitendError::GetColumnName(
                        "the column name is a null pointer, this shouldn't happen".into(),
                    ));
                }
                let Ok(column_name) = CStr::from_ptr(ptr).to_str() else {
                    return Err(SQLitendError::GetColumnName(
                        "the column name is not a string, this shouldn't happen".into(),
                    ));
                };
                (column_name, sqlite3_column_type(self.stmt, col_ndx))
            };

            // https://www.sqlite.org/c3ref/column_blob.html
            let value = unsafe {
                match column_type {
                    SQLITE_NULL => JsonValue::Null,
                    SQLITE_INTEGER => {
                        let number = sqlite3_column_int64(self.stmt, col_ndx);
                        JsonValue::from(number)
                    }
                    SQLITE_FLOAT => JsonValue::from(sqlite3_column_double(self.stmt, col_ndx)),
                    SQLITE_TEXT => {
                        let slice = {
                            let text = sqlite3_column_text(self.stmt, col_ndx);
                            // get text size, there may be problems if use as cstr
                            let len = sqlite3_column_bytes(self.stmt, col_ndx);
                            std::slice::from_raw_parts(text, len as usize)
                        };
                        // must be UTF-8 TEXT result
                        let Ok(text) = std::str::from_utf8(slice) else {
                            return Err(SQLitendError::Utf8Text);
                        };
                        JsonValue::from(text)
                    }
                    SQLITE_BLOB => {
                        let slice = {
                            let blob = sqlite3_column_blob(self.stmt, col_ndx);
                            let len = sqlite3_column_bytes(self.stmt, col_ndx);
                            std::slice::from_raw_parts(blob.cast::<u8>(), len as usize)
                        };
                        JsonValue::from(slice)
                    }
                    _ => return Err(SQLitendError::UnsupportColumnType(column_type)),
                }
            };

            column.push(column_name.into());
            row.push(value);
        }

        Ok(Some(SQLiteStatementValues {
            columns: column,
            rows: vec![row],
        }))
    }
}

impl Drop for SQLitePreparedStatement {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.stmt);
        };
    }
}
