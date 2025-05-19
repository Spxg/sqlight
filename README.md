# SQLight

A SQLite Playground that runs completely locally.

<img alt="image" src="https://github.com/user-attachments/assets/c75858f3-27a4-49b6-805e-c42db6d40593" width="75%" />

## How it works

Run SQLite as wasm in the browser using [`sqlite-wasm-rs`](https://github.com/Spxg/sqlite-wasm-rs).

## About UI

Users who have used [rust playground](https://play.rust-lang.org) will feel very familiar. 

Because the layout and component design are derived from it, but rewritten using [leptos](https://leptos.dev).

## Local deployment

```sh
npm install
npx postcss --dir assets/module.postcss assets/module.css --base assets/module.css
# Choose your preferred installation method
# https://trunkrs.dev/#install
cargo install trunk --locked
# The product is in the dist folder
trunk build --release
```
