//! syntax/token.envo: token kinds and their spellings, numbered as in every stage so the
//! compilers can be compared token for token.

#[derive(Clone, Copy, Debug, Default)]
pub struct Token {
    pub kind: u16,
    pub flags: u16,
    pub start: u32,
    pub len: u32,
    pub line: u32,
    pub file: u32,
}

pub const TF_SPACE_BEFORE: u16 = 1;
pub const TF_NEWLINE_BEFORE: u16 = 2;
pub const TK_EOF: u16 = 0;
pub const TK_IDENT: u16 = 1;
pub const TK_INT: u16 = 2;
pub const TK_FLOAT: u16 = 3;
pub const TK_CHAR: u16 = 4;
pub const TK_STRING: u16 = 5;
pub const TK_KW_UNDERSCORE: u16 = 6;
pub const TK_KW_ABSTRACT: u16 = 7;
pub const TK_KW_ALIGNOF: u16 = 8;
pub const TK_KW_AS: u16 = 9;
pub const TK_KW_ASM: u16 = 10;
pub const TK_KW_ASYNC: u16 = 11;
pub const TK_KW_AWAIT: u16 = 12;
pub const TK_KW_BASE: u16 = 13;
pub const TK_KW_BREAK: u16 = 14;
pub const TK_KW_CATCH: u16 = 15;
pub const TK_KW_CLASS: u16 = 16;
pub const TK_KW_COMPTIME: u16 = 17;
pub const TK_KW_CONST: u16 = 18;
pub const TK_KW_CONTINUE: u16 = 19;
pub const TK_KW_DO: u16 = 20;
pub const TK_KW_ELSE: u16 = 21;
pub const TK_KW_ENUM: u16 = 22;
pub const TK_KW_EXTERN: u16 = 23;
pub const TK_KW_FALSE: u16 = 24;
pub const TK_KW_FOR: u16 = 25;
pub const TK_KW_FOREACH: u16 = 26;
pub const TK_KW_GOTO: u16 = 27;
pub const TK_KW_IF: u16 = 28;
pub const TK_KW_IMPL: u16 = 29;
pub const TK_KW_IN: u16 = 30;
pub const TK_KW_INTERFACE: u16 = 31;
pub const TK_KW_INTERNAL: u16 = 32;
pub const TK_KW_MACRO: u16 = 33;
pub const TK_KW_MATCH: u16 = 34;
pub const TK_KW_NAMESPACE: u16 = 35;
pub const TK_KW_NEW: u16 = 36;
pub const TK_KW_NULL: u16 = 37;
pub const TK_KW_OUT: u16 = 38;
pub const TK_KW_OVERRIDE: u16 = 39;
pub const TK_KW_PRIVATE: u16 = 40;
pub const TK_KW_PROTECTED: u16 = 41;
pub const TK_KW_PUBLIC: u16 = 42;
pub const TK_KW_REF: u16 = 43;
pub const TK_KW_RETURN: u16 = 44;
pub const TK_KW_SEALED: u16 = 45;
pub const TK_KW_SIZEOF: u16 = 46;
pub const TK_KW_STATIC: u16 = 47;
pub const TK_KW_STRUCT: u16 = 48;
pub const TK_KW_THIS: u16 = 49;
pub const TK_KW_THROW: u16 = 50;
pub const TK_KW_TRUE: u16 = 51;
pub const TK_KW_TRY: u16 = 52;
pub const TK_KW_UNSAFE: u16 = 53;
pub const TK_KW_USE: u16 = 54;
pub const TK_KW_USING: u16 = 55;
pub const TK_KW_VAR: u16 = 56;
pub const TK_KW_VIRTUAL: u16 = 57;
pub const TK_KW_WEAK: u16 = 58;
pub const TK_KW_WHERE: u16 = 59;
pub const TK_KW_WHILE: u16 = 60;
pub const TK_KW_YIELD: u16 = 61;
pub const TK_SHL_EQ: u16 = 62;
pub const TK_SHR_EQ: u16 = 63;
pub const TK_DOT_DOT_EQ: u16 = 64;
pub const TK_QQ_EQ: u16 = 65;
pub const TK_WRAP_ADD_EQ: u16 = 66;
pub const TK_WRAP_SUB_EQ: u16 = 67;
pub const TK_WRAP_MUL_EQ: u16 = 68;
pub const TK_EQ_EQ: u16 = 69;
pub const TK_NOT_EQ: u16 = 70;
pub const TK_LT_EQ: u16 = 71;
pub const TK_GT_EQ: u16 = 72;
pub const TK_AND_AND: u16 = 73;
pub const TK_OR_OR: u16 = 74;
pub const TK_SHL: u16 = 75;
pub const TK_SHR: u16 = 76;
pub const TK_PLUS_EQ: u16 = 77;
pub const TK_MINUS_EQ: u16 = 78;
pub const TK_STAR_EQ: u16 = 79;
pub const TK_SLASH_EQ: u16 = 80;
pub const TK_PERCENT_EQ: u16 = 81;
pub const TK_AMP_EQ: u16 = 82;
pub const TK_PIPE_EQ: u16 = 83;
pub const TK_CARET_EQ: u16 = 84;
pub const TK_FAT_ARROW: u16 = 85;
pub const TK_ARROW: u16 = 86;
pub const TK_COLON_COLON: u16 = 87;
pub const TK_DOT_DOT: u16 = 88;
pub const TK_QQ: u16 = 89;
pub const TK_PLUS_PLUS: u16 = 90;
pub const TK_MINUS_MINUS: u16 = 91;
pub const TK_WRAP_ADD: u16 = 92;
pub const TK_WRAP_SUB: u16 = 93;
pub const TK_WRAP_MUL: u16 = 94;
pub const TK_PLUS: u16 = 95;
pub const TK_MINUS: u16 = 96;
pub const TK_STAR: u16 = 97;
pub const TK_SLASH: u16 = 98;
pub const TK_PERCENT: u16 = 99;
pub const TK_EQ: u16 = 100;
pub const TK_LT: u16 = 101;
pub const TK_GT: u16 = 102;
pub const TK_BANG: u16 = 103;
pub const TK_TILDE: u16 = 104;
pub const TK_AMP: u16 = 105;
pub const TK_PIPE: u16 = 106;
pub const TK_CARET: u16 = 107;
pub const TK_QUESTION: u16 = 108;
pub const TK_COLON: u16 = 109;
pub const TK_SEMI: u16 = 110;
pub const TK_COMMA: u16 = 111;
pub const TK_DOT: u16 = 112;
pub const TK_LPAREN: u16 = 113;
pub const TK_RPAREN: u16 = 114;
pub const TK_LBRACKET: u16 = 115;
pub const TK_RBRACKET: u16 = 116;
pub const TK_LBRACE: u16 = 117;
pub const TK_RBRACE: u16 = 118;
pub const TK_KW_FIRST: u16 = 6;
pub const TK_KW_LAST: u16 = 61;
pub const TK_PUNCT_FIRST: u16 = 62;
pub const TK_PUNCT_LAST: u16 = 118;
pub const TK_COUNT: u16 = 119;

