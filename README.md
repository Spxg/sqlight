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

Share links include the SQL, database engine, and Keep Context setting. Opening a
link loads the example without running it. After loading, the share parameters
are removed from the address bar so refreshing restores your subsequent edits.
Database files are not included in share links.

| Extension | Description |
| --- | --- |
| [sqlite-vec](https://crates.io/crates/sqlite-wasm-vec) | Vector search for SQLite. |

## Database engines and encryption

Choose SQLite, SQLite3 Multiple Ciphers, or SQLCipher under **Advanced options →
SQLite Engine**. Each engine uses a separate OPFS storage directory, so switching
engines keeps their databases separate.

To create or open an encrypted SQLCipher database, select SQLCipher and run the
key statement before any database reads or writes:

```sql
PRAGMA key = 'example';
CREATE TABLE notes (text TEXT);
INSERT INTO notes VALUES ('Hello from SQLCipher');
SELECT * FROM notes;
```

Enable **Keep Context** to keep the keyed connection across executions. After
importing an encrypted database, run `PRAGMA key = 'example';` with its passphrase
before querying it. Import and export reopen the connection, so run the key
statement again afterward. Keys are not restored automatically; SQL entered in
the editor follows the normal editor saving and sharing behavior.

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
execution, SQL results and error recovery, saved preferences, sharing, and database
import and export. It also checks all three engines, sqlite-vec, and OPFS
persistence while blocking external network requests.

The Checks workflow runs formatting, unit tests, Clippy, the production build, and
these browser checks on pushes and pull requests.
