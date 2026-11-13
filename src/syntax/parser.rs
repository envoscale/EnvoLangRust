//! syntax/parser.envo and parser_s1.envo: recursive descent over the token list, one method
//! per grammar rule, in the same order as envoc's, so the same trees come out and the same
//! first error is reported at the same token.

use super::lexer::E_NOT_S1;
use super::node::*;
use super::token::*;
use crate::diag::{column, fail_at, Sources};
use crate::util::append_decimal;

pub const MAX_DEPTH: usize = 256;

pub const E_EXPECTED: u32 = 101;
pub const E_CHAINED_COMPARISON: u32 = 102;
pub const E_BAD_ASSIGN_TARGET: u32 = 103;
pub const E_TOO_DEEP: u32 = 104;
pub const E_ENTRY_ONLY: u32 = 210;
pub const E_CORE_CONSTRUCT: u32 = 905;

/// A parsed program: its nodes (node 0 is "none"; root is the program node) and the tokens
/// they refer to.
pub struct Tree {
    pub toks: Vec<Token>,
    pub nodes: Vec<Node>,
    pub root: u32,
}

/// Parses the files whose tokens are in toks, each starting at its index in starts, and
/// joins their items into one program.
pub fn parse(src: &Sources, toks: Vec<Token>, starts: &[usize], core: bool) -> Tree {
    let mut p = Parser {
        toks,
        nodes: Vec::new(),
        undo: Vec::new(),
        pos: 0,
        end: 0,
        depth: 0,
        core,
        in_pattern: false,
        path: b"",
        text: b"",
    };
    p.make(NK_NONE, 0);
    let mut program = 0u32;
    let files = starts.len();
    for f in 0..files {
        p.pos = starts[f];
        p.end = if f + 1 < files {
            starts[f + 1]
        } else {
            p.toks.len()
        };
        p.depth = 0;
        let file = p.toks[p.pos].file as usize;
        p.path = src.path(file);
        p.text = src.text(file);
        let prog = p.parse_program();
        if f == 0 {
            program = prog;
            continue;
        }
        let first = p.at(prog).a;
        let mut n = first;
        while n != 0 {
            let node = p.at(n);
            if !node_is_item(node.kind) {
                p.err_at(E_ENTRY_ONLY, node.tok as usize);
            }
            n = node.next;
        }
        if first == 0 {
            continue;
        }
        let mut tail = p.at(program).a;
        if tail == 0 {
            p.nodes[program as usize].a = first;
            continue;
        }
        while p.at(tail).next != 0 {
            tail = p.at(tail).next;
        }
        p.nodes[tail as usize].next = first;
    }
    Tree {
        toks: p.toks,
        nodes: p.nodes,
        root: program,
    }
}

/// `parse`: appends one node per line, "<2 spaces per level><name>[ <text>] @<line>:<col>".
pub fn dump_tree(dst: &mut Vec<u8>, tree: &Tree, src: &Sources, first: u32, depth: usize) {
    let mut n = first;
    while n != 0 {
        let node = tree.nodes[n as usize];
        let t = tree.toks[node.tok as usize];
        let text = src.text(t.file as usize);
        for _ in 0..depth * 2 {
            dst.push(b' ');
        }
        dst.extend_from_slice(node_name(node.kind).as_bytes());
        if node_has_text(node.kind) {
            dst.push(b' ');
            dst.extend_from_slice(&text[t.start as usize..(t.start + t.len) as usize]);
        }
        append_node_flags(dst, node.flags);
        dst.extend_from_slice(b" @");
        append_decimal(dst, t.line as u64);
        dst.push(b':');
        append_decimal(dst, column(text, t.start as usize) as u64);
        dst.push(10);
        let slots = node_slots(node.kind);
        for s in 0..slots {
            let child = match s {
                0 => node.a,
                1 => node.b,
                2 => node.c,
                _ => node.d,
            };
            if child == 0 {
                for _ in 0..(depth + 1) * 2 {
                    dst.push(b' ');
                }
                dst.extend_from_slice(b"-\n");
            } else {
                dump_tree(dst, tree, src, child, depth + 1);
            }
        }
        if node.e != 0 {
            dump_tree(dst, tree, src, node.e, depth + 1);
        }
        if node.f != 0 {
            dump_tree(dst, tree, src, node.f, depth + 1);
        }
        if node.g != 0 {
            dump_tree(dst, tree, src, node.g, depth + 1);
        }
        n = node.next;
    }
}

/// A list of nodes being built: its first and last node.
#[derive(Clone, Copy)]
struct NodeList {
    head: u32,
    tail: u32,
}

fn no_nodes() -> NodeList {
    NodeList { head: 0, tail: 0 }
}

/// A speculation point: the position, node count and token-split log to restore.
#[derive(Clone, Copy)]
struct Mark {
    pos: usize,
    nodes: usize,
    undo: usize,
}

/// A token split by accept_gt, as it was.
struct Split {
    index: usize,
    was: Token,
}

struct Parser<'a> {
    toks: Vec<Token>,
    nodes: Vec<Node>,
    undo: Vec<Split>,
    pos: usize,
    end: usize,
    depth: usize,
    core: bool,
    in_pattern: bool,
    path: &'a [u8],
    text: &'a [u8],
}

impl<'a> Parser<'a> {
    // ------------------------------------------------------------------ tokens

    #[inline]
    fn kind(&self) -> u16 {
        self.toks[self.pos].kind
    }

    /// The kind of the token k places ahead, clamped to this file's EOF.
    #[inline]
    fn kind_at(&self, k: usize) -> u16 {
        let mut i = self.pos + k;
        if i > self.end - 1 {
            i = self.end - 1;
        }
        self.toks[i].kind
    }

