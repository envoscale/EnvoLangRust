//! syntax/lexer.envol: the lexer. Same first error at the same position as envoc; literal
//! values are validated, not computed.

use super::token::*;
use crate::diag::{column, fail_at};
use crate::util::{append_decimal, StrMap};

pub const E_INVALID_UTF8: u32 = 1;
pub const E_UNEXPECTED_CHAR: u32 = 2;
pub const E_UNTERMINATED_COMMENT: u32 = 3;
pub const E_UNTERMINATED_STRING: u32 = 4;
pub const E_UNTERMINATED_CHAR: u32 = 5;
pub const E_BAD_ESCAPE: u32 = 6;
pub const E_BAD_NUMBER: u32 = 7;
pub const E_BAD_SUFFIX: u32 = 8;
pub const E_CHAR_LENGTH: u32 = 9;
pub const E_BAD_UNICODE_ESCAPE: u32 = 10;
pub const E_CONTROL_CHAR: u32 = 11;
pub const E_CORE_INTERPOLATION: u32 = 901;
pub const E_CORE_RAW_STRING: u32 = 902;
pub const E_CORE_DIRECTIVE: u32 = 903;
pub const E_CORE_NON_ASCII: u32 = 904;
pub const E_NOT_S1: u32 = 950;

/// The tables keywords and punctuation are looked up in; built once per compilation.
pub struct LexTables {
    pub keywords: StrMap<u16>,
    pub puncts: StrMap<u16>,
}

impl LexTables {
    pub fn new() -> Self {
        let mut t = LexTables {
            keywords: StrMap::new(),
            puncts: StrMap::new(),
        };
        for k in TK_KW_FIRST..=TK_KW_LAST {
            t.keywords.put(spelling(k).as_bytes(), k);
        }
        for k in TK_PUNCT_FIRST..=TK_PUNCT_LAST {
            t.puncts.put(spelling(k).as_bytes(), k);
        }
        t
    }
}

/// Tokenizes one source file and returns its tokens, ending with TK_EOF. Stops at the first
/// error.
pub fn lex(tables: &LexTables, path: &[u8], text: &[u8], file: u32, core: bool) -> Vec<Token> {
    let mut lx = Lexer {
        path,
        src: text,
        pos: 0,
        line: 1,
        flags: TF_SPACE_BEFORE | TF_NEWLINE_BEFORE,
        file,
        core,
    };
    let bad = utf8_invalid(text);
    if bad != text.len() {
        lx.fail(bad, E_INVALID_UTF8);
    }
    if text.len() >= 2 && text[0] == b'#' && text[1] == b'!' {
        while lx.pos < text.len() && text[lx.pos] != 10 {
            lx.pos += 1;
        }
    }
    let mut toks: Vec<Token> = Vec::new();
    while lx.trivia() {
        let t = lx.next(tables);
        toks.push(t);
    }
    lx.pos = text.len();
    toks.push(lx.emit(TK_EOF, text.len()));
    toks
}

/// The offset of the first byte that makes text invalid UTF-8, or text.len().
pub fn utf8_invalid(text: &[u8]) -> usize {
    let mut i = 0;
    let n = text.len();
    while i < n {
        let b = text[i];
        if b < 0x80 {
            i += 1;
            continue;
        }
        if b < 0xC2 || b >= 0xF5 {
            return i;
        }
        if b < 0xE0 {
            if i + 1 >= n || text[i + 1] & 0xC0 != 0x80 {
                return i;
            }
            i += 2;
            continue;
        }
        if b < 0xF0 {
            if i + 2 >= n {
                return i;
            }
            let mut lo = 0x80u8;
            let mut hi = 0xBFu8;
            if b == 0xE0 {
                lo = 0xA0;
            }
            if b == 0xED {
                hi = 0x9F;
            }
            let c1 = text[i + 1];
            if c1 < lo || c1 > hi || text[i + 2] & 0xC0 != 0x80 {
                return i;
            }
            i += 3;
            continue;
        }
        if i + 3 >= n {
            return i;
        }
        let mut lo4 = 0x80u8;
        let mut hi4 = 0xBFu8;
        if b == 0xF0 {
            lo4 = 0x90;
        }
        if b == 0xF4 {
            hi4 = 0x8F;
        }
        let d1 = text[i + 1];
        if d1 < lo4 || d1 > hi4 || text[i + 2] & 0xC0 != 0x80 || text[i + 3] & 0xC0 != 0x80 {
            return i;
        }
        i += 4;
    }
    n
}

