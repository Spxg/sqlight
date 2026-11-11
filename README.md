# SQLight

A SQLite Playground that runs entirely in your browser. SQL runs locally in a
Web Worker with [sqlite-wasm-rs](https://github.com/Spxg/sqlite-wasm-rs); no database
server is required.

The layout follows the [Rust Playground](https://play.rust-lang.org), using
[Leptos](https://leptos.dev) for the UI and Ace for SQL editing. It includes
resizable editor and output panes, light and dark themes, alternative editor
keybindings, database import and export, and SQLite extensions.

| Extension | Description |
| --- | --- |
| [sqlite-vec](https://crates.io/crates/sqlite-wasm-vec) | Vector search for SQLite. |

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
cargo clippy --target wasm32-unknown-unknown \
  --workspace --all-targets --all-features -- -D warnings
```

## Production build

```sh
npm ci
npm run build
```

Serve the generated `dist` directory with a static web server. The app requires
HTTPS or localhost for browser storage features. Both SQLite worker binaries are
built automatically, including the SQLite3 Multiple Ciphers variant.

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

The script serves `dist` on localhost and checks editing, formatting, keyboard
execution, SQL results and error recovery, saved preferences, sharing, and database
import and export. It also checks both worker engines, sqlite-vec, and OPFS
persistence while blocking external network requests.

The Checks workflow runs formatting, unit tests, Clippy, the production build, and
these browser checks on pushes and pull requests.
