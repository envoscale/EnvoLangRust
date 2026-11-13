# EnvoScaleRust

`envoc-rs`: the Envo compiler ([EnvoScale](https://github.com/envoscale/EnvoScale)'s
`compiler/`) ported to Rust, function for function, so the same compiler can be timed
built by `envoc` and built by `rustc`. That tells us how good Envo's generated code is,
and where to make it better.

Rules for the port:

- **Same algorithms and data structures.** The hash map is envoc's (FNV-1a, linear
  probing, the same growth); the tree is the same flat array of nodes; the same steps in
  the same order. What changes is only what the language makes natural: Rust's `Vec`,
  slices and borrowing instead of Envo's `List`, `str` and `ref`.
- **Same output, byte for byte.** Every command prints exactly what `envoc` prints,
  errors included; `scripts/parity.sh` checks it on every `.envo` file in EnvoScale.
- **No crates.** The standard library only.

## Status

| Stage | Ported | Same output as envoc |
|---|---|---|
| Lexer (`lex`) | yes | 751 of 751 files |
| Parser (`parse`, `parse --core`) | yes | 751 of 751 files, both modes |
| Checker (`check`) | next | |
| EIR, optimizer, x86-64 backend (`eir`, `build`) | later | |

## First numbers

The compiler's own sources joined ten times (237,550 lines, 9.7 MB), on Windows 10,
best of five runs. Both write the same 46 MB of output.

| | envoc (Envo, -O2) | envoc-rs (Rust, release) | |
|---|---|---|---|
| `lex` | 4,312 ms | 628 ms | Rust 6.9× faster |
| `parse` | 4,938 ms | 585 ms | Rust 8.4× faster |

Most of the difference is code generation, not the algorithms: a sampling profile of
envoc (`scripts/winprof.py`) is flat, with much of the time in calls to tiny functions
the optimizer does not inline yet (`List.Get`, `IsLetter`, accessors), and in the
allocator. Making envoc faster starts there.

## Use

```bash
cargo build --release
./scripts/parity.sh <envoc> target/release/envoc-rs <EnvoScale checkout> parse
```

On Windows, build with the GNU toolchain (`cargo +stable-x86_64-pc-windows-gnu build
--release`) if the MSVC linker is not installed. The `release` profile leaves fat LTO off,
because Windows Defender flags fat-LTO MinGW builds as malware (a false positive); the
`lto` profile turns it on for the Linux benchmarks.

| Script | |
|---|---|
| `scripts/parity.sh` | Both compilers on every file: stdout, stderr and exit codes must match |
| `scripts/bench-files.sh` | Times a command over every file in a folder |
| `scripts/winprof.py` | A sampling profiler for envoc-built Windows programs, using envoc's `-map` |
