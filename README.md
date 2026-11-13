<img src="https://raw.githubusercontent.com/envoscale/.github/main/profile/assets/logo.svg" alt="" width="56" align="left">

# EnvoLangRust

`envoc-rs`: the EnvoL compiler ([EnvoLang](https://github.com/envoscale/EnvoLang)'s
`compiler/`) ported to Rust, function for function, so the same compiler can be timed
built by `envoc` and built by `rustc`. That tells us how good EnvoL's generated code is,
and where to make it better.

Rules for the port:

- **Same algorithms and data structures.** The hash map is envoc's (FNV-1a, linear
  probing, the same growth); the tree is the same flat array of nodes; the same steps in
  the same order. What changes is only what the language makes natural: Rust's `Vec`,
  slices and borrowing instead of EnvoL's `List`, `str` and `ref`.
- **Same output, byte for byte.** Every command prints exactly what `envoc` prints,
  errors included; `scripts/parity.sh` checks it on every `.envol` file in EnvoLang.
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

| | envoc (EnvoL) | envoc-rs (Rust) | |
|---|---|---|---|
| `lex` | 2,606 ms | 228 ms | Rust 11× faster |
| `parse` | 2,850 ms | 285 ms | Rust 10× faster |

The same on 2026-11-13 (270,480 lines), envoc at EnvoLang 001dc6f:

| | envoc (EnvoL) | envoc-rs (Rust) | |
|---|---|---|---|
| `lex` | 897 ms | 331 ms | Rust 2.7× faster |
| `parse` | 1,037 ms | 408 ms | Rust 2.5× faster |

The Windows numbers below are closer because envoc-rs is built there without LTO.

What each change to envoc's code generation gave, on Windows 10 (envoc-rs: 656 ms and
617 ms, without LTO):

| envoc | `lex` | `parse` |
|---|---|---|
| 0.5.0 | 4,429 ms | 5,028 ms |
| locals promoted to SSA values (`opt/mem2reg.envol`) | 3,755 ms | 4,418 ms |
| compare and branch fused, fall-through, immediates, direct branch moves | 3,328 ms | 4,195 ms |
| results computed in their own registers | 3,133 ms | 3,894 ms |
| ten registers instead of five | 3,000 ms | 3,724 ms |
| unsigned division by constants, `match` as jump tables | 2,752 ms | 3,505 ms |
| arguments in registers, `&&`/`||` threaded into one branch, small multi-block functions inlined | 2,610 ms | 3,453 ms |
| small copies unrolled (`str` and slice copies were `rep movsb`), field addresses folded into loads and stores, short encodings, loop-invariant loads hoisted | 1,518 ms | 1,835 ms |
| (the same envoc, measured again on 2026-11-13) | 1,442 ms | 1,737 ms |
| common subexpressions and redundant loads (`opt/cse.envol`), block layout (`opt/layout.envol`), register hints for branch arguments, panic paths that no longer stop small accessors from inlining | 1,169 ms | 1,544 ms |
| slots zeroed on entry only when something may read them first | 1,059 ms | 1,218 ms |
| loop-invariant arithmetic, spill costs weighted by loop depth, chains of copies through temporaries collapsed, one quotient for `/` and `%` | 976 ms | 1,101 ms |
| registers shared across lifetime holes (a loop counter returned after the loop needs no copy per round) | 921 ms | 1,083 ms |
| bounds checks a dominating `i < n` already covers removed, `select` as `cmov` | 892 ms | 1,077 ms |
| constants stored as immediates, small copies kept off rsi and rdi, a value just spilled read from its register; `List.Grow` moves its items in one copy | 881 ms | 1,060 ms |

From 0.5.0: 80% and 79% less time. envoc-rs measured 685 ms and 644 ms the same day, so
Rust is 1.29× and 1.65× faster. The profile (`scripts/winprof.py`; `WINPROF_FUNC` lists
the hottest addresses inside one function) says what is next: per-byte loops such as
`diag.Column`'s (Rust's slice iterator checks bounds once, and LLVM counts several bytes
at a time), the six registers envoc keeps for scratch and frames out of x86-64's
sixteen, and the load and store of a `string`'s length on every appended byte.

## Use

```bash
cargo build --release
./scripts/parity.sh <envoc> target/release/envoc-rs <EnvoLang checkout> parse
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

## License

Apache-2.0: see [LICENSE](LICENSE) and [NOTICE](NOTICE).
