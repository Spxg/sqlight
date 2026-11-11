# SQLight

A SQLite Playground that runs entirely in your browser. SQL runs locally in a
Web Worker with [sqlite-wasm-rs](https://github.com/Spxg/sqlite-wasm-rs); no database
server is required.

The layout follows the [Rust Playground](https://play.rust-lang.org), using
[Leptos](https://leptos.dev) for the UI and Ace for SQL editing. It includes
resizable editor and output panes, light and dark themes, alternative editor
keybindings, database import and export, and SQLite extensions.

Query results show 100 rows per page. Switching output tabs or collapsing the
panel keeps the current result page. If a statement fails, execution stops and
completed query results remain available in Execution. Statements are not
automatically wrapped in a transaction; use `BEGIN` and `ROLLBACK` when needed.

Share links include the SQL, database engine, Fresh database per run setting, and
Read only mode. Opening a link creates a configuration with a separate scratch
database in OPFS, without running the SQL or replacing your existing drafts.
After loading, the share parameters are removed from the address bar so refreshing
restores your subsequent edits. Database files and the original main filename are
not included in share links.

| Extension | Description |
| --- | --- |
| [sqlite-vec](https://crates.io/crates/sqlite-wasm-vec) | Vector search for SQLite. |

## Workspaces and read-only queries

Use the workspace menu in the header to create, rename, select, or delete a
workspace. Each workspace keeps its own SQL draft, database engine, main database
selection, and Fresh database per run setting. Appearance, editor preferences,
and **Config → Run Selected SQL** apply to every workspace. New configurations
inherit the selected engine, main database, and Read only setting, preserve the
database between runs, and start with a query listing the schema's tables.
The initial main database is `default.db`.

Enter a filename in **Workspace → New database name** and choose **Create
database** to create and select an empty SQLite file. It is available for
download immediately. Creation turns off Fresh database per run and preserves
Read only and the SQL draft. Existing filenames are rejected without replacing
their files.

Configurations using the same engine share database files. Several configurations
can select `a.db` while keeping different SQL drafts. Renaming or
deleting a configuration leaves those shared files intact; deleting removes its
saved SQL and settings. At least one configuration remains available.

Configure **Main database**, **SQLite Engine**, **Read only**, and **Fresh database
per run** directly in the workspace menu. Database files are stored in browser
OPFS and persist across visits. Fresh database per run empties the selected main
database before each execution, affecting every configuration that uses that
file.

Configurations using the same main database and Read only setting reuse the
current connection, including its transaction and encryption key. Commit or roll
back before switching to another main database or changing Read only. Changing
these settings reopens the connection.

Enable **Read only** in the workspace menu to block SQL writes to database files.
It works independently of **Fresh database per run**. Enable both to start each
execution with an empty main database opened for read-only queries, such as
`PRAGMA compile_options` or `SELECT sqlite_version()`. Clear Read only to allow
SQL writes again. Explicit file imports remain available and preserve Read only.

Existing installations become the **Default** workspace. Their saved SQL and
preferences, and existing OPFS databases, remain available. An engine's OPFS files
can be open in one browser tab at a time. Close or reload the tab using that
engine before running queries from another tab.

## Multiple database files

Use **Database → Load** to import several files together. Uploads preserve their
filenames and leave the workspace's main database, SQL, and settings unchanged.
Replacing an existing filename requires confirmation. Select an uploaded file
with **Workspace → Main database**; this turns off Fresh database per run.
Uploads and downloads use raw SQLite files; ZIP archives are not supported.

Files in the same engine can be queried together with `ATTACH`. For example,
after importing `a.db` and `b.db`, select `a.db` and run:

```sql
ATTACH 'b.db' AS b;
SELECT customers.name, sum(b.orders.total) AS total
FROM customers
JOIN b.orders ON b.orders.customer_id = customers.id
GROUP BY customers.id;
DETACH b;
```

Use **Database → Download database** to choose any shared file to download. Its
filename is preserved, and downloading leaves the main database, SQL draft,
connection, and Read only setting unchanged. The three database engines keep
separate sets of files.

## Database engines and encryption

Choose SQLite, SQLite3 Multiple Ciphers, or SQLCipher under **Workspace → SQLite
Engine**. Each engine uses a separate OPFS storage directory, so switching
engines keeps their databases separate.

To create or open an encrypted SQLCipher database, select SQLCipher and run the
key statement before any database reads or writes:

```sql
PRAGMA key = 'example';
CREATE TABLE notes (text TEXT);
INSERT INTO notes VALUES ('Hello from SQLCipher');
SELECT * FROM notes;
```

Leave **Fresh database per run** off to keep the keyed connection across
executions. After importing an encrypted database, run `PRAGMA key = 'example';`
with its passphrase before querying it. Import or changing the main database or
Read only reopens the connection, so run the key statement again afterward.
Downloads preserve the connection and its key. Keys are not restored
automatically; SQL entered in the editor follows the normal editor saving and
sharing behavior.

Setting a key does not encrypt an existing plaintext database. Use
[`sqlcipher_export()`](https://www.zetetic.net/sqlcipher/sqlcipher-api/#sqlcipher_export)
to convert one. See the [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/)
for encryption settings and compatibility options.

## Development

Install [Rust](https://rustup.rs), [Node.js 26 LTS](https://nodejs.org), and
[Trunk](https://trunkrs.dev). The checked-in `rust-toolchain.toml` installs the
stable Rust toolchain and the `wasm32-unknown-unknown` target.

```sh
cargo install trunk --version 0.22.1 --locked
npm ci
npm run dev
```

Open the URL printed by Trunk. It rebuilds and reloads the application when Rust,
CSS, or JavaScript source files change.

Trunk runs `npm run assets` before every build. This compiles CSS Modules and
their Rust class-name maps, and bundles Ace, Floating UI, Split Grid, normalize.css,
and the fonts from the locked npm dependencies. Generated files live in
`assets/module.postcss` and `assets/generated`; they are not committed. Runtime
assets are served locally without third-party CDNs.

For direct Rust checks, generate the assets first:

```sh
npm run assets
cargo fmt --all -- --check
cargo test --lib --no-default-features
for engine in sqlite3 sqlite3mc sqlcipher; do
  cargo clippy --target wasm32-unknown-unknown \
    --workspace --all-targets --no-default-features --features "$engine" -- -D warnings
done
```

SQLCipher and SQLite3 Multiple Ciphers are mutually exclusive Cargo features, so
check each engine separately instead of using `--all-features`.

## Production build

```sh
npm ci
npm run build
```

Serve the generated `dist` directory with a static web server. The app requires
HTTPS or localhost for browser storage features. All three worker binaries are
built automatically: SQLite, SQLite3 Multiple Ciphers, and SQLCipher.

The manually triggered Pages workflow runs the same build and publishes `dist`
to GitHub Pages.

## Browser checks

Build the application first, then install Chromium and run the browser regression
checks:

```sh
npm run build
npx playwright install chromium
npm run test:browser
```

To check a build outside `dist`, pass its directory with
`npm run test:browser -- /path/to/build`.

The script serves `dist` on localhost and checks editing, formatting, keyboard
execution, SQL results and error recovery, saved preferences, sharing, workspaces,
read-only queries, and database import and export. It also checks all three
engines, sqlite-vec, and OPFS persistence while blocking external network requests.

The Checks workflow runs formatting, unit tests, Clippy, the production build, and
these browser checks on pushes and pull requests.
