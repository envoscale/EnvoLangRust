//! syntax/node.envol: syntax tree nodes. A tree is a Vec<Node> addressed by index; 0 means
//! "no node". A child slot may hold a list: its first node, the rest linked through next.

use super::token::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Node {
    pub kind: u16,
    pub op: u16,
    pub tok: u32,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
    pub next: u32,
    pub flags: u32,
    pub e: u32,
    pub f: u32,
    pub g: u32,
}

pub const NF_PAREN: u32 = 1;
pub const NF_IN: u32 = 2;
pub const NF_REF: u32 = 4;
pub const NF_MUT: u32 = 8;
pub const NF_ONCE: u32 = 16;
pub const NF_MOVE: u32 = 32;
pub const NF_STATIC: u32 = 64;
pub const NF_PUBLIC: u32 = 128;
pub const NF_INTERNAL: u32 = 256;
pub const NF_EXPR: u32 = 512;
pub const NK_NONE: u16 = 0;
pub const NK_PROGRAM: u16 = 1;
pub const NK_FN: u16 = 2;
pub const NK_PARAM: u16 = 3;
pub const NK_STRUCT: u16 = 4;
pub const NK_FIELD: u16 = 5;
pub const NK_ENUM: u16 = 6;
pub const NK_VARIANT: u16 = 7;
pub const NK_CONST: u16 = 8;
pub const NK_STATIC: u16 = 9;
pub const NK_TNAME: u16 = 10;
pub const NK_TPTR: u16 = 11;
pub const NK_TARRAY: u16 = 12;
pub const NK_TSLICE: u16 = 13;
pub const NK_BLOCK: u16 = 14;
pub const NK_UNSAFE: u16 = 15;
pub const NK_IF: u16 = 16;
pub const NK_WHILE: u16 = 17;
pub const NK_FOR: u16 = 18;
pub const NK_MATCH: u16 = 19;
pub const NK_ARM: u16 = 20;
pub const NK_WILD: u16 = 21;
pub const NK_BREAK: u16 = 22;
pub const NK_CONTINUE: u16 = 23;
pub const NK_RETURN: u16 = 24;
pub const NK_VAR: u16 = 25;
pub const NK_EXPR: u16 = 26;
pub const NK_ASSIGN: u16 = 27;
pub const NK_INC: u16 = 28;
pub const NK_DEC: u16 = 29;
pub const NK_INT: u16 = 30;
pub const NK_CHAR: u16 = 31;
pub const NK_STRING: u16 = 32;
pub const NK_BOOL: u16 = 33;
pub const NK_NAME: u16 = 34;
pub const NK_CALL: u16 = 35;
pub const NK_INDEX: u16 = 36;
pub const NK_MEMBER: u16 = 37;
pub const NK_UNARY: u16 = 38;
pub const NK_BINARY: u16 = 39;
pub const NK_CAST: u16 = 40;
pub const NK_SIZEOF: u16 = 41;
pub const NK_ALIGNOF: u16 = 42;
pub const NK_NEW: u16 = 43;
pub const NK_INIT: u16 = 44;
pub const NK_NAMESPACE: u16 = 45;
pub const NK_USE: u16 = 46;
pub const NK_INTERFACE: u16 = 47;
pub const NK_IMPL: u16 = 48;
pub const NK_TPARAM: u16 = 49;
pub const NK_WHERE: u16 = 50;
pub const NK_TOPT: u16 = 51;
pub const NK_TFN: u16 = 52;
pub const NK_THIS: u16 = 53;
pub const NK_NULL: u16 = 54;
pub const NK_LAMBDA: u16 = 55;
pub const NK_TRY: u16 = 56;
pub const NK_COND: u16 = 57;
pub const NK_RANGE: u16 = 58;
pub const NK_FOREACH: u16 = 59;
pub const NK_FLOAT: u16 = 60;
pub const NK_COUNT: u16 = 61;
pub const TC_PREC: u32 = 15;
pub const TC_ASSIGN: u32 = 16;
pub const TC_NOT_CORE: u32 = 32;
pub const TC_CORE_KW: u32 = 64;
pub const PREC_COMPARE: u32 = 3;

