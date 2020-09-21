# JW Library Notes Sync

JW-Sync is a utility to merge 2 or more `.jwlibrary` backup files,
containing your personal notes, highlighting, etc.
At time of writing, the JW Library app has backup and restore commands,
but no merge command.
With the official app, you can transfer user data between devices,
but you can't combine them into a single set.

This project is a port of <https://github.com/AntonyCorbett/JWLMerge>.
While JWLMerge only supports Windows (C#, .NET Framework), this utility is programmed
using the Rust programming language to support more platforms:
Browser (WebAssembly), Linux, MacOS and Windows.

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
target/release/merge-jwl previous.jwpub revised.jwpub
```

## Build web app

1. Run `cargo install cargo-wasi`
1. Install [`wasi-sdk`](https://github.com/WebAssembly/wasi-sdk)
1. Install `wasm-opt` from [binaryen](https://github.com/WebAssembly/binaryen) (if the bundled one fails)
1. Install Node.js and Yarn

```sh
export CC_wasm32_wasi="/opt/wasi-sdk/bin/clang"
export CARGO_TARGET_WASM32_WASI_LINKER="/opt/wasi-sdk/bin/clang"
export WASM_OPT=/usr/bin/wasm-opt
RUSTFLAGS="-C target-feature=-crt-static" cargo wasi build --release
cp target/wasm32-wasi/release/merge-jwl.wasm www/public/
cd www
yarn build
```
