//the same logic as in semantic_error.rs, 
//we collect the error vector and output them in main.rs

use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorKind {
    Syntax,
    Semantic,
    Type,
    Codegen,
    Linker,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ErrorKind,
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub src: Option<String>,
    pub suggestion: Option<String>,
    pub severity: Severity,
}

impl ParseError {
    pub fn new(
        kind: ErrorKind,
        message: String,
        start: usize,
        end: usize,
        src: Option<String>,
        suggestion: Option<String>,
        severity: Severity,
    ) -> Self {
        ParseError {
            kind,
            message,
            start,
            end,
            src,
            suggestion,
            severity,
        }
    }

    pub fn eof(message: String) -> Self {
        ParseError {
            kind: ErrorKind::Syntax,
            message,
            start: 0,
            end: 0,
            src: None,
            suggestion: None,
            severity: Severity::Error,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        
        writeln!(f, "{}", self.message)?;
        if let Some(src) = &self.src {
            let start_char = src[..self.start].chars().count();
            let end_char = src[..self.end].chars().count();
            let line_num = src[..self.start].chars().filter(|&c| c == '\n').count() + 1;
            let line_start_byte = src[..self.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end_byte = src[self.start..]
                .find('\n')
                .map(|i| self.start + i)
                .unwrap_or(src.len());
            let line_start_char = src[..line_start_byte].chars().count();
            let line_text = &src[line_start_byte..line_end_byte];
            writeln!(f, "{:>4} | {}", line_num, line_text)?;
            writeln!(
                f,
                "     | {}{}",
                " ".repeat(start_char - line_start_char),
                "\x1b[31m^\x1b[0m".repeat((end_char - start_char).max(1))
            )?;
        }
        if let Some(suggestion) = &self.suggestion {
            writeln!(f, "\x1b[36mSuggestion: {}\x1b[0m", suggestion)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ParseResult<T> {
    pub result: Option<T>,
    pub errors: Vec<ParseError>,
}

impl<T> ParseResult<T> {
    pub fn ok(result: T) -> Self {
        ParseResult {
            result: Some(result),
            errors: Vec::new(),
        }
    }

    pub fn err(error: ParseError) -> Self {
        ParseResult {
            result: None,
            errors: vec![error],
        }
    }

    pub fn with_result(result: Option<T>, errors: Vec<ParseError>) -> Self {
        ParseResult { result, errors }
    }
}