pub fn node_name(kind: u16) -> &'static str {
    match kind {
        NK_PROGRAM => "program",
        NK_FN => "fn",
        NK_PARAM => "param",
        NK_STRUCT => "struct",
        NK_FIELD => "field",
        NK_ENUM => "enum",
        NK_VARIANT => "variant",
        NK_CONST => "const",
        NK_STATIC => "static",
        NK_TNAME => "type",
        NK_TPTR => "ptr",
        NK_TARRAY => "array",
        NK_TSLICE => "slice",
        NK_BLOCK => "block",
        NK_UNSAFE => "unsafe",
        NK_IF => "if",
        NK_WHILE => "while",
        NK_FOR => "for",
        NK_MATCH => "match",
        NK_ARM => "arm",
        NK_WILD => "wild",
        NK_BREAK => "break",
        NK_CONTINUE => "continue",
        NK_RETURN => "return",
        NK_VAR => "var",
        NK_EXPR => "expr",
        NK_ASSIGN => "assign",
        NK_INC => "inc",
        NK_DEC => "dec",
        NK_INT => "int",
        NK_CHAR => "char",
        NK_STRING => "string",
        NK_BOOL => "bool",
        NK_NAME => "name",
        NK_CALL => "call",
        NK_INDEX => "index",
        NK_MEMBER => "member",
        NK_UNARY => "unary",
        NK_BINARY => "binary",
        NK_CAST => "as",
        NK_SIZEOF => "sizeof",
        NK_ALIGNOF => "alignof",
        NK_NEW => "new",
        NK_INIT => "init",
        NK_NAMESPACE => "namespace",
        NK_USE => "use",
        NK_INTERFACE => "interface",
        NK_IMPL => "impl",
        NK_TPARAM => "tparam",
        NK_WHERE => "where",
        NK_TOPT => "optional",
        NK_TFN => "fntype",
        NK_THIS => "this",
        NK_NULL => "null",
        NK_LAMBDA => "lambda",
        NK_TRY => "try",
        NK_COND => "cond",
        NK_RANGE => "range",
        NK_FOREACH => "foreach",
        NK_FLOAT => "float",
        _ => "none",
    }
}

pub fn node_slots(kind: u16) -> u32 {
    match kind {
        NK_FN | NK_IF | NK_USE | NK_IMPL | NK_COND | NK_FOREACH => 3,
        NK_FOR => 4,
        NK_ENUM | NK_CONST | NK_STATIC | NK_TARRAY | NK_WHILE | NK_MATCH | NK_ARM | NK_VAR
        | NK_ASSIGN | NK_CALL | NK_INDEX | NK_BINARY | NK_CAST | NK_NEW | NK_TFN | NK_LAMBDA
        | NK_RANGE => 2,
        NK_PROGRAM | NK_PARAM | NK_STRUCT | NK_FIELD | NK_VARIANT | NK_TPTR | NK_TSLICE
        | NK_BLOCK | NK_UNSAFE | NK_RETURN | NK_EXPR | NK_INC | NK_DEC | NK_MEMBER | NK_UNARY
        | NK_SIZEOF | NK_ALIGNOF | NK_INIT | NK_NAMESPACE | NK_INTERFACE | NK_TPARAM | NK_WHERE
        | NK_TOPT | NK_TRY => 1,
        _ => 0,
    }
}

