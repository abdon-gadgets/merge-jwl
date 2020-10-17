# JW Library Notes Sync

JW-Sync is a utility to merge 2 or more `.jwlibrary` backup files,
containing your personal notes, highlighting, etc.
At time of writing, the JW Library app has backup and restore commands,
but no merge command.
With the official app, you can transfer user data between devices,
but you can't combine them into a single set.

This project is a port of <https://github.com/AntonyCorbett/JWLMerge>.
While JWLMerge is a desktop application (.NET Framework), this utility is programmed
using the Rust programming language to support more platforms:
Browser (WASM), Linux, MacOS and Windows.

You can use the web app <https://merge-jwl.netlify.app/> with Safari on iOS, Chrome on Android
and other modern browsers.
The `.jwlibrary` files that you upload stay on your device, they are not send over the internet.
This is possible because the app merges your data client side using WebAssembly.
Privacy Policy: The web server doesn't collect or process any user data.

## Important Notes

The authors of this software are not affiliated with the developers of JW Library,
and the software should be considered *unofficial*.
“JW Library” is a registered trademark of Watch Tower Bible and Tract Society of Pennsylvania.

Please review the JW Library terms and conditions of use.
Some view the backup data files as their own data and not subject to the conditions;
others feel differently. Make your own decision on this matter.

## Build desktop app

1. Install [Rust](https://www.rust-lang.org/tools/install)

```sh
cargo build --release
target/release/merge-jwl a.jwlibrary b.jwlibrary c.jwlibrary
```

## Build web app

1. Run `rustup install 1.45.2` (1.46 has a regression)
1. Run `cargo install cargo-wasi`
1. Install [`wasi-sdk`](https://github.com/WebAssembly/wasi-sdk)
1. Install `wasm-opt` from [binaryen](https://github.com/WebAssembly/binaryen) (the bundled one fails currently)
1. Install Node.js and Yarn

```sh
export CC_wasm32_wasi="/opt/wasi-sdk/bin/clang"
export CARGO_TARGET_WASM32_WASI_LINKER="/opt/wasi-sdk/bin/clang"
export WASM_OPT=/usr/bin/wasm-opt
RUSTFLAGS="-C target-feature=-crt-static" cargo +1.45.2 wasi build --release
cp target/wasm32-wasi/release/merge-jwl.wasm www/public/
cd www
yarn
yarn build
```
