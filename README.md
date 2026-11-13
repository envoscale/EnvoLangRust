<img src="https://raw.githubusercontent.com/envoscale/EnvoScale/main/docs/images/logo.svg" alt="" width="56" align="left">

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

## Numbers

The compiler's own sources joined ten times (237,550 lines, 9.7 MB); best of five runs;
both write the same output (46 MB for `parse`). CI times it on Linux on every push (the
job summary has the table).

On Linux (CI, envoc 0.5.0 against envoc-rs with fat LTO):

| | envoc (Envo) | envoc-rs (Rust) | |
|---|---|---|---|
| `lex` | 2,606 ms | 228 ms | Rust 11× faster |
| `parse` | 2,850 ms | 285 ms | Rust 10× faster |

What each change to envoc's code generation gave, on Windows 10 (envoc-rs: 656 ms and
617 ms, without LTO):

| envoc | `lex` | `parse` |
|---|---|---|
| 0.5.0 | 4,429 ms | 5,028 ms |
| locals promoted to SSA values (`opt/mem2reg.envo`) | 3,755 ms | 4,418 ms |
| compare and branch fused, fall-through, immediates, direct branch moves | 3,328 ms | 4,195 ms |
| results computed in their own registers | 3,133 ms | 3,894 ms |
| ten registers instead of five | 3,000 ms | 3,724 ms |
| unsigned division by constants, `match` as jump tables | 2,752 ms | 3,505 ms |

So far: 38% and 30% faster, and still about 4.5× behind. The profile (`scripts/winprof.py`)
says what is next: the copies `List.Get` returns, calls (arguments go through the stack),
loops (nothing is hoisted yet), and the allocator.

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
