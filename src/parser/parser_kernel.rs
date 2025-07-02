//all parser logic

use crate::ast::*;
use crate::lexer_tokenizer::{PositionedToken, Token};
use crate::parser::parser_error::{ErrorKind, ParseError, ParseResult, Severity};

pub struct Parser<'a> {
    pub tokens: &'a [PositionedToken],
    pub pos: usize,
    pub src: Option<String>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [PositionedToken], src: Option<String>) -> Self {
        Self {
            tokens,
            pos: 0,
            src,
        }
    }

    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    pub fn advance(&mut self) {
        self.pos += 1;
    }

    pub fn match_token(&mut self, expected: &Token) -> bool {
        if let Some(token) = self.current() {
            if token == expected {
                self.advance();
                return true;
            }
        }
        false
    }

    pub fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        if let Some(token) = self.tokens.get(self.pos) {
            if &token.token == expected {
                self.pos += 1;
                Ok(())
            } else {
                Err(ParseError::new(
                    ErrorKind::Syntax,
                    format!("expected token {:?}, found {:?}", expected, token.token),
                    token.span.start,
                    token.span.end,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ))
            }
        } else {
            Err(ParseError::eof(format!("expected token {:?}", expected)))
        }
    }

    pub fn parse_program(&mut self) -> ParseResult<Vec<Statement>> {
        let mut statements = Vec::new();
        let mut errors = Vec::new();

        while self.current().is_some() {
            if self.current() == Some(&Token::EndStr) {
                self.advance();
                continue;
            }
            let stmt_res = self.parse_statement();
            if let Some(stmt) = stmt_res.result.clone() {
                statements.push(stmt);
            }
            let had_error = !stmt_res.errors.is_empty();
            errors.extend(stmt_res.errors);


            if had_error {
                while self.current().is_some() && self.current() != Some(&Token::EndStr) {
                    self.advance();
                }
                if self.current() == Some(&Token::EndStr) {
                    self.advance();
                }

            }
        }
        ParseResult {
            result: Some(statements),
            errors,
        }
    }
}