#[inline]
fn is_letter(c: u8) -> bool {
    (c | 0x20) >= b'a' && (c | 0x20) <= b'z'
}
#[inline]
fn is_ident_cont(c: u8) -> bool {
    c == b'_' || c.is_ascii_digit() || is_letter(c)
}
#[inline]
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

/// The value of a digit or letter for bases up to 36, or 99.
fn digit_value(c: u8) -> u32 {
    if is_digit(c) {
        return (c - b'0') as u32;
    }
    if is_letter(c) {
        return ((c | 0x20) - b'a') as u32 + 10;
    }
    99
}

/// 0 if s is not a numeric suffix, 1 for an integer suffix, 2 for a float suffix.
fn suffix_kind(s: &[u8]) -> u32 {
    match s {
        b"f32" | b"f64" => 2,
        b"i8" | b"i16" | b"i32" | b"i64" | b"i128" | b"isize" | b"u8" | b"u16" | b"u32"
        | b"u64" | b"u128" | b"usize" => 1,
        _ => 0,
    }
}

struct Lexer<'a> {
    path: &'a [u8],
    src: &'a [u8],
    pos: usize,
    line: u32,
    flags: u16,
    file: u32,
    core: bool,
}

impl<'a> Lexer<'a> {
    fn fail(&self, off: usize, code: u32) -> ! {
        let msg = match code {
            E_INVALID_UTF8 => "source file is not valid UTF-8",
            E_UNEXPECTED_CHAR => "unexpected character",
            E_UNTERMINATED_COMMENT => "unterminated block comment",
            E_UNTERMINATED_STRING => "unterminated string literal",
            E_UNTERMINATED_CHAR => "unterminated character literal",
            E_BAD_ESCAPE => "unknown or malformed escape sequence",
            E_BAD_NUMBER => "malformed numeric literal",
            E_BAD_SUFFIX => "invalid suffix on numeric literal",
            E_CHAR_LENGTH => "character literal must contain exactly one character",
            E_BAD_UNICODE_ESCAPE => "invalid Unicode escape",
            E_CONTROL_CHAR => "control character in literal; use an escape sequence",
            _ => "?",
        };
        fail_at(self.path, self.src, off, code, msg)
    }

    fn fail_subset(&self, off: usize, core_code: u32) -> ! {
        if self.core {
            let msg = match core_code {
                E_CORE_INTERPOLATION => "interpolated strings are not part of Envo-Core",
                E_CORE_RAW_STRING => "raw string literals are not part of Envo-Core",
                E_CORE_DIRECTIVE => "directives are not part of Envo-Core",
                _ => "non-ASCII characters outside literals and comments are not supported in Envo-Core",
            };
            fail_at(self.path, self.src, off, core_code, msg);
        }
        let what = match core_code {
            E_CORE_INTERPOLATION => "not part of Envo-S1: interpolated strings",
            E_CORE_RAW_STRING => "not part of Envo-S1: raw string literals",
            E_CORE_DIRECTIVE => "not part of Envo-S1: directives",
            _ => "not part of Envo-S1: non-ASCII characters outside literals and comments",
        };
        fail_at(self.path, self.src, off, E_NOT_S1, what)
    }

    #[inline]
    fn emit(&mut self, kind: u16, start: usize) -> Token {
        let t = Token {
            kind,
            flags: self.flags,
            start: start as u32,
            len: (self.pos - start) as u32,
            line: self.line,
            file: self.file,
        };
        self.flags = 0;
        t
    }