/// The spelling of each kind (for literals and EOF, the class name `lex` prints).
pub fn spelling(kind: u16) -> &'static str {
    match kind {
        TK_EOF => "eof",
        TK_IDENT => "ident",
        TK_INT => "int",
        TK_FLOAT => "float",
        TK_CHAR => "char",
        TK_STRING => "string",
        TK_KW_UNDERSCORE => "_",
        TK_KW_ABSTRACT => "abstract",
        TK_KW_ALIGNOF => "alignof",
        TK_KW_AS => "as",
        TK_KW_ASM => "asm",
        TK_KW_ASYNC => "async",
        TK_KW_AWAIT => "await",
        TK_KW_BASE => "base",
        TK_KW_BREAK => "break",
        TK_KW_CATCH => "catch",
        TK_KW_CLASS => "class",
        TK_KW_COMPTIME => "comptime",
        TK_KW_CONST => "const",
        TK_KW_CONTINUE => "continue",
        TK_KW_DO => "do",
        TK_KW_ELSE => "else",
        TK_KW_ENUM => "enum",
        TK_KW_EXTERN => "extern",
        TK_KW_FALSE => "false",
        TK_KW_FOR => "for",
        TK_KW_FOREACH => "foreach",
        TK_KW_GOTO => "goto",
        TK_KW_IF => "if",
        TK_KW_IMPL => "impl",
        TK_KW_IN => "in",
        TK_KW_INTERFACE => "interface",
        TK_KW_INTERNAL => "internal",
        TK_KW_MACRO => "macro",
        TK_KW_MATCH => "match",
        TK_KW_NAMESPACE => "namespace",
        TK_KW_NEW => "new",
        TK_KW_NULL => "null",
        TK_KW_OUT => "out",
        TK_KW_OVERRIDE => "override",
        TK_KW_PRIVATE => "private",
        TK_KW_PROTECTED => "protected",
        TK_KW_PUBLIC => "public",
        TK_KW_REF => "ref",
        TK_KW_RETURN => "return",
        TK_KW_SEALED => "sealed",
        TK_KW_SIZEOF => "sizeof",
        TK_KW_STATIC => "static",
        TK_KW_STRUCT => "struct",
        TK_KW_THIS => "this",
        TK_KW_THROW => "throw",
        TK_KW_TRUE => "true",
        TK_KW_TRY => "try",
        TK_KW_UNSAFE => "unsafe",
        TK_KW_USE => "use",
        TK_KW_USING => "using",
        TK_KW_VAR => "var",
        TK_KW_VIRTUAL => "virtual",
        TK_KW_WEAK => "weak",
        TK_KW_WHERE => "where",
        TK_KW_WHILE => "while",
        TK_KW_YIELD => "yield",
        TK_SHL_EQ => "<<=",
        TK_SHR_EQ => ">>=",
        TK_DOT_DOT_EQ => "..=",
        TK_QQ_EQ => "??=",
        TK_WRAP_ADD_EQ => "+%=",
        TK_WRAP_SUB_EQ => "-%=",
        TK_WRAP_MUL_EQ => "*%=",
        TK_EQ_EQ => "==",
        TK_NOT_EQ => "!=",
        TK_LT_EQ => "<=",
        TK_GT_EQ => ">=",
        TK_AND_AND => "&&",
        TK_OR_OR => "||",
        TK_SHL => "<<",
        TK_SHR => ">>",
        TK_PLUS_EQ => "+=",
        TK_MINUS_EQ => "-=",
        TK_STAR_EQ => "*=",
        TK_SLASH_EQ => "/=",
        TK_PERCENT_EQ => "%=",
        TK_AMP_EQ => "&=",
        TK_PIPE_EQ => "|=",
        TK_CARET_EQ => "^=",
        TK_FAT_ARROW => "=>",
        TK_ARROW => "->",
        TK_COLON_COLON => "::",
        TK_DOT_DOT => "..",
        TK_QQ => "??",
        TK_PLUS_PLUS => "++",
        TK_MINUS_MINUS => "--",
        TK_WRAP_ADD => "+%",
        TK_WRAP_SUB => "-%",
        TK_WRAP_MUL => "*%",
        TK_PLUS => "+",
        TK_MINUS => "-",
        TK_STAR => "*",
        TK_SLASH => "/",
        TK_PERCENT => "%",
        TK_EQ => "=",
        TK_LT => "<",
        TK_GT => ">",
        TK_BANG => "!",
        TK_TILDE => "~",
        TK_AMP => "&",
        TK_PIPE => "|",
        TK_CARET => "^",
        TK_QUESTION => "?",
        TK_COLON => ":",
        TK_SEMI => ";",
        TK_COMMA => ",",
        TK_DOT => ".",
        TK_LPAREN => "(",
        TK_RPAREN => ")",
        TK_LBRACKET => "[",
        TK_RBRACKET => "]",
        TK_LBRACE => "{",
        TK_RBRACE => "}",
        _ => "?",
    }
}
