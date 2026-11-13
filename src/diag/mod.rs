//! diag/report.envo: source files and diagnostics. Errors print in the format every stage
//! shares, "path:line:col: error[E####]: message", then exit with code 1.

use crate::util::append_decimal;
use std::io::Write;

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERRORS: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_INTERNAL: i32 = 70;

pub struct SourceFile {
    pub path: Vec<u8>,
    pub text: Vec<u8>,
}

/// Every source file the compiler has read, in the order read; a token refers to its file
/// by index.
#[derive(Default)]
pub struct Sources {
    pub files: Vec<SourceFile>,
}

impl Sources {
    pub fn new() -> Self {
        Sources { files: Vec::new() }
    }
    pub fn add(&mut self, path: &[u8], text: Vec<u8>) -> usize {
        self.files.push(SourceFile {
            path: path.to_vec(),
            text,
        });
        self.files.len() - 1
    }
    pub fn count(&self) -> usize {
        self.files.len()
    }
    pub fn path(&self, file: usize) -> &[u8] {
        &self.files[file].path
    }
    pub fn text(&self, file: usize) -> &[u8] {
        &self.files[file].text
    }
}

/// Line and column (both from 1; columns count characters, not bytes) of byte offset off.
pub fn position(text: &[u8], off: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;
    let mut i = 0;
    while i < off && i < text.len() {
        let b = text[i];
        if b == 10 {
            line += 1;
            col = 1;
        } else if b & 0xC0 != 0x80 {
            col += 1;
        }
        i += 1;
    }
    (line, col)
}

/// The column of byte offset off: characters from the start of its line, from 1.
pub fn column(text: &[u8], off: usize) -> u32 {
    let mut line_start = off;
    while line_start > 0 && text[line_start - 1] != 10 {
        line_start -= 1;
    }
    let mut col = 1u32;
    for &b in &text[line_start..off] {
        if b & 0xC0 != 0x80 {
            col += 1;
        }
    }
    col
}

/// Reports an error at byte offset off of a source file and exits.
pub fn fail(src: &Sources, file: usize, off: usize, code: u32, msg: &str) -> ! {
    fail_at(src.path(file), src.text(file), off, code, msg)
}

fn append_code(e: &mut Vec<u8>, code: u32) {
    if code < 1000 {
        e.push(b'0');
    }
    if code < 100 {
        e.push(b'0');
    }
    if code < 10 {
        e.push(b'0');
    }
    append_decimal(e, code as u64);
}

/// Reports an error at byte offset off of text, read from path, and exits.
pub fn fail_at(path: &[u8], text: &[u8], off: usize, code: u32, msg: &str) -> ! {
    let (line, col) = position(text, off);
    let mut e: Vec<u8> = path.to_vec();
    e.push(b':');
    append_decimal(&mut e, line as u64);
    e.push(b':');
    append_decimal(&mut e, col as u64);
    e.extend_from_slice(b": error[E");
    append_code(&mut e, code);
    e.extend_from_slice(b"]: ");
    e.extend_from_slice(msg.as_bytes());
    e.push(10);
    let _ = std::io::stderr().write_all(&e);
    std::process::exit(EXIT_ERRORS);
}

/// A problem with the command line or the environment: "envoc: error: msg", then exit.
pub fn tool_fail(msg: &str, code: i32) -> ! {
    let mut e: Vec<u8> = b"envoc: error: ".to_vec();
    e.extend_from_slice(msg.as_bytes());
    e.push(10);
    let _ = std::io::stderr().write_all(&e);
    std::process::exit(code);
}