    /// Skips whitespace and comments, recording them in the next token's flags. Returns
    /// false at the end of the file.
    fn trivia(&mut self) -> bool {
        let n = self.src.len();
        while self.pos < n {
            let c = self.src[self.pos];
            if c == b' ' || c == 9 || c == 13 {
                self.flags |= TF_SPACE_BEFORE;
                self.pos += 1;
            } else if c == 10 {
                self.flags |= TF_SPACE_BEFORE | TF_NEWLINE_BEFORE;
                self.line += 1;
                self.pos += 1;
            } else if c == b'/' && self.pos + 1 < n && self.src[self.pos + 1] == b'/' {
                self.flags |= TF_SPACE_BEFORE;
                while self.pos < n && self.src[self.pos] != 10 {
                    self.pos += 1;
                }
            } else if c == b'/' && self.pos + 1 < n && self.src[self.pos + 1] == b'*' {
                self.flags |= TF_SPACE_BEFORE;
                let open = self.pos;
                self.pos += 2;
                let mut depth = 1u32;
                while depth > 0 {
                    if self.pos >= n {
                        self.fail(open, E_UNTERMINATED_COMMENT);
                    }
                    let b = self.src[self.pos];
                    if b == 10 {
                        self.line += 1;
                        self.flags |= TF_NEWLINE_BEFORE;
                        self.pos += 1;
                    } else if b == b'*' && self.pos + 1 < n && self.src[self.pos + 1] == b'/' {
                        self.pos += 2;
                        depth -= 1;
                    } else if b == b'/' && self.pos + 1 < n && self.src[self.pos + 1] == b'*' {
                        self.pos += 2;
                        depth += 1;
                    } else {
                        self.pos += 1;
                    }
                }
            } else {
                return true;
            }
        }
        false
    }

    /// The token at the current position (there is one: trivia returned true).
    fn next(&mut self, tables: &LexTables) -> Token {
        let start = self.pos;
        let n = self.src.len();
        let c = self.src[start];
        if c >= 0x80 {
            self.fail_subset(start, E_CORE_NON_ASCII);
        }
        if is_letter(c) || c == b'_' {
            let mut p = start + 1;
            while p < n && is_ident_cont(self.src[p]) {
                p += 1;
            }
            self.pos = p;
            let kind = tables.keywords.get(&self.src[start..p]).unwrap_or(TK_IDENT);
            return self.emit(kind, start);
        }
        if is_digit(c) {
            return self.number(start);
        }
        if c == b'"' {
            return self.string_lit(start);
        }
        if c == 0x27 {
            return self.char_lit(start);
        }
        if c == b'@' {
            if start + 1 >= n {
                self.fail(start, E_UNEXPECTED_CHAR);
            }
            let d = self.src[start + 1];
            if d != b'_' && !is_letter(d) {
                self.fail(start, E_UNEXPECTED_CHAR);
            }
            let mut p = start + 2;
            while p < n && is_ident_cont(self.src[p]) {
                p += 1;
            }
            if p - start == 2 && d == b'_' {
                self.fail(start, E_UNEXPECTED_CHAR);
            }
            self.pos = p;
            return self.emit(TK_IDENT, start);
        }
        if c == b'$' {
            if start + 1 < n && self.src[start + 1] == b'"' {
                self.fail_subset(start, E_CORE_INTERPOLATION);
            }
            self.fail(start, E_UNEXPECTED_CHAR);
        }
        if c == b'#' {
            if self.flags & TF_NEWLINE_BEFORE != 0 {
                self.fail_subset(start, E_CORE_DIRECTIVE);
            }
            self.fail(start, E_UNEXPECTED_CHAR);
        }
        let mut len = 3usize;
        while len > 0 {
            if start + len <= n && (start + len == n || self.src[start + len] & 0xC0 != 0x80) {
                if let Some(k) = tables.puncts.get(&self.src[start..start + len]) {
                    self.pos = start + len;
                    return self.emit(k, start);
                }
            }
            len -= 1;
        }
        self.fail(start, E_UNEXPECTED_CHAR)
    }

