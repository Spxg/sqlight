# SQLite log fixtures

`logged-databases.zip` contains genuine SQLite files, generated with Python's
SQLite 3.54.1. Browser tests read this checked-in archive without requiring
Python at runtime.

| Files | Expected contents |
| --- | --- |
| `wal.db`, `wal.db-wal`, `wal.db-shm` | The main file has an empty `records` table. Committed WAL records contain values 41 and 42: two rows with sum 83. |
| `hot.db`, `hot.db-journal` | An interrupted transaction spilled uncommitted changes into the main file. Writable recovery restores 80 rows whose `value` is 1, giving sum 80. A read-only connection must reject recovery. |

The hot journal has a valid nonzero header and no super-journal reference. The
WAL snapshot is copied before closing its connection, so committed records have
not been checkpointed. The SHM file is included to verify that archive transfers
preserve it even when the browser's exclusive connection uses an in-memory WAL
index.

To regenerate the archive, use Python 3 with its built-in `sqlite3` module:

```sh
python3 scripts/fixtures/generate-logged-databases.py
```

Generation uses a temporary directory. Its child process exits without closing
SQLite to leave a real hot journal; it never accesses browser data. SQLite
versions and random journal salts can change the binary bytes while retaining
the expected query results.

See SQLite's [WAL documentation](https://www.sqlite.org/wal.html) and
[hot-journal recovery](https://www.sqlite.org/lockingv3.html#hot_journals).
