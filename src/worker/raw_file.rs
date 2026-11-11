use std::ffi::CString;

use sqlite_wasm_rs::*;

use crate::WorkerError;

type Result<T> = std::result::Result<T, WorkerError>;

pub(super) struct ReadOnlyFile {
    handle: *mut sqlite3_file,
    filename: sqlite3_filename,
}

impl ReadOnlyFile {
    pub fn open(filename: &str) -> Result<Self> {
        let kind = if filename.ends_with("-journal") {
            SQLITE_OPEN_MAIN_JOURNAL
        } else if filename.ends_with("-wal") {
            SQLITE_OPEN_WAL
        } else if filename.ends_with("-shm") {
            // SHM is copied as a raw file; this VFS does not use xShm methods.
            0
        } else {
            SQLITE_OPEN_MAIN_DB
        };
        let vfs = unsafe { sqlite3_vfs_find(c"opfs".as_ptr()) };
        if vfs.is_null() {
            return Err(WorkerError::DownloadDb(
                "Database VFS is unavailable".into(),
            ));
        }
        let database = CString::new(filename).map_err(|_| invalid_filename())?;
        let journal =
            CString::new(format!("{filename}-journal")).map_err(|_| invalid_filename())?;
        let wal = CString::new(format!("{filename}-wal")).map_err(|_| invalid_filename())?;
        let filename = unsafe {
            sqlite3_create_filename(
                database.as_ptr(),
                journal.as_ptr(),
                wal.as_ptr(),
                0,
                std::ptr::null_mut(),
            )
        };
        if filename.is_null() {
            return Err(WorkerError::DownloadDb(
                "Unable to allocate a VFS filename".into(),
            ));
        }
        let size = unsafe { (*vfs).szOsFile } as usize;
        let handle = unsafe { sqlite3_malloc64(size as u64) }.cast::<sqlite3_file>();
        if handle.is_null() {
            unsafe { sqlite3_free_filename(filename) };
            return Err(WorkerError::DownloadDb(
                "Unable to allocate a VFS handle".into(),
            ));
        }
        unsafe { std::ptr::write_bytes(handle.cast::<u8>(), 0, size) };
        let file = Self { handle, filename };
        let code = unsafe {
            (*vfs).xOpen.unwrap()(
                vfs,
                filename,
                handle,
                SQLITE_OPEN_READONLY | kind,
                std::ptr::null_mut(),
            )
        };
        check(code, "Opening the database file")?;
        Ok(file)
    }

    pub fn len(&self) -> Result<u64> {
        let mut size = 0;
        let code = unsafe { (*(*self.handle).pMethods).xFileSize.unwrap()(self.handle, &mut size) };
        check(code, "Reading the database size")?;
        u64::try_from(size).map_err(|_| WorkerError::DownloadDb("Invalid database size".into()))
    }

    pub fn first_byte(&self) -> Result<u8> {
        let mut byte = [0];
        self.read(&mut byte, 0)?;
        Ok(byte[0])
    }

    fn read(&self, buffer: &mut [u8], offset: i64) -> Result<()> {
        let code = unsafe {
            (*(*self.handle).pMethods).xRead.unwrap()(
                self.handle,
                buffer.as_mut_ptr().cast(),
                buffer.len() as i32,
                offset,
            )
        };
        check(code, "Reading the database file")
    }

    pub fn read_all(&self) -> Result<Vec<u8>> {
        let size = usize::try_from(self.len()?)
            .map_err(|_| WorkerError::DownloadDb("Database is too large to download".into()))?;
        let mut data = Vec::new();
        data.try_reserve_exact(size).map_err(|_| {
            WorkerError::DownloadDb("Unable to allocate the database download".into())
        })?;
        data.resize(size, 0);
        const CHUNK_SIZE: usize = 64 * 1024;
        for (index, chunk) in data.chunks_mut(CHUNK_SIZE).enumerate() {
            self.read(chunk, (index * CHUNK_SIZE) as i64)?;
        }
        Ok(data)
    }
}

impl Drop for ReadOnlyFile {
    fn drop(&mut self) {
        unsafe {
            if let Some(methods) = (*self.handle).pMethods.as_ref() {
                methods.xClose.unwrap()(self.handle);
            }
            sqlite3_free(self.handle.cast());
            sqlite3_free_filename(self.filename);
        }
    }
}

fn invalid_filename() -> WorkerError {
    WorkerError::DownloadDb("Invalid database filename".into())
}

fn check(code: i32, operation: &str) -> Result<()> {
    if code == SQLITE_OK {
        Ok(())
    } else {
        Err(WorkerError::DownloadDb(format!(
            "{operation} failed (SQLite error {code})"
        )))
    }
}
