//lexical code analysis, 
//it breaks down what you wrote into token
//that are then parsed into AST
//I used the logos library


use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // Types of var
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,
    #[token("str")]
    TypeString,
    #[token("bool")]
    Bool,
    #[token("Vec")]
    Vector,

    // Functions
    #[token("do")]
    Do,
    #[token("set")]
    Set,
    #[token("cast")]
    Function,
    #[token("if")]
    If,
    #[token("when")]
    When,
    #[token("not")]
    Not,
    #[token("else")]
    Else,
    #[token("yes")]
    True,
    #[token("no")]
    False,
    #[token("out")]
    Print,
    #[token("math")]
    Math,
    #[token("loop")]
    Loop,
    #[token("while")]
    While,
    #[token("break")]
    Break,
    #[token("in")]
    Input,

    // Punctuation
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(":")]
    Colon,
    #[token(";")]
    EndStr,
    #[token(".")]
    Dot,
    #[token("=")]
    Assign,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,

    // Arithmetic
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Mod,

    // Logic and comparisons
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    Greater,
    #[token("<")]
    Less,

    // End and beginning of all functions
    #[token(">>")]
    DoubleGt,
    #[token("<<")]
    DoubleLt,
    //Whitespace
    #[regex(r"\s+", logos::skip)]
    Whitespace,

    // Names
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_\-\*\$]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Float values
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse().unwrap_or(0.0))]
    Float(f64),

    // Integer values
    #[regex("[0-9]+", |lex| lex.slice().parse().unwrap_or(0))]
    Number(i64),
    // Strings
    #[regex(r#""([^"\\]|\\.)*""#, |lex| unescape(lex.slice()))]
    StringLiteral(String),

    // Comments
    #[regex(r"\?\?[^\r\n]*", logos::skip)]
    Comment,
    Error,
}

// Struct with token and its byte position in source
#[derive(Debug)]
pub struct PositionedToken {
    pub token: Token,
    pub span: std::ops::Range<usize>,
}

// Lexing function, returns tokens with spans
pub fn lex_with_span(source: &str) -> Vec<PositionedToken> {
    let mut lexer = Token::lexer(source);
    let mut tokens = Vec::new();

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        match result {
            Ok(token) => tokens.push(PositionedToken { token, span }),
            Err(_) => tokens.push(PositionedToken {
                token: Token::Error,
                span,
            }),
        }
    }

    tokens
}

// Converts a string with escapes like \n, \t, \" etc.
fn unescape(s: &str) -> String {
    let mut chars = s[1..s.len() - 1].chars(); // cut quotes
    let mut result = String::new();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => break,
            }
        } else {
            result.push(c);
        }
    }

    result
}
