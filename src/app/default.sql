-- Welcome to SQLight! Everything runs locally in your browser.
-- Click Run, or press Ctrl+Enter / Cmd+Enter, to try this bookstore demo.

PRAGMA page_size=4096;

CREATE TABLE IF NOT EXISTS blobs (
    id INTEGER PRIMARY KEY,
    data BLOB
);

INSERT INTO blobs(data) VALUES (randomblob(12));

SELECT 'Hello World!',
        datetime('now','localtime') AS TM,
        unhex('73716c69676874') AS BLOB_VAL,
        NULL as NULL_VAL;

SELECT * FROM blobs;
