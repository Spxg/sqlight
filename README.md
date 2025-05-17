# SQLite Playground

A SQLite Playground that runs entirely locally.

## How it works

Run SQLite as wasm in the browser using [`sqlite-wasm-rs`](https://github.com/Spxg/sqlite-wasm-rs).

## About UI

Users who have used [Rust Playground](https://play.rust-lang.org) will feel very familiar. 

Because the layout and component design are derived from it, but rewritten using [leptos](https://leptos.dev).

## Local deployment

```sh
npm install
npx postcss --dir assets/module.postcss assets/module.css --base assets/module.css
# Choose your preferred installation method
# https://trunkrs.dev/#install
cargo install trunk --locked
trunk serve --open
```
