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

The same on 2026-11-13 (270,480 lines):

| envoc at | `lex` | `parse` | envoc-rs (Rust) | Rust faster by |
|---|---|---|---|---|
| EnvoLang 001dc6f | 897 ms | 1,037 ms | 331 ms, 408 ms | 2.7×, 2.5× |
| EnvoLang 805fcc0: aggregates split into scalars, `realloc` growing large blocks in place | 510 ms | 582 ms | 265 ms, 311 ms | 1.9×, 1.9× |
| EnvoLang 75faef9: caller-saved registers by liveness at each call, aggregate arguments without copies, temporaries merged into variables | 698 ms | 795 ms | 357 ms, 442 ms | 2.0×, 1.8× |
| EnvoLang 0a51102: byte loops eight bytes a step, `\|\|` chains threaded fully | 404 ms | 469 ms | 215 ms, 293 ms | 1.9×, 1.6× |
| EnvoLang 55dba06: dead zeroing of inlined locals removed, loop depth from real loops | 445 ms | 533 ms | 290 ms, 348 ms | 1.5×, 1.5× |
| EnvoLang 4449f8b: no frame pointer, large blocks reserving 64 times their room | 535 ms | 637 ms | 360 ms, 427 ms | 1.5×, 1.5× |
| EnvoLang c08cd00: `return g(...)` writing the caller's storage, tighter register hints | 372 ms | 460 ms | 267 ms, 309 ms | 1.4×, 1.5× |

CI runners differ from run to run (the input also grows a little with each commit, since it
is EnvoLang's own sources), so compare the last column. The second row's runner was faster
(envoc-rs too). Most of the gain on Linux came from
`realloc`: a list or string that doubles is no longer copied into freshly faulted pages,
which costs more on Linux than on Windows.

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
| locals known to be zero, compact frames, aggregates split into scalars (`opt/sroa.envol`), `realloc` growing large blocks in place | 752 ms | 905 ms |
| everything up to EnvoLang c08cd00 (the Linux table's rows), averaged over four code offsets | 579 ms | 693 ms |

From 0.5.0: 87% and 86% less time. envoc-rs measured 675 ms and 634 ms the same day: on
this machine envoc now lexes faster than envoc-rs, and Rust parses 1.09× faster. Code
placement alone moves envoc's times by up to 10% on this Zen 1 processor, so the last row
is the mean over the same compiler built at four code offsets. The profile
(`scripts/winprof.py`; `WINPROF_FUNC` lists the hottest addresses inside one function)
says what is next: the spills of large functions (DumpTree, Lexer.Next) that live across
many calls, the load and store of a `string`'s length on every appended byte, and the
allocation each number printed makes.

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
