# ForSM

A standalone Forth interpreter/compiler for WebAssembly. Bootstrapped from a Rust program, but the ultimate goal for it is to be self-hosting.

## Architecture
[./src/rust-bootstrapper](./src/rust-bootstrapper) is a Rust program which compiles a (very minimal) Forth interpreter.

This minimal interpreter is just powerful enough to import every file in [./src/prelude](./src/prelude), which define the rest of its functionality.

Features include:
 - WASI-compliant, no extra imports needed.
 - Many standard Forth words. Most, even!
 - An interactive interpreter, supports stdin or `include`d files.
 - Runtime colon definitions (including custom runtime behavior with `does>`).
 - Heap allocation with `allocate`, `resize`, and `free`.

## Running it
```bash
# Compile the minimal interpreter
cargo run

# Run it with any WASI implementation; e.g. with wasmmer
wasmer --dir=. ./bin/forsm.wasm

# Pass filenames and it'll run them
wasmer --dir=. ./bin/forsm.wasm src/scripts/test_allocation.fth

# The first preopened directory must be this directory; the interpreter needs to load its own source code from ./src/prelude.

```