pub fn node_has_text(kind: u16) -> bool {
    matches!(
        kind,
        NK_FN
            | NK_PARAM
            | NK_STRUCT
            | NK_FIELD
            | NK_ENUM
            | NK_VARIANT
            | NK_CONST
            | NK_STATIC
            | NK_TNAME
            | NK_VAR
            | NK_ASSIGN
            | NK_INT
            | NK_FLOAT
            | NK_CHAR
            | NK_STRING
            | NK_BOOL
            | NK_NAME
            | NK_MEMBER
            | NK_UNARY
            | NK_BINARY
            | NK_INIT
            | NK_INTERFACE
            | NK_TPARAM
            | NK_WHERE
            | NK_RANGE
    )
}

/// Items, as opposed to statements, at the top level of a file.
pub fn node_is_item(kind: u16) -> bool {
    kind == NK_FN
        || kind == NK_STRUCT
        || kind == NK_ENUM
        || kind == NK_CONST
        || kind == NK_STATIC
        || kind == NK_INTERFACE
        || kind == NK_IMPL
        || kind == NK_NAMESPACE
        || kind == NK_USE
}

pub fn tok_class(kind: u16) -> u32 {
    match kind {
        TK_KW_UNDERSCORE | TK_KW_ALIGNOF | TK_KW_AS | TK_KW_BREAK | TK_KW_CONST
        | TK_KW_CONTINUE | TK_KW_ELSE | TK_KW_ENUM | TK_KW_FALSE | TK_KW_FOR | TK_KW_IF
        | TK_KW_MATCH | TK_KW_NEW | TK_KW_PRIVATE | TK_KW_PUBLIC | TK_KW_RETURN | TK_KW_SIZEOF
        | TK_KW_STATIC | TK_KW_STRUCT | TK_KW_TRUE | TK_KW_UNSAFE | TK_KW_VAR | TK_KW_WHILE => {
            TC_CORE_KW
        }
        TK_SHL_EQ | TK_SHR_EQ | TK_WRAP_ADD_EQ | TK_WRAP_SUB_EQ | TK_WRAP_MUL_EQ | TK_PLUS_EQ
        | TK_MINUS_EQ | TK_STAR_EQ | TK_SLASH_EQ | TK_PERCENT_EQ | TK_AMP_EQ | TK_PIPE_EQ
        | TK_CARET_EQ | TK_EQ => TC_ASSIGN,
        TK_DOT_DOT_EQ | TK_QQ_EQ | TK_FAT_ARROW | TK_ARROW | TK_COLON_COLON | TK_DOT_DOT
        | TK_QQ | TK_QUESTION => TC_NOT_CORE,
        TK_OR_OR => 1,
        TK_AND_AND => 2,
        TK_EQ_EQ | TK_NOT_EQ | TK_LT_EQ | TK_GT_EQ | TK_LT | TK_GT => 3,
        TK_PIPE => 4,
        TK_CARET => 5,
        TK_AMP => 6,
        TK_SHL | TK_SHR => 7,
        TK_PLUS | TK_MINUS | TK_WRAP_ADD | TK_WRAP_SUB => 8,
        TK_STAR | TK_SLASH | TK_PERCENT | TK_WRAP_MUL => 9,
        _ => 0,
    }
}

/// The Envo-S1 flags of a node, as the dump prints them: " [in mut ...]", or nothing.
pub fn append_node_flags(dst: &mut Vec<u8>, flags: u32) {
    let flags = flags & !(NF_PAREN | NF_PUBLIC | NF_INTERNAL);
    if flags == 0 {
        return;
    }
    dst.extend_from_slice(b" [");
    let mut first = true;
    let mut bit = 2u32;
    while bit <= NF_EXPR {
        if flags & bit != 0 {
            if !first {
                dst.push(b' ');
            }
            first = false;
            dst.extend_from_slice(match bit {
                NF_IN => b"in" as &[u8],
                NF_REF => b"ref",
                NF_MUT => b"mut",
                NF_ONCE => b"once",
                NF_MOVE => b"move",
                NF_STATIC => b"static",
                NF_PUBLIC => b"public",
                NF_INTERNAL => b"internal",
                _ => b"expr",
            });
        }
        bit <<= 1;
    }
    dst.push(b']');
}
