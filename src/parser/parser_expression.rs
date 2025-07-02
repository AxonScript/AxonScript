//parsing expressons(types, binary op... )


use crate::ast::{Expr, Operator};
use crate::lexer_tokenizer::Token;
use crate::parser::{
    parser_error::{ErrorKind, ParseError, ParseResult, Severity},
    parser_kernel::Parser,
};

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_binary_op(0)
    }

    fn parse_binary_op(&mut self, min_precedence: i32) -> ParseResult<Expr> {
        let left_res = self.parse_term();
        if let Some(left) = left_res.result {
            let mut result = left;
            let mut errors = left_res.errors;

            while let Some(op) = self.current().and_then(|t| self.token_to_operator(t)) {
                let (prec, assoc) = self.operator_precedence(op);
                if prec < min_precedence {
                    break;
                }
                self.advance();
                let next_min_prec = if assoc == Assoc::Left { prec + 1 } else { prec };
                let right_res = self.parse_binary_op(next_min_prec);
                if let Some(right) = right_res.result {
                    result = Expr::BinaryOp {
                        left: Box::new(result),
                        op,
                        right: Box::new(right),
                    };
                    errors.extend(right_res.errors);
                } else {
                    errors.extend(right_res.errors);
                    return ParseResult {
                        result: None,
                        errors,
                    };
                }
            }
            ParseResult {
                result: Some(result),
                errors,
            }
        } else {
            ParseResult {
                result: None,
                errors: left_res.errors,
            }
        }
    }

    pub fn parse_term(&mut self) -> ParseResult<Expr> {
        let current = self.current().cloned();
        let span = self.tokens.get(self.pos).map(|t| t.span.clone());
        match current {
            Some(Token::Number(n)) => {
                self.advance();
                ParseResult::ok(Expr::Int32(n as i32))
            }
            Some(Token::Float(f)) => {
                self.advance();
                ParseResult::ok(Expr::Float32(f as f32))
            }
            Some(Token::StringLiteral(s)) => {
                self.advance();
                ParseResult::ok(Expr::String(s))
            }
            Some(Token::True) => {
                self.advance();
                ParseResult::ok(Expr::Bool(true))
            }
            Some(Token::False) => {
                self.advance();
                ParseResult::ok(Expr::Bool(false))
            }
            Some(Token::Identifier(id)) => {
                self.advance();
                ParseResult::ok(Expr::Identifier(id))
            }
            Some(Token::LBracket) => self.parse_vector(),
            Some(Token::LParen) => {
                let mut errors = Vec::new();
                self.advance();
                let expr_res = self.parse_expr();
                errors.extend(expr_res.errors);
                if let Err(err) = self.expect(&Token::RParen) {
                    errors.push(err);
                    ParseResult { result: None, errors }
                } else {
                    ParseResult {
                        result: expr_res.result,
                        errors,
                    }
                }
            }
            Some(token) => ParseResult::err(ParseError::new(
                ErrorKind::Syntax,
                format!(
                    "\x1b[31m[ERR-SYN-004] Unexpected token '{:?}' in expression at position {}.\x1b[0m",
                    token, self.pos
                ),
                span.as_ref().map_or(self.pos, |s| s.start),
                span.as_ref().map_or(self.pos, |s| s.end),
                self.src.clone(),
                Some("Expected a number, string, boolean, identifier, vector, or parenthesized expression.".to_string()),
                Severity::Error,
            )),
            None => ParseResult::err(ParseError::new(
                ErrorKind::Syntax,
                format!(
                    "\x1b[31m[ERR-SYN-005] Unexpected end of input at position {}. Expected an expression.\x1b[0m",
                    self.pos
                ),
                self.pos,
                self.pos,
                self.src.clone(),
                None,
                Severity::Error,
            )),
        }
    }

    fn parse_vector(&mut self) -> ParseResult<Expr> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::LBracket) {
            return ParseResult::err(err);
        }
        let mut elements = Vec::new();
        if self.current() != Some(&Token::RBracket) {
            loop {
                let elem_res = self.parse_expr();
                if let Some(elem) = elem_res.result {
                    elements.push(elem);
                }
                errors.extend(elem_res.errors);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        if let Err(err) = self.expect(&Token::RBracket) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        if elements.is_empty() {
            errors.push(ParseError::new(
                ErrorKind::Semantic,
                "\x1b[33m[WARN-SEM-006] Empty vector literal detected.\x1b[0m".to_string(),
                self.pos,
                self.pos,
                self.src.clone(),
                Some("Consider adding elements to the vector.".to_string()),
                Severity::Warning,
            ));
        }
        ParseResult {
            result: Some(Expr::Vector(elements)),
            errors,
        }
    }

    fn token_to_operator(&self, token: &Token) -> Option<Operator> {
        match token {
            Token::Plus => Some(Operator::Plus),
            Token::Minus => Some(Operator::Minus),
            Token::Star => Some(Operator::Multiply),
            Token::Slash => Some(Operator::Divide),
            _ => None,
        }
    }

    fn operator_precedence(&self, op: Operator) -> (i32, Assoc) {
        match op {
            Operator::Plus | Operator::Minus => (1, Assoc::Left),
            Operator::Multiply | Operator::Divide => (2, Assoc::Left),
        }
    }
}

#[derive(PartialEq)]
enum Assoc {
    Left,
    Right,
}
