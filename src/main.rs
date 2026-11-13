//! envoc-rs: the Envo compiler, ported to Rust line for line from EnvoScale's compiler/
//! (driver/main.envol), so the two can be timed on the same work. Each command prints
//! exactly what envoc prints.

mod diag;
mod syntax;
mod util;

use diag::*;
use std::io::Write;
use syntax::lexer::*;
use syntax::parser::{dump_tree, parse};

const VERSION: &str = "envoc-rs 0.1.0 (a Rust port of envoc; ";

const MAX_SOURCE: usize = 16 << 20;

fn usage(fd: i32) {
    let text = "usage: envoc-rs [--core] <command> [arguments]\n\ncommands:\n  lex <file.envol>     print the token stream of a source file\n  version             print the version\n";
    if fd == 1 {
        let _ = std::io::stdout().write_all(text.as_bytes());
    } else {
        let _ = std::io::stderr().write_all(text.as_bytes());
    }
}

fn host_target() -> &'static str {
    if cfg!(windows) {
        "x86_64-windows"
    } else {
        "x86_64-linux"
    }
}

/// Reads the source file at path into src, without a leading byte order mark, and returns
/// its index (driver/build.envol `LoadSource`).
fn load_source(src: &mut Sources, path: &[u8]) -> usize {
    let text = match util::file::read_file(path) {
        Some(t) => t,
        None => tool_fail(
            &format!("cannot open '{}'", String::from_utf8_lossy(path)),
            EXIT_ERRORS,
        ),
    };
    if text.len() > MAX_SOURCE {
        tool_fail(
            &format!(
                "file too large: '{}' (envoc accepts source files up to 16 MiB)",
                String::from_utf8_lossy(path)
            ),
            EXIT_ERRORS,
        );
    }
    if text.len() >= 3 && text[0] == 0xEF && text[1] == 0xBB && text[2] == 0xBF {
        return src.add(path, text[3..].to_vec());
    }
    src.add(path, text)
}

fn write_out(bytes: &[u8]) {
    let _ = std::io::stdout().write_all(bytes);
}

/// `envoc lex <file.envol>`
fn cmd_lex(args: &[Vec<u8>], core: bool) -> i32 {
    if args.len() != 1 {
        usage(2);
        return EXIT_USAGE;
    }
    let mut src = Sources::new();
    let f = load_source(&mut src, &args[0]);
    let tables = LexTables::new();
    let toks = lex(&tables, src.path(f), src.text(f), f as u32, core);
    let mut listing: Vec<u8> = Vec::new();
    dump_tokens(&mut listing, &toks, src.text(f));
    write_out(&listing);
    EXIT_OK
}

/// Lexes every file in src, in order, into one token list; starts gets each file's first
/// token (driver/build.envol `LexAll`).
fn lex_all(src: &Sources, starts: &mut Vec<usize>, core: bool) -> Vec<syntax::token::Token> {
    let tables = LexTables::new();
    let mut all = Vec::new();
    for f in 0..src.count() {
        starts.push(all.len());
        let toks = lex(&tables, src.path(f), src.text(f), f as u32, core);
        all.extend_from_slice(&toks);
    }
    all
}

/// `envoc parse <file.envol>`
fn cmd_parse(args: &[Vec<u8>], core: bool) -> i32 {
    if args.len() != 1 {
        usage(2);
        return EXIT_USAGE;
    }
    let mut src = Sources::new();
    load_source(&mut src, &args[0]);
    let mut starts = Vec::new();
    let toks = lex_all(&src, &mut starts, core);
    let tree = parse(&src, toks, &starts, core);
    let mut listing: Vec<u8> = Vec::new();
    dump_tree(&mut listing, &tree, &src, tree.root, 0);
    write_out(&listing);
    EXIT_OK
}

fn args_bytes() -> Vec<Vec<u8>> {
    std::env::args_os()
        .map(|a| {
            #[cfg(windows)]
            {
                a.to_string_lossy().into_owned().into_bytes()
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::ffi::OsStringExt;
                a.into_vec()
            }
        })
        .collect()
}

fn main() {
    let args = args_bytes();
    let mut argi = 1;
    let mut core = false;
    if argi < args.len() && args[argi] == b"--core" {
        core = true;
        argi += 1;
    }
    if argi >= args.len() {
        usage(2);
        std::process::exit(EXIT_USAGE);
    }
    let command = args[argi].clone();
    argi += 1;
    let rest = &args[argi..];
    let code = match command.as_slice() {
        b"version" | b"--version" => {
            write_out(format!("{}{})\n", VERSION, host_target()).as_bytes());
            EXIT_OK
        }
        b"lex" => cmd_lex(rest, core),
        b"parse" => cmd_parse(rest, core),
        b"help" | b"--help" => {
            usage(1);
            EXIT_OK
        }
        _ => tool_fail(
            &format!(
                "unknown command '{}'; run `envoc help` for the list",
                String::from_utf8_lossy(&command)
            ),
            EXIT_USAGE,
        ),
    };
    let _ = std::io::stdout().flush();
    std::process::exit(code);
}