    /// Validates the escape whose backslash is at pos and moves past it.
    fn escape(&mut self) {
        let p = self.pos;
        let n = self.src.len();
        if p + 1 >= n {
            self.pos = p + 1;
            return;
        }
        let c = self.src[p + 1];
        if c == b'n' || c == b'r' || c == b't' || c == b'0' || c == 0x5C || c == 0x27 || c == b'"' {
            self.pos = p + 2;
            return;
        }
        if c == b'x' {
            if p + 3 >= n {
                self.fail(p, E_BAD_ESCAPE);
            }
            let h = self.src[p + 2];
            if !(b'0'..=b'7').contains(&h) {
                self.fail(p, E_BAD_ESCAPE);
            }
            if digit_value(self.src[p + 3]) >= 16 {
                self.fail(p, E_BAD_ESCAPE);
            }
            self.pos = p + 4;
            return;
        }
        if c != b'u' {
            self.fail(p, E_BAD_ESCAPE);
        }
        if p + 2 >= n || self.src[p + 2] != b'{' {
            self.fail(p, E_BAD_UNICODE_ESCAPE);
        }
        let mut q = p + 3;
        let mut v: u64 = 0;
        loop {
            if q >= n {
                self.fail(p, E_BAD_UNICODE_ESCAPE);
            }
            let d = self.src[q];
            if d == b'}' {
                break;
            }
            let dv = digit_value(d);
            if dv >= 16 {
                self.fail(p, E_BAD_UNICODE_ESCAPE);
            }
            v = (v << 4) | dv as u64;
            q += 1;
            if q - p > 3 + 6 {
                self.fail(p, E_BAD_UNICODE_ESCAPE);
            }
        }
        if q - p == 3 || v > 0x10FFFF || v & 0xFFFFF800 == 0xD800 {
            self.fail(p, E_BAD_UNICODE_ESCAPE);
        }
        self.pos = q + 1;
    }

    fn number(&mut self, start: usize) -> Token {
        let n = self.src.len();
        let mut p = start;
        let mut is_float = false;
        let mut radix = 10u32;
        if self.src[p] == b'0' && p + 1 < n {
            match self.src[p + 1] {
                b'x' => radix = 16,
                b'b' => radix = 2,
                b'o' => radix = 8,
                _ => {}
            }
        }
        if radix != 10 {
            p += 2;
            let mut digits = 0u32;
            while p < n {
                let c = self.src[p];
                if c != b'_' {
                    let v = digit_value(c);
                    if v >= radix {
                        if v < 10 {
                            self.fail(p, E_BAD_NUMBER);
                        }
                        break;
                    }
                    digits += 1;
                }
                p += 1;
            }
            if digits == 0 {
                self.fail(start, E_BAD_NUMBER);
            }
        } else {
            while p < n && (self.src[p] == b'_' || is_digit(self.src[p])) {
                p += 1;
            }
            if p + 1 < n && self.src[p] == b'.' && is_digit(self.src[p + 1]) {
                is_float = true;
                p += 2;
                while p < n && (self.src[p] == b'_' || is_digit(self.src[p])) {
                    p += 1;
                }
            }
            if p < n && self.src[p] | 0x20 == b'e' {
                is_float = true;
                p += 1;
                if p >= n {
                    self.fail(start, E_BAD_NUMBER);
                }
                if self.src[p] == b'+' || self.src[p] == b'-' {
                    p += 1;
                }
                let mut edigits = 0u32;
                while p < n {
                    let c = self.src[p];
                    if c != b'_' {
                        if !is_digit(c) {
                            break;
                        }
                        edigits += 1;
                    }
                    p += 1;
                }
                if edigits == 0 {
                    self.fail(start, E_BAD_NUMBER);
                }
            }
        }
        if p < n && is_ident_cont(self.src[p]) {
            let s = p;
            while p < n && is_ident_cont(self.src[p]) {
                p += 1;
            }
            let k = suffix_kind(&self.src[s..p]);
            if k == 0 {
                self.fail(s, E_BAD_SUFFIX);
            }
            if k == 1 && is_float {
                self.fail(s, E_BAD_SUFFIX);
            }
            if k == 2 {
                if radix != 10 {
                    self.fail(s, E_BAD_SUFFIX);
                }
                is_float = true;
            }
        }
        self.pos = p;
        if is_float {
            return self.emit(TK_FLOAT, start);
        }
        self.emit(TK_INT, start)
    }