    fn tok_text(&self, i: usize) -> &'a [u8] {
        let t = self.toks[i];
        &self.text[t.start as usize..(t.start + t.len) as usize]
    }

    #[inline]
    fn space_before(&self, i: usize) -> bool {
        self.toks[i].flags & TF_SPACE_BEFORE != 0
    }

    /// Consumes the current token and returns its index. EOF is never consumed.
    #[inline]
    fn advance(&mut self) -> u32 {
        let i = self.pos;
        if self.toks[i].kind != TK_EOF {
            self.pos = i + 1;
        }
        i as u32
    }

    #[inline]
    fn accept(&mut self, kind: u16) -> bool {
        if self.kind() != kind {
            return false;
        }
        self.advance();
        true
    }

    fn expect(&mut self, kind: u16) -> u32 {
        if self.kind() != kind {
            if kind == TK_IDENT {
                self.err_expected("an identifier");
            }
            self.err_expected_kind(kind);
        }
        self.advance()
    }

    fn end_stmt(&mut self) {
        let k = self.kind();
        if k == TK_SEMI {
            self.advance();
            return;
        }
        if k == TK_RBRACE || k == TK_EOF {
            return;
        }
        self.err_expected_kind(TK_SEMI);
    }

    #[inline]
    fn enter(&mut self) {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.err_at(E_TOO_DEEP, self.pos);
        }
    }

    fn save(&self) -> Mark {
        Mark {
            pos: self.pos,
            nodes: self.nodes.len(),
            undo: self.undo.len(),
        }
    }

    fn restore(&mut self, m: Mark) {
        self.pos = m.pos;
        self.nodes.truncate(m.nodes);
        while self.undo.len() > m.undo {
            let s = self.undo.pop().unwrap();
            self.toks[s.index] = s.was;
        }
    }

    // ------------------------------------------------------------------ nodes

    #[inline]
    fn at(&self, n: u32) -> Node {
        self.nodes[n as usize]
    }

    #[inline]
    fn make(&mut self, kind: u16, tok: u32) -> u32 {
        let n = self.nodes.len() as u32;
        self.nodes.push(Node {
            kind,
            tok,
            ..Node::default()
        });
        n
    }

    #[inline]
    fn set_a(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].a = v;
    }
    #[inline]
    fn set_b(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].b = v;
    }
    #[inline]
    fn set_c(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].c = v;
    }
    #[inline]
    fn set_d(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].d = v;
    }
    #[inline]
    fn set_e(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].e = v;
    }
    #[inline]
    fn set_f(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].f = v;
    }
    #[inline]
    fn set_g(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].g = v;
    }
    #[inline]
    fn set_next(&mut self, n: u32, v: u32) {
        self.nodes[n as usize].next = v;
    }
    #[inline]
    fn set_op(&mut self, n: u32, op: u16) {
        self.nodes[n as usize].op = op;
    }
    #[inline]
    fn add_flags(&mut self, n: u32, flags: u32) {
        self.nodes[n as usize].flags |= flags;
    }
    #[inline]
    fn set_flags(&mut self, n: u32, flags: u32) {
        self.nodes[n as usize].flags = flags;
    }

    fn add(&mut self, list: &mut NodeList, n: u32) {
        if list.tail == 0 {
            list.head = n;
        } else {
            self.set_next(list.tail, n);
        }
        list.tail = n;
    }

    fn is_place(&self, n: u32) -> bool {
        let node = self.at(n);
        if node.kind == NK_NAME || node.kind == NK_MEMBER || node.kind == NK_INDEX {
            return true;
        }
        node.kind == NK_UNARY && node.op == TK_STAR
    }

    fn is_name_like(&self, e: u32) -> bool {
        self.at(e).kind == NK_NAME || self.at(e).kind == NK_MEMBER
    }

    // ------------------------------------------------------------------ errors

    fn append_tok_desc(&self, b: &mut Vec<u8>, tok: usize) {
        if self.toks[tok].kind == TK_EOF {
            b.extend_from_slice(b"end of file");
            return;
        }
        b.push(b'`');
        b.extend_from_slice(self.tok_text(tok));
        b.push(b'`');
    }

    fn fail_tok(&self, tok: usize, code: u32, msg: &[u8]) -> ! {
        fail_at(
            self.path,
            self.text,
            self.toks[tok].start as usize,
            code,
            &String::from_utf8_lossy(msg),
        )
    }

    fn err_expected(&self, what: &str) -> ! {
        let mut b: Vec<u8> = b"expected ".to_vec();
        b.extend_from_slice(what.as_bytes());
        b.extend_from_slice(b", found ");
        self.append_tok_desc(&mut b, self.pos);
        self.fail_tok(self.pos, E_EXPECTED, &b)
    }

    fn err_expected_kind(&self, kind: u16) -> ! {
        let w = format!("`{}`", spelling(kind));
        self.err_expected(&w)
    }

    fn err_not_core(&self, what: &str, tok: usize) -> ! {
        let mut code = E_CORE_CONSTRUCT;
        let mut b: Vec<u8> = Vec::new();
        if self.core {
            b.extend_from_slice(b"not part of Envo-Core: ");
        } else {
            b.extend_from_slice(b"not part of Envo-S1: ");
            code = E_NOT_S1;
        }
        if what.is_empty() {
            self.append_tok_desc(&mut b, tok);
        } else {
            b.extend_from_slice(what.as_bytes());
        }
        self.fail_tok(tok, code, &b)
    }

    fn err_at(&self, code: u32, tok: usize) -> ! {
        let msg: &[u8] = match code {
            E_CHAINED_COMPARISON => {
                b"comparison operators cannot be chained; use `&&` or parentheses"
            }
            E_BAD_ASSIGN_TARGET => b"invalid target for assignment",
            E_TOO_DEEP => b"nested too deeply (Stage 1 limit: 256 levels)",
            E_ENTRY_ONLY => {
                b"top-level statements are only allowed in the entry file (the first file given)"
            }
            _ => b"?",
        };
        self.fail_tok(tok, code, msg)
    }

    // ------------------------------------------------------------------ items

    fn parse_program(&mut self) -> u32 {
        let mut items = no_nodes();
        let prog = self.make(NK_PROGRAM, 0);
        while self.kind() != TK_EOF {
            let item = self.parse_toplevel();
            self.add(&mut items, item);
        }
        self.set_a(prog, items.head);
        prog
    }

    fn parse_toplevel(&mut self) -> u32 {
        if !self.core {
            return self.parse_toplevel_s1();
        }
        let mut k = self.kind();
        let visibility = k == TK_KW_PUBLIC || k == TK_KW_PRIVATE;
        if visibility {
            self.advance();
            k = self.kind();
        }
        if k == TK_KW_STRUCT {
            return self.parse_struct();
        }
        if k == TK_KW_ENUM {
            return self.parse_enum();
        }
        if k == TK_KW_CONST {
            return self.parse_binding(NK_CONST);
        }
        if k == TK_KW_STATIC {
            return self.parse_binding(NK_STATIC);
        }
        if self.is_fn_start() {
            return self.parse_function();
        }
        if visibility {
            self.err_expected("a function, `struct`, `enum`, `const` or `static`");
        }
        self.parse_statement()
    }

    /// Does "Type IDENT (" start here? Looks ahead only.
    fn is_fn_start(&mut self) -> bool {
        let m = self.save();
        let mut result = false;
        if self.try_type() != 0 && self.kind() == TK_IDENT {
            let next = self.kind_at(1);
            if next == TK_LPAREN {
                result = true;
            } else if next == TK_LT && !self.core {
                result = true;
            } else if next == TK_LT {
                self.err_not_core("generic functions", self.pos + 1);
            }
        }
        self.restore(m);
        result
    }

    fn parse_function(&mut self) -> u32 {
        let mut params = no_nodes();
        let ty = self.parse_decl_type();
        let name = self.expect(TK_IDENT);
        let f = self.make(NK_FN, name);
        self.set_a(f, ty);
        if !self.core {
            self.parse_function_rest(f, false);
            return f;
        }
        self.expect(TK_LPAREN);
        while self.kind() != TK_RPAREN {
            let k = self.kind();
            if k == TK_KW_IN || k == TK_KW_REF || k == TK_KW_OUT || k == TK_KW_THIS {
                self.err_not_core("", self.pos);
            }
            let ptype = self.parse_decl_type();
            let pname = self.expect(TK_IDENT);
            let param = self.make(NK_PARAM, pname);
            self.set_a(param, ptype);
            if self.kind() == TK_EQ {
                self.err_not_core("default parameter values", self.pos);
            }
            self.add(&mut params, param);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        self.expect(TK_RPAREN);
        let mut body = 0;
        if self.kind() == TK_LBRACE {
            body = self.parse_block();
        } else if self.kind() == TK_FAT_ARROW {
            self.advance();
            body = self.parse_expr();
            self.end_stmt();
        } else {
            self.err_expected("`{` or `=>`");
        }
        self.set_b(f, params.head);
        self.set_c(f, body);
        f
    }

    fn parse_struct(&mut self) -> u32 {
        let mut fields = no_nodes();
        self.advance();
        let name = self.expect(TK_IDENT);
        let st = self.make(NK_STRUCT, name);
        if !self.core {
            if self.kind() == TK_LT {
                let tps = self.parse_type_params();
                self.set_e(st, tps);
            }
            if self.kind() == TK_COLON {
                let bases = self.parse_bases();
                self.set_f(st, bases);
            }
            self.expect(TK_LBRACE);
            self.parse_struct_body(st);
            return st;
        }
        if self.kind() == TK_COLON {
            self.err_not_core("interface lists on structs", self.pos);
        }
        if self.kind() == TK_LT {
            self.err_not_core("generic types", self.pos);
        }
        self.expect(TK_LBRACE);
        while self.kind() != TK_RBRACE {
            if self.kind() == TK_KW_PUBLIC || self.kind() == TK_KW_PRIVATE {
                self.advance();
            }
            let ty = self.parse_decl_type();
            let fname = self.expect(TK_IDENT);
            let field = self.make(NK_FIELD, fname);
            self.set_a(field, ty);
            let k = self.kind();
            if k == TK_LPAREN {
                self.err_not_core("methods", self.pos);
            }
            if k == TK_LBRACE || k == TK_FAT_ARROW {
                self.err_not_core("properties", self.pos);
            }
            if k == TK_EQ {
                self.err_not_core("field initializers", self.pos);
            }
            self.end_stmt();
            self.add(&mut fields, field);
        }
        self.advance();
        self.set_a(st, fields.head);
        st
    }

    fn parse_enum(&mut self) -> u32 {
        let mut variants = no_nodes();
        self.advance();
        let name = self.expect(TK_IDENT);
        let en = self.make(NK_ENUM, name);
        if !self.core && self.kind() == TK_LT {
            let tps = self.parse_type_params();
            self.set_e(en, tps);
        }
        if self.accept(TK_COLON) {
            let under = self.parse_decl_type();
            self.set_a(en, under);
        }
        self.expect(TK_LBRACE);
        while self.kind() != TK_RBRACE {
            let vname = self.expect(TK_IDENT);
            let v = self.make(NK_VARIANT, vname);
            if self.kind() == TK_LPAREN {
                if self.core {
                    self.err_not_core("enum payloads", self.pos);
                }
                let payload = self.parse_payload();
                self.set_e(v, payload);
            }
            if self.accept(TK_EQ) {
                let value = self.parse_expr();
                self.set_a(v, value);
            }
            self.add(&mut variants, v);
            if self.accept(TK_COMMA) {
                continue;
            }
            if self.kind() == TK_RBRACE {
                break;
            }
            self.err_expected("`,` or `}`");
        }
        self.advance();
        self.set_b(en, variants.head);
        en
    }

    fn parse_binding(&mut self, kind: u16) -> u32 {
        self.advance();
        let ty = self.parse_decl_type();
        let name = self.expect(TK_IDENT);
        let n = self.make(kind, name);
        self.set_a(n, ty);
        if kind == NK_CONST || self.kind() == TK_EQ {
            self.expect(TK_EQ);
            let value = self.parse_expr();
            self.set_b(n, value);
        }
        self.end_stmt();
        n
    }

    // ------------------------------------------------------------------ types

    fn try_type(&mut self) -> u32 {
        if !self.core {
            return self.try_type_s1();
        }
        let mut stars = 0usize;
        let first_star = self.pos;
        while self.kind() == TK_STAR {
            self.advance();
            stars += 1;
            if stars > MAX_DEPTH {
                return 0;
            }
        }
        if self.kind() != TK_IDENT {
            return 0;
        }
        let name_tok = self.advance();
        let mut ty = self.make(NK_TNAME, name_tok);
        let mut suffixes = 0usize;
        while self.kind() == TK_LBRACKET {
            let k1 = self.kind_at(1);
            if k1 == TK_RBRACKET {
                suffixes += 1;
                if suffixes > MAX_DEPTH {
                    return 0;
                }
                let open = self.advance();
                self.advance();
                let slice = self.make(NK_TSLICE, open);
                self.set_a(slice, ty);
                ty = slice;
                continue;
            }
            let size_kind = if k1 == TK_IDENT {
                NK_NAME
            } else if k1 != TK_INT {
                break;
            } else {
                NK_INT
            };
            if self.kind_at(2) != TK_RBRACKET {
                break;
            }
            suffixes += 1;
            if suffixes > MAX_DEPTH {
                return 0;
            }
            let open = self.advance();
            let size_tok = self.advance();
            let size = self.make(size_kind, size_tok);
            self.advance();
            let array = self.make(NK_TARRAY, open);
            self.set_a(array, ty);
            self.set_b(array, size);
            ty = array;
        }
        while stars > 0 {
            stars -= 1;
            let ptr = self.make(NK_TPTR, (first_star + stars) as u32);
            self.set_a(ptr, ty);
            ty = ptr;
        }
        ty
    }

    fn parse_type(&mut self) -> u32 {
        let t = self.try_type();
        if t == 0 {
            self.err_expected("a type");
        }
        t
    }

    fn parse_decl_type(&mut self) -> u32 {
        let t = self.parse_type();
        if !self.core {
            return t;
        }
        if self.kind() == TK_QUESTION {
            self.err_not_core("optional types (`T?`)", self.pos);
        }
        if self.kind() == TK_LT {
            self.err_not_core("generic types", self.pos);
        }
        t
    }

    // ------------------------------------------------------------------ statements

    fn parse_statement(&mut self) -> u32 {
        let saved = self.depth;
        self.enter();
        let result;
        let k = self.kind();
        if k == TK_LBRACE {
            result = self.parse_block();
        } else if k == TK_KW_UNSAFE {
            let tok = self.advance();
            let body = self.parse_block();
            result = self.make(NK_UNSAFE, tok);
            self.set_a(result, body);
        } else if k == TK_KW_IF {
            let tok = self.advance();
            let cond = self.paren_condition();
            let then = self.parse_statement();
            let mut other = 0;
            if self.accept(TK_KW_ELSE) {
                other = self.parse_statement();
            }
            result = self.make(NK_IF, tok);
            self.set_a(result, cond);
            self.set_b(result, then);
            self.set_c(result, other);
        } else if k == TK_KW_WHILE {
            let tok = self.advance();
            let cond = self.paren_condition();
            let body = self.parse_statement();
            result = self.make(NK_WHILE, tok);
            self.set_a(result, cond);
            self.set_b(result, body);
        } else if k == TK_KW_FOR {
            let tok = self.advance();
            self.expect(TK_LPAREN);
            let mut init = 0;
            let mut cond = 0;
            let mut step = 0;
            if self.kind() != TK_SEMI {
                init = self.parse_simple();
            }
            self.expect(TK_SEMI);
            if self.kind() != TK_SEMI {
                cond = self.parse_expr();
            }
            self.expect(TK_SEMI);
            if self.kind() != TK_RPAREN {
                step = self.parse_simple();
            }
            self.expect(TK_RPAREN);
            let body = self.parse_statement();
            result = self.make(NK_FOR, tok);
            self.set_a(result, init);
            self.set_b(result, cond);
            self.set_c(result, step);
            self.set_d(result, body);
        } else if k == TK_KW_MATCH {
            result = self.parse_match();
        } else if k == TK_KW_FOREACH && !self.core {
            result = self.parse_foreach();
        } else if k == TK_KW_BREAK || k == TK_KW_CONTINUE {
            let kind = if k == TK_KW_CONTINUE {
                NK_CONTINUE
            } else {
                NK_BREAK
            };
            let tok = self.advance();
            result = self.make(kind, tok);
            self.end_stmt();
        } else if k == TK_KW_RETURN {
            let tok = self.advance();
            let mut value = 0;
            let next = self.kind();
            if next != TK_SEMI && next != TK_RBRACE && next != TK_EOF {
                value = self.parse_expr();
            }
            result = self.make(NK_RETURN, tok);
            self.set_a(result, value);
            self.end_stmt();
        } else if k == TK_KW_STRUCT || k == TK_KW_ENUM || k == TK_KW_CONST || k == TK_KW_STATIC {
            self.err_not_core(
                "local items (`struct`, `enum`, `const` and `static` inside a block)",
                self.pos,
            );
        } else {
            result = self.parse_simple();
            self.end_stmt();
        }
        self.depth = saved;
        result
    }

    fn paren_condition(&mut self) -> u32 {
        self.expect(TK_LPAREN);
        let e = self.parse_expr();
        self.expect(TK_RPAREN);
        e
    }

    fn parse_block(&mut self) -> u32 {
        let mut stmts = no_nodes();
        let open = self.expect(TK_LBRACE);
        let block = self.make(NK_BLOCK, open);
        loop {
            let k = self.kind();
            if k == TK_RBRACE {
                break;
            }
            if k == TK_EOF {
                self.err_expected_kind(TK_RBRACE);
            }
            let s = self.parse_statement();
            self.add(&mut stmts, s);
        }
        self.advance();
        self.set_a(block, stmts.head);
        block
    }

    fn parse_simple(&mut self) -> u32 {
        if self.kind() == TK_KW_VAR {
            self.advance();
            let name = self.expect(TK_IDENT);
            self.expect(TK_EQ);
            let value = self.parse_expr();
            let v = self.make(NK_VAR, name);
            self.set_b(v, value);
            return v;
        }
        let start = self.pos;
        let m = self.save();
        let ty = self.try_type();
        if ty != 0 && self.kind() == TK_IDENT {
            let next = self.kind_at(1);
            if next == TK_EQ || next == TK_SEMI || next == TK_RBRACE || next == TK_EOF {
                let name = self.advance();
                let mut value = 0;
                if self.accept(TK_EQ) {
                    value = self.parse_expr();
                }
                let v = self.make(NK_VAR, name);
                self.set_a(v, ty);
                self.set_b(v, value);
                return v;
            }
        }
        self.restore(m);
        let e = self.parse_expr();
        let k = self.kind();
        if tok_class(k) & TC_ASSIGN != 0 {
            let op = self.advance();
            if !self.is_place(e) {
                self.err_at(E_BAD_ASSIGN_TARGET, op as usize);
            }
            let value = self.parse_expr();
            let a = self.make(NK_ASSIGN, op);
            self.set_a(a, e);
            self.set_b(a, value);
            return a;
        }
        if k == TK_PLUS_PLUS || k == TK_MINUS_MINUS {
            let kind = if k == TK_MINUS_MINUS { NK_DEC } else { NK_INC };
            let op = self.advance();
            if !self.is_place(e) {
                self.err_at(E_BAD_ASSIGN_TARGET, op as usize);
            }
            let s = self.make(kind, op);
            self.set_a(s, e);
            return s;
        }
        let stmt = self.make(NK_EXPR, start as u32);
        self.set_a(stmt, e);
        stmt
    }

    fn parse_match(&mut self) -> u32 {
        let mut arms = no_nodes();
        let tok = self.advance();
        let subject = self.parse_expr();
        let m = self.make(NK_MATCH, tok);
        self.set_a(m, subject);
        self.expect(TK_LBRACE);
        loop {
            let k = self.kind();
            if k == TK_RBRACE {
                break;
            }
            if k == TK_EOF {
                self.err_expected_kind(TK_RBRACE);
            }
            let mut patterns = no_nodes();
            loop {
                let pat;
                if !self.core {
                    pat = self.parse_pattern();
                } else if self.kind() == TK_KW_UNDERSCORE {
                    let wt = self.advance();
                    pat = self.make(NK_WILD, wt);
                } else {
                    pat = self.parse_prefix();
                }
                self.add(&mut patterns, pat);
                if !self.accept(TK_PIPE) {
                    break;
                }
            }
            let mut guard = 0;
            if !self.core && self.accept(TK_KW_IF) {
                guard = self.parse_expr();
            }
            let arrow = self.expect(TK_FAT_ARROW);
            let body;
            if self.kind() == TK_LBRACE {
                body = self.parse_block();
                self.accept(TK_COMMA);
            } else {
                body = self.parse_simple();
                if !self.accept(TK_COMMA) && self.kind() != TK_RBRACE {
                    self.err_expected("`,` or `}`");
                }
            }
            let arm = self.make(NK_ARM, arrow);
            self.set_a(arm, patterns.head);
            self.set_b(arm, body);
            self.set_e(arm, guard);
            self.add(&mut arms, arm);
        }
        self.advance();
        self.set_b(m, arms.head);
        m
    }

    // ------------------------------------------------------------------ expressions

    fn parse_expr(&mut self) -> u32 {
        let saved = self.depth;
        self.enter();
        let e = if self.core {
            self.parse_binary(1)
        } else {
            self.parse_cond()
        };
        self.depth = saved;
        e
    }

    fn parse_binary(&mut self, min_level: u32) -> u32 {
        let saved = self.depth;
        let mut left = self.parse_cast();
        loop {
            let op = self.kind();
            let cls = tok_class(op);
            if cls & TC_NOT_CORE != 0 {
                if self.core {
                    self.err_not_core("", self.pos);
                }
                break;
            }
            let level = cls & TC_PREC;
            if level == 0 || level < min_level {
                break;
            }
            if level == PREC_COMPARE {
                let l = self.at(left);
                if l.kind == NK_BINARY
                    && l.flags & NF_PAREN == 0
                    && tok_class(l.op) & TC_PREC == PREC_COMPARE
                {
                    self.err_at(E_CHAINED_COMPARISON, self.pos);
                }
            }
            self.enter();
            let tok = self.advance();
            let right = self.parse_binary(level + 1);
            let b = self.make(NK_BINARY, tok);
            self.set_op(b, op);
            self.set_a(b, left);
            self.set_b(b, right);
            left = b;
        }
        self.depth = saved;
        left
    }

    fn parse_cast(&mut self) -> u32 {
        let saved = self.depth;
        let mut e = self.parse_prefix();
        while self.kind() == TK_KW_AS {
            self.enter();
            let tok = self.advance();
            let ty = self.parse_type();
            let c = self.make(NK_CAST, tok);
            self.set_a(c, e);
            self.set_b(c, ty);
            e = c;
        }
        self.depth = saved;
        e
    }

    fn parse_prefix(&mut self) -> u32 {
        let saved = self.depth;
        self.enter();
        let k = self.kind();
        let result;
        if k == TK_MINUS || k == TK_BANG || k == TK_TILDE || k == TK_STAR || k == TK_AMP {
            let tok = self.advance();
            let operand = self.parse_prefix();
            result = self.make(NK_UNARY, tok);
            self.set_op(result, k);
            self.set_a(result, operand);
        } else {
            result = self.parse_postfix();
        }
        self.depth = saved;
        result
    }

    fn parse_postfix(&mut self) -> u32 {
        let saved = self.depth;
        let mut e = self.parse_primary();
        loop {
            let k = self.kind();
            if k == TK_LPAREN {
                self.enter();
                let tok = self.advance();
                let mut args = no_nodes();
                while self.kind() != TK_RPAREN {
                    let ref_arg = !self.core && self.kind() == TK_KW_REF;
                    if ref_arg {
                        self.advance();
                    }
                    let a = self.parse_expr();
                    if ref_arg {
                        self.add_flags(a, NF_REF);
                    }
                    if self.kind() == TK_COLON {
                        self.err_not_core("named arguments", self.pos);
                    }
                    self.add(&mut args, a);
                    if !self.accept(TK_COMMA) {
                        break;
                    }
                }
                self.expect(TK_RPAREN);
                let call = self.make(NK_CALL, tok);
                self.set_a(call, e);
                self.set_b(call, args.head);
                e = call;
            } else if k == TK_LBRACKET {
                self.enter();
                let tok = self.advance();
                let index = self.parse_expr();
                self.expect(TK_RBRACKET);
                let n = self.make(NK_INDEX, tok);
                self.set_a(n, e);
                self.set_b(n, index);
                e = n;
            } else if k == TK_DOT {
                self.enter();
                self.advance();
                let name = self.expect(TK_IDENT);
                let n = self.make(NK_MEMBER, name);
                self.set_a(n, e);
                e = n;
            } else if k == TK_QUESTION && !self.core && !self.space_before(self.pos) {
                self.enter();
                let q = self.advance();
                let n = self.make(NK_TRY, q);
                self.set_a(n, e);
                e = n;
            } else if k == TK_LT
                && !self.core
                && self.is_name_like(e)
                && !self.space_before(self.pos)
            {
                let args = self.try_expr_type_args();
                if args == 0 {
                    break;
                }
                self.set_e(e, args);
            } else {
                break;
            }
        }
        self.depth = saved;
        e
    }

    fn parse_primary(&mut self) -> u32 {
        let k = self.kind();
        if !self.core {
            let s1 = self.parse_primary_s1();
            if s1 != 0 {
                return s1;
            }
        }
        let mut leaf = NK_NONE;
        if k == TK_INT {
            leaf = NK_INT;
        } else if k == TK_FLOAT && !self.core {
            leaf = NK_FLOAT;
        } else if k == TK_CHAR {
            leaf = NK_CHAR;
        } else if k == TK_STRING {
            leaf = NK_STRING;
        } else if k == TK_IDENT {
            leaf = NK_NAME;
        } else if k == TK_KW_TRUE || k == TK_KW_FALSE {
            leaf = NK_BOOL;
        }
        if leaf != NK_NONE {
            let tok = self.advance();
            return self.make(leaf, tok);
        }
        if k == TK_LPAREN {
            self.advance();
            let e = self.parse_expr();
            if self.kind() == TK_COMMA {
                self.err_not_core("tuples", self.pos);
            }
            self.expect(TK_RPAREN);
            self.add_flags(e, NF_PAREN);
            return e;
        }
        if k == TK_KW_SIZEOF || k == TK_KW_ALIGNOF {
            let kind = if k == TK_KW_ALIGNOF {
                NK_ALIGNOF
            } else {
                NK_SIZEOF
            };
            let tok = self.advance();
            self.expect(TK_LPAREN);
            let ty = self.parse_decl_type();
            self.expect(TK_RPAREN);
            let n = self.make(kind, tok);
            self.set_a(n, ty);
            return n;
        }
        if k == TK_KW_NEW {
            let tok = self.advance();
            let ty = self.parse_decl_type();
            if self.kind() == TK_LPAREN {
                self.err_not_core("constructors", self.pos);
            }
            self.expect(TK_LBRACE);
            let mut inits = no_nodes();
            while self.kind() != TK_RBRACE {
                let name = self.expect(TK_IDENT);
                self.expect(TK_EQ);
                let value = self.parse_expr();
                let init = self.make(NK_INIT, name);
                self.set_a(init, value);
                self.add(&mut inits, init);
                if self.accept(TK_COMMA) {
                    continue;
                }
                if self.kind() == TK_RBRACE {
                    break;
                }
                self.err_expected("`,` or `}`");
            }
            self.advance();
            let n = self.make(NK_NEW, tok);
            self.set_a(n, ty);
            self.set_b(n, inits.head);
            return n;
        }
        if k == TK_FLOAT {
            self.err_not_core("floating-point literals", self.pos);
        }
        if k == TK_LBRACKET {
            self.err_not_core("array literals", self.pos);
        }
        if (TK_KW_FIRST..=TK_KW_LAST).contains(&k) && tok_class(k) & TC_CORE_KW == 0 {
            self.err_not_core("", self.pos);
        }
        self.err_expected("an expression")
    }

    // ================================================================== Envo-S1 (parser_s1.envo)

    fn is_word(&self, word: &[u8]) -> bool {
        self.kind() == TK_IDENT && self.tok_text(self.pos) == word
    }

    /// Consumes one `>` closing a generic argument list; `>>`, `>=` and `>>=` give up their
    /// first `>`. Returns false if there is no `>`.
    fn accept_gt(&mut self) -> bool {
        let k = self.kind();
        if k == TK_GT {
            self.advance();
            return true;
        }
        let rest = if k == TK_SHR {
            TK_GT
        } else if k == TK_GT_EQ {
            TK_EQ
        } else if k == TK_SHR_EQ {
            TK_GT_EQ
        } else {
            return false;
        };
        let at = self.pos;
        let mut t = self.toks[at];
        self.undo.push(Split { index: at, was: t });
        t.kind = rest;
        t.start += 1;
        t.len -= 1;
        t.flags = 0;
        self.toks[at] = t;
        true
    }

    fn try_type_args(&mut self) -> u32 {
        let m = self.save();
        self.advance();
        let mut args = no_nodes();
        loop {
            let t = self.try_type();
            if t == 0 {
                self.restore(m);
                return 0;
            }
            self.add(&mut args, t);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        if !self.accept_gt() {
            self.restore(m);
            return 0;
        }
        args.head
    }

    fn try_type_s1(&mut self) -> u32 {
        if self.is_word(b"mut") || self.is_word(b"once") {
            let once = self.is_word(b"once");
            let m = self.save();
            self.advance();
            if self.is_word(b"fn") && self.kind_at(1) == TK_LPAREN {
                let t = self.try_fn_type();
                if t != 0 {
                    self.add_flags(t, if once { NF_ONCE } else { NF_MUT });
                    return t;
                }
            } else if !once {
                let inner = self.try_type_s1();
                if inner != 0 && self.at(inner).kind == NK_TSLICE {
                    self.add_flags(inner, NF_MUT);
                    return inner;
                }
            }
            self.restore(m);
            return 0;
        }
        if self.is_word(b"fn") && self.kind_at(1) == TK_LPAREN {
            return self.try_fn_type();
        }
        let mut stars = 0usize;
        let first_star = self.pos;
        while self.kind() == TK_STAR {
            self.advance();
            stars += 1;
            if stars > MAX_DEPTH {
                return 0;
            }
        }
        if self.kind() != TK_IDENT {
            return 0;
        }
        let mut quals = no_nodes();
        let mut name_tok = self.advance();
        while self.kind() == TK_DOT && self.kind_at(1) == TK_IDENT {
            let q = self.make(NK_NAME, name_tok);
            self.add(&mut quals, q);
            self.advance();
            name_tok = self.advance();
        }
        let mut ty = self.make(NK_TNAME, name_tok);
        self.set_f(ty, quals.head);
        if self.kind() == TK_LT {
            let args = self.try_type_args();
            if args == 0 {
                return 0;
            }
            self.set_e(ty, args);
        }
        let mut suffixes = 0usize;
        loop {
            let k = self.kind();
            if k == TK_QUESTION && !self.space_before(self.pos) {
                let q = self.advance();
                let opt = self.make(NK_TOPT, q);
                self.set_a(opt, ty);
                ty = opt;
                continue;
            }
            if k != TK_LBRACKET {
                break;
            }
            let k1 = self.kind_at(1);
            if k1 == TK_RBRACKET {
                suffixes += 1;
                if suffixes > MAX_DEPTH {
                    return 0;
                }
                let open = self.advance();
                self.advance();
                let slice = self.make(NK_TSLICE, open);
                self.set_a(slice, ty);
                ty = slice;
                continue;
            }
            let size_kind = if k1 == TK_IDENT {
                NK_NAME
            } else if k1 != TK_INT {
                break;
            } else {
                NK_INT
            };
            if self.kind_at(2) != TK_RBRACKET {
                break;
            }
            suffixes += 1;
            if suffixes > MAX_DEPTH {
                return 0;
            }
            let open = self.advance();
            let size_tok = self.advance();
            let size = self.make(size_kind, size_tok);
            self.advance();
            let array = self.make(NK_TARRAY, open);
            self.set_a(array, ty);
            self.set_b(array, size);
            ty = array;
        }
        while stars > 0 {
            stars -= 1;
            let ptr = self.make(NK_TPTR, (first_star + stars) as u32);
            self.set_a(ptr, ty);
            ty = ptr;
        }
        ty
    }

    fn try_fn_type(&mut self) -> u32 {
        let m = self.save();
        let fn_tok = self.advance();
        self.advance();
        let mut params = no_nodes();
        while self.kind() != TK_RPAREN {
            let mut mode = 0;
            if self.kind() == TK_KW_IN {
                mode = NF_IN;
            } else if self.kind() == TK_KW_REF {
                mode = NF_REF;
            }
            if mode != 0 {
                self.advance();
            }
            let t = self.try_type_s1();
            if t == 0 {
                self.restore(m);
                return 0;
            }
            self.add_flags(t, mode);
            self.add(&mut params, t);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        if !self.accept(TK_RPAREN) {
            self.restore(m);
            return 0;
        }
        let mut result = 0;
        if self.accept(TK_ARROW) {
            result = self.try_type_s1();
            if result == 0 {
                self.restore(m);
                return 0;
            }
        }
        let t = self.make(NK_TFN, fn_tok);
        self.set_a(t, params.head);
        self.set_b(t, result);
        t
    }

    fn parse_type_params(&mut self) -> u32 {
        let mut params = no_nodes();
        self.expect(TK_LT);
        loop {
            if self.kind() == TK_KW_CONST {
                self.err_not_core("const generics", self.pos);
            }
            let name = self.expect(TK_IDENT);
            let tp = self.make(NK_TPARAM, name);
            self.add(&mut params, tp);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        if !self.accept_gt() {
            self.err_expected_kind(TK_GT);
        }
        params.head
    }

    fn parse_where(&mut self) -> u32 {
        let mut clauses = no_nodes();
        self.advance();
        loop {
            let name = self.expect(TK_IDENT);
            let w = self.make(NK_WHERE, name);
            self.expect(TK_COLON);
            let mut bounds = no_nodes();
            loop {
                let b = self.parse_type();
                self.add(&mut bounds, b);
                if self.kind() != TK_COMMA {
                    break;
                }
                if self.kind_at(1) == TK_IDENT && self.kind_at(2) == TK_COLON {
                    break;
                }
                self.advance();
            }
            self.set_a(w, bounds.head);
            self.add(&mut clauses, w);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        clauses.head
    }

    fn parse_origins(&mut self) -> u32 {
        let mut origins = no_nodes();
        self.advance();
        loop {
            if self.kind() == TK_KW_THIS {
                let tok = self.advance();
                let o = self.make(NK_THIS, tok);
                self.add(&mut origins, o);
            } else {
                let tok = self.expect(TK_IDENT);
                let o = self.make(NK_NAME, tok);
                self.add(&mut origins, o);
            }
            if self.kind() != TK_COMMA {
                break;
            }
            let k2 = self.kind_at(2);
            if k2 != TK_COMMA
                && k2 != TK_RPAREN
                && k2 != TK_LBRACE
                && k2 != TK_FAT_ARROW
                && k2 != TK_KW_WHERE
                && k2 != TK_SEMI
            {
                break;
            }
            self.advance();
        }
        origins.head
    }

    fn parse_param_s1(&mut self) -> u32 {
        let mut mode = 0;
        let k = self.kind();
        if k == TK_KW_OUT {
            self.err_not_core("", self.pos);
        }
        if k == TK_KW_IN {
            mode = NF_IN;
        }
        if k == TK_KW_REF {
            mode = NF_REF;
        }
        if mode != 0 {
            self.advance();
        }
        if self.kind() == TK_KW_THIS {
            let tok = self.advance();
            let recv = self.make(NK_PARAM, tok);
            self.set_flags(recv, mode);
            return recv;
        }
        let ty = self.parse_type();
        let name = self.expect(TK_IDENT);
        let param = self.make(NK_PARAM, name);
        self.set_a(param, ty);
        self.set_flags(param, mode);
        if self.is_word(b"from") {
            let origins = self.parse_origins();
            self.set_e(param, origins);
        }
        if self.kind() == TK_EQ {
            self.err_not_core("default parameter values", self.pos);
        }
        param
    }

    fn parse_function_rest(&mut self, f: u32, in_interface: bool) {
        if self.kind() == TK_LT {
            let tps = self.parse_type_params();
            self.set_e(f, tps);
        }
        self.expect(TK_LPAREN);
        let mut params = no_nodes();
        while self.kind() != TK_RPAREN {
            let param = self.parse_param_s1();
            self.add(&mut params, param);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        self.expect(TK_RPAREN);
        if self.is_word(b"from") {
            let origins = self.parse_origins();
            self.set_g(f, origins);
        }
        if self.kind() == TK_KW_WHERE {
            let clauses = self.parse_where();
            self.set_f(f, clauses);
        }
        let mut body = 0;
        if self.kind() == TK_LBRACE {
            body = self.parse_block();
        } else if self.kind() == TK_FAT_ARROW {
            self.advance();
            body = self.parse_expr();
            self.end_stmt();
        } else if in_interface && self.kind() == TK_SEMI {
            self.advance();
        } else {
            self.err_expected("`{` or `=>`");
        }
        self.set_b(f, params.head);
        self.set_c(f, body);
    }

    fn parse_modifiers(&mut self) -> u32 {
        let mut flags = 0;
        loop {
            let k = self.kind();
            if k == TK_KW_PUBLIC {
                flags |= NF_PUBLIC;
            } else if k == TK_KW_INTERNAL {
                flags |= NF_INTERNAL;
            } else if k == TK_KW_STATIC {
                flags |= NF_STATIC;
            } else if k != TK_KW_PRIVATE {
                break;
            }
            self.advance();
        }
        flags
    }

    fn parse_method(&mut self, flags: u32, in_interface: bool) -> u32 {
        let ty = self.parse_type();
        let name = self.expect(TK_IDENT);
        let f = self.make(NK_FN, name);
        self.set_a(f, ty);
        self.set_flags(f, flags);
        self.parse_function_rest(f, in_interface);
        f
    }

    fn parse_struct_body(&mut self, st: u32) {
        let mut fields = no_nodes();
        let mut methods = no_nodes();
        while self.kind() != TK_RBRACE {
            if self.kind() == TK_EOF {
                self.err_expected_kind(TK_RBRACE);
            }
            let flags = self.parse_modifiers();
            let m = self.save();
            self.parse_type();
            self.expect(TK_IDENT);
            let k = self.kind();
            if k == TK_LPAREN || k == TK_LT {
                self.restore(m);
                let method = self.parse_method(flags, false);
                self.add(&mut methods, method);
                continue;
            }
            self.restore(m);
            let ty = self.parse_type();
            let name = self.expect(TK_IDENT);
            let field = self.make(NK_FIELD, name);
            self.set_a(field, ty);
            self.set_flags(field, flags);
            let k = self.kind();
            if k == TK_LBRACE || k == TK_FAT_ARROW {
                self.err_not_core("properties", self.pos);
            }
            if k == TK_EQ {
                self.err_not_core("field initializers", self.pos);
            }
            self.end_stmt();
            self.add(&mut fields, field);
        }
        self.advance();
        self.set_a(st, fields.head);
        self.set_g(st, methods.head);
    }

    fn parse_bases(&mut self) -> u32 {
        let mut bases = no_nodes();
        self.advance();
        loop {
            let b = self.parse_type();
            self.add(&mut bases, b);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        bases.head
    }

    fn parse_payload(&mut self) -> u32 {
        let mut fields = no_nodes();
        self.advance();
        while self.kind() != TK_RPAREN {
            let ty = self.parse_type();
            let name = self.expect(TK_IDENT);
            let f = self.make(NK_PARAM, name);
            self.set_a(f, ty);
            self.add(&mut fields, f);
            if !self.accept(TK_COMMA) {
                break;
            }
        }
        self.expect(TK_RPAREN);
        fields.head
    }

    fn parse_namespace(&mut self) -> u32 {
        let tok = self.advance();
        let ns = self.make(NK_NAMESPACE, tok);
        let path = self.parse_path();
        self.set_a(ns, path);
        self.end_stmt();
        ns
    }

    fn parse_path(&mut self) -> u32 {
        let mut path = no_nodes();
        loop {
            let tok = self.expect(TK_IDENT);
            let n = self.make(NK_NAME, tok);
            self.add(&mut path, n);
            if self.kind() != TK_DOT || self.kind_at(1) != TK_IDENT {
                break;
            }
            self.advance();
        }
        path.head
    }

    fn parse_use(&mut self) -> u32 {
        let tok = self.advance();
        let u = self.make(NK_USE, tok);
        let path = self.parse_path();
        self.set_a(u, path);
        if self.kind() == TK_DOT && self.kind_at(1) == TK_LBRACE {
            self.advance();
            self.advance();
            let mut names = no_nodes();
            while self.kind() != TK_RBRACE {
                let nt = self.expect(TK_IDENT);
                let n = self.make(NK_NAME, nt);
                self.add(&mut names, n);
                if !self.accept(TK_COMMA) {
                    break;
                }
            }
            self.expect(TK_RBRACE);
            self.set_b(u, names.head);
        } else if self.accept(TK_KW_AS) {
            let at = self.expect(TK_IDENT);
            let alias = self.make(NK_NAME, at);
            self.set_c(u, alias);
        }
        self.end_stmt();
        u
    }

    fn parse_interface(&mut self) -> u32 {
        self.advance();
        let name = self.expect(TK_IDENT);
        let it = self.make(NK_INTERFACE, name);
        if self.kind() == TK_LT {
            let tps = self.parse_type_params();
            self.set_e(it, tps);
        }
        self.expect(TK_LBRACE);
        let mut methods = no_nodes();
        while self.kind() != TK_RBRACE {
            if self.kind() == TK_EOF {
                self.err_expected_kind(TK_RBRACE);
            }
            let flags = self.parse_modifiers();
            let method = self.parse_method(flags, true);
            self.add(&mut methods, method);
        }
        self.advance();
        self.set_a(it, methods.head);
        it
    }

    fn parse_impl(&mut self) -> u32 {
        let tok = self.advance();
        let im = self.make(NK_IMPL, tok);
        if self.kind() == TK_LT {
            let tps = self.parse_type_params();
            self.set_e(im, tps);
        }
        let first = self.parse_type();
        if self.accept(TK_KW_FOR) {
            self.set_b(im, first);
            let target = self.parse_type();
            self.set_a(im, target);
        } else {
            self.set_a(im, first);
        }
        self.expect(TK_LBRACE);
        let mut methods = no_nodes();
        while self.kind() != TK_RBRACE {
            if self.kind() == TK_EOF {
                self.err_expected_kind(TK_RBRACE);
            }
            let flags = self.parse_modifiers();
            let method = self.parse_method(flags, false);
            self.add(&mut methods, method);
        }
        self.advance();
        self.set_c(im, methods.head);
        im
    }

    fn parse_foreach(&mut self) -> u32 {
        let tok = self.advance();
        self.expect(TK_LPAREN);
        self.expect(TK_KW_VAR);
        let nt = self.expect(TK_IDENT);
        let name = self.make(NK_NAME, nt);
        self.expect(TK_KW_IN);
        let seq = self.parse_expr();
        self.expect(TK_RPAREN);
        let body = self.parse_statement();
        let n = self.make(NK_FOREACH, tok);
        self.set_a(n, name);
        self.set_b(n, seq);
        self.set_c(n, body);
        n
    }

    fn parse_pattern(&mut self) -> u32 {
        let saved = self.in_pattern;
        self.in_pattern = true;
        let pat = if self.kind() == TK_KW_UNDERSCORE {
            let tok = self.advance();
            self.make(NK_WILD, tok)
        } else {
            self.parse_prefix()
        };
        self.in_pattern = saved;
        pat
    }

    fn parse_cond(&mut self) -> u32 {
        let c = self.parse_range();
        if self.kind() == TK_QUESTION && self.space_before(self.pos) {
            let tok = self.advance();
            let a = self.parse_expr();
            self.expect(TK_COLON);
            let b = self.parse_cond();
            let n = self.make(NK_COND, tok);
            self.set_a(n, c);
            self.set_b(n, a);
            self.set_c(n, b);
            return n;
        }
        c
    }

    fn starts_expr(&self) -> bool {
        let k = self.kind();
        !(k == TK_RBRACKET
            || k == TK_RPAREN
            || k == TK_COMMA
            || k == TK_SEMI
            || k == TK_RBRACE
            || k == TK_LBRACE
            || k == TK_EOF
            || k == TK_COLON
            || k == TK_FAT_ARROW)
    }

    fn parse_range(&mut self) -> u32 {
        let lo = self.parse_coalesce();
        let k = self.kind();
        if k == TK_DOT_DOT || k == TK_DOT_DOT_EQ {
            return self.parse_range_rest(lo);
        }
        lo
    }

    fn parse_range_rest(&mut self, lo: u32) -> u32 {
        let k = self.kind();
        let tok = self.advance();
        let mut hi = 0;
        if self.starts_expr() {
            hi = self.parse_coalesce();
        } else if k == TK_DOT_DOT_EQ {
            self.err_expected("an expression");
        }
        let n = self.make(NK_RANGE, tok);
        self.set_op(n, k);
        self.set_a(n, lo);
        self.set_b(n, hi);
        n
    }

    fn parse_coalesce(&mut self) -> u32 {
        let l = self.parse_binary(1);
        if self.kind() == TK_QQ {
            let tok = self.advance();
            let r = self.parse_coalesce();
            let n = self.make(NK_BINARY, tok);
            self.set_op(n, TK_QQ);
            self.set_a(n, l);
            self.set_b(n, r);
            return n;
        }
        l
    }

    fn at_lambda(&self) -> bool {
        if self.in_pattern {
            return false;
        }
        let mut skip = 0usize;
        if self.is_word(b"move") && (self.kind_at(1) == TK_IDENT || self.kind_at(1) == TK_LPAREN) {
            skip = 1;
        }
        let k = self.kind_at(skip);
        if k == TK_IDENT {
            return self.kind_at(skip + 1) == TK_FAT_ARROW;
        }
        if k != TK_LPAREN {
            return false;
        }
        let mut depth = 0usize;
        let mut i = skip;
        loop {
            let t = self.kind_at(i);
            if t == TK_EOF {
                return false;
            }
            if t == TK_LPAREN {
                depth += 1;
            }
            if t == TK_RPAREN {
                depth -= 1;
                if depth == 0 {
                    return self.kind_at(i + 1) == TK_FAT_ARROW;
                }
            }
            i += 1;
        }
    }

    fn parse_lambda(&mut self) -> u32 {
        let mut flags = 0;
        if self.is_word(b"move") {
            self.advance();
            flags = NF_MOVE;
        }
        let mut params = no_nodes();
        if self.kind() == TK_IDENT {
            let tok = self.advance();
            let param = self.make(NK_PARAM, tok);
            self.add(&mut params, param);
        } else {
            self.expect(TK_LPAREN);
            while self.kind() != TK_RPAREN {
                let mut mode = 0;
                if self.kind() == TK_KW_IN {
                    mode = NF_IN;
                } else if self.kind() == TK_KW_REF {
                    mode = NF_REF;
                }
                if mode != 0 {
                    self.advance();
                }
                let mut ty = 0;
                let after = self.kind_at(1);
                if !(self.kind() == TK_IDENT && (after == TK_COMMA || after == TK_RPAREN)) {
                    ty = self.parse_type();
                }
                let name = self.expect(TK_IDENT);
                let param = self.make(NK_PARAM, name);
                self.set_a(param, ty);
                self.set_flags(param, mode);
                self.add(&mut params, param);
                if !self.accept(TK_COMMA) {
                    break;
                }
            }
            self.expect(TK_RPAREN);
        }
        let tok = self.expect(TK_FAT_ARROW);
        let body = if self.kind() == TK_LBRACE {
            self.parse_block()
        } else {
            self.parse_expr()
        };
        let n = self.make(NK_LAMBDA, tok);
        self.set_a(n, params.head);
        self.set_b(n, body);
        self.set_flags(n, flags);
        n
    }

    fn parse_match_expr(&mut self) -> u32 {
        let mut arms = no_nodes();
        let tok = self.advance();
        let subject = self.parse_expr();
        let m = self.make(NK_MATCH, tok);
        self.set_a(m, subject);
        self.set_flags(m, NF_EXPR);
        self.expect(TK_LBRACE);
        loop {
            let k = self.kind();
            if k == TK_RBRACE {
                break;
            }
            if k == TK_EOF {
                self.err_expected_kind(TK_RBRACE);
            }
            let mut patterns = no_nodes();
            loop {
                let pat = self.parse_pattern();
                self.add(&mut patterns, pat);
                if !self.accept(TK_PIPE) {
                    break;
                }
            }
            let mut guard = 0;
            if self.accept(TK_KW_IF) {
                guard = self.parse_expr();
            }
            let arrow = self.expect(TK_FAT_ARROW);
            let body = if self.kind() == TK_LBRACE {
                self.parse_block()
            } else {
                self.parse_expr()
            };
            let arm = self.make(NK_ARM, arrow);
            self.set_a(arm, patterns.head);
            self.set_b(arm, body);
            self.set_e(arm, guard);
            self.add(&mut arms, arm);
            if self.accept(TK_COMMA) {
                continue;
            }
            if self.kind() == TK_RBRACE {
                continue;
            }
            if self.toks[self.pos].flags & TF_NEWLINE_BEFORE != 0 {
                continue;
            }
            self.err_expected("`,` or `}`");
        }
        self.advance();
        self.set_b(m, arms.head);
        m
    }

    fn try_expr_type_args(&mut self) -> u32 {
        let m = self.save();
        let args = self.try_type_args();
        if args == 0 {
            return 0;
        }
        if self.kind() == TK_LPAREN || self.kind() == TK_DOT {
            return args;
        }
        self.restore(m);
        0
    }

    fn parse_primary_s1(&mut self) -> u32 {
        let k = self.kind();
        if k == TK_KW_NULL {
            let tok = self.advance();
            return self.make(NK_NULL, tok);
        }
        if k == TK_KW_THIS {
            let tok = self.advance();
            return self.make(NK_THIS, tok);
        }
        if k == TK_KW_MATCH {
            return self.parse_match_expr();
        }
        if k == TK_DOT_DOT || k == TK_DOT_DOT_EQ {
            return self.parse_range_rest(0);
        }
        if k == TK_KW_UNDERSCORE && self.in_pattern {
            let tok = self.advance();
            return self.make(NK_WILD, tok);
        }
        if (k == TK_IDENT || k == TK_LPAREN) && self.at_lambda() {
            return self.parse_lambda();
        }
        0
    }

    fn parse_toplevel_s1(&mut self) -> u32 {
        let mut flags = 0;
        let mut k = self.kind();
        let visibility = k == TK_KW_PUBLIC || k == TK_KW_PRIVATE || k == TK_KW_INTERNAL;
        if visibility {
            if k == TK_KW_PUBLIC {
                flags = NF_PUBLIC;
            }
            if k == TK_KW_INTERNAL {
                flags = NF_INTERNAL;
            }
            self.advance();
            k = self.kind();
        }
        let item = if k == TK_KW_STRUCT {
            self.parse_struct()
        } else if k == TK_KW_ENUM {
            self.parse_enum()
        } else if k == TK_KW_CONST {
            self.parse_binding(NK_CONST)
        } else if k == TK_KW_STATIC {
            self.parse_binding(NK_STATIC)
        } else if k == TK_KW_INTERFACE {
            self.parse_interface()
        } else if k == TK_KW_IMPL && !visibility {
            self.parse_impl()
        } else if k == TK_KW_NAMESPACE && !visibility {
            self.parse_namespace()
        } else if k == TK_KW_USE && !visibility {
            self.parse_use()
        } else if self.is_fn_start() {
            self.parse_function()
        } else if visibility {
            self.err_expected("a function, `struct`, `enum`, `interface`, `const` or `static`")
        } else {
            return self.parse_statement();
        };
        self.add_flags(item, flags);
        item
    }
}
