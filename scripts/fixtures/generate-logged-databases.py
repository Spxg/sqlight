"""Regenerate the real WAL and hot-journal archive used by browser tests."""

import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import zipfile


def crash_with_hot_journal(path):
    connection = sqlite3.connect(path)
    connection.execute("PRAGMA page_size=1024")
    connection.execute("PRAGMA journal_mode=DELETE")
    connection.execute("PRAGMA synchronous=FULL")
    connection.execute("CREATE TABLE records(value INTEGER, payload BLOB)")
    connection.executemany(
        "INSERT INTO records VALUES (1, ?)", [(b"x" * 850,) for _ in range(80)]
    )
    connection.commit()
    connection.execute("PRAGMA cache_size=4")
    connection.execute("PRAGMA cache_spill=ON")
    connection.execute("BEGIN IMMEDIATE")
    connection.execute("UPDATE records SET value=99, payload=?", (b"y" * 850,))
    # Bypass connection cleanup so SQLite cannot roll back the spilled writes.
    os._exit(0)


def generate():
    output = Path(__file__).with_name("logged-databases.zip")
    with tempfile.TemporaryDirectory(prefix="sqlight-logged-fixture-") as temporary:
        root = Path(temporary)
        wal = root / "wal.db"
        connection = sqlite3.connect(wal)
        connection.execute("PRAGMA journal_mode=WAL")
        connection.execute("PRAGMA wal_autocheckpoint=0")
        connection.execute("CREATE TABLE records(value INTEGER)")
        connection.commit()
        connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        connection.executemany("INSERT INTO records VALUES (?)", [(41,), (42,)])
        connection.commit()
        snapshot = root / "snapshot"
        snapshot.mkdir()
        # Closing the last connection checkpoints WAL, so copy the group first.
        for suffix in ["", "-wal", "-shm"]:
            source = Path(f"{wal}{suffix}")
            shutil.copy2(source, snapshot / source.name)
        connection.close()

        hot = root / "hot.db"
        subprocess.run(
            [sys.executable, str(Path(__file__).resolve()), "--crash-hot", str(hot)],
            check=True,
        )
        for suffix in ["", "-journal"]:
            source = Path(f"{hot}{suffix}")
            shutil.copy2(source, snapshot / source.name)
        with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for source in sorted(snapshot.iterdir()):
                archive.write(source, source.name)
    print(output)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--crash-hot":
        crash_with_hot_journal(Path(sys.argv[2]))
    else:
        generate()