    fn string_lit(&mut self, start: usize) -> Token {
        let n = self.src.len();
        if start + 2 < n && self.src[start + 1] == b'"' && self.src[start + 2] == b'"' {
            self.fail_subset(start, E_CORE_RAW_STRING);
        }
        self.pos = start + 1;
        loop {
            let p = self.pos;
            if p >= n {
                self.fail(start, E_UNTERMINATED_STRING);
            }
            let c = self.src[p];
            if c == b'"' {
                self.pos = p + 1;
                break;
            }
            if c == 0x5C {
                self.escape();
                continue;
            }
            if c == 10 || c == 13 {
                self.fail(start, E_UNTERMINATED_STRING);
            }
            if c != 9 && (c < 0x20 || c == 0x7F) {
                self.fail(p, E_CONTROL_CHAR);
            }
            self.pos = p + 1;
        }
        self.emit(TK_STRING, start)
    }

    fn char_lit(&mut self, start: usize) -> Token {
        let n = self.src.len();
        let mut p = start + 1;
        if p >= n {
            self.fail(start, E_UNTERMINATED_CHAR);
        }
        let c = self.src[p];
        if c == 0x27 {
            self.fail(start, E_CHAR_LENGTH);
        }
        if c == 10 || c == 13 {
            self.fail(start, E_UNTERMINATED_CHAR);
        }
        if c == 0x5C {
            self.pos = p;
            self.escape();
            p = self.pos;
        } else {
            if c != 9 && (c < 0x20 || c == 0x7F) {
                self.fail(p, E_CONTROL_CHAR);
            }
            if c < 0x80 {
                p += 1;
            } else if c < 0xE0 {
                p += 2;
            } else if c < 0xF0 {
                p += 3;
            } else {
                p += 4;
            }
        }
        if p >= n {
            self.fail(start, E_UNTERMINATED_CHAR);
        }
        if self.src[p] != 0x27 {
            let mut q = p;
            loop {
                if q >= n || self.src[q] == 10 {
                    self.fail(start, E_UNTERMINATED_CHAR);
                }
                if self.src[q] == 0x27 {
                    self.fail(start, E_CHAR_LENGTH);
                }
                q += 1;
            }
        }
        self.pos = p + 1;
        self.emit(TK_CHAR, start)
    }
}

/// `lex`: appends one line per token, "<line>:<col> <n|s|j> <class> <text>".
pub fn dump_tokens(dst: &mut Vec<u8>, toks: &[Token], text: &[u8]) {
    for t in toks {
        append_decimal(dst, t.line as u64);
        dst.push(b':');
        append_decimal(dst, column(text, t.start as usize) as u64);
        if t.flags & TF_NEWLINE_BEFORE != 0 {
            dst.extend_from_slice(b" n ");
        } else if t.flags & TF_SPACE_BEFORE != 0 {
            dst.extend_from_slice(b" s ");
        } else {
            dst.extend_from_slice(b" j ");
        }
        if t.kind >= TK_PUNCT_FIRST {
            dst.extend_from_slice(b"punct ");
            dst.extend_from_slice(spelling(t.kind).as_bytes());
        } else if t.kind >= TK_KW_FIRST {
            dst.extend_from_slice(b"kw ");
            dst.extend_from_slice(spelling(t.kind).as_bytes());
        } else {
            dst.extend_from_slice(spelling(t.kind).as_bytes());
            if t.kind != TK_EOF {
                dst.push(b' ');
                dst.extend_from_slice(&text[t.start as usize..(t.start + t.len) as usize]);
            }
        }
        dst.push(10);
    }
}
