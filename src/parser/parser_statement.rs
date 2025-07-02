//parsing all tokens in the code and converting them to AST



use crate::ast::*;
use crate::lexer_tokenizer::Token;
use crate::parser::{
    parser_error::{ErrorKind, ParseError, ParseResult, Severity},
    parser_kernel::Parser,
};

impl<'a> Parser<'a> {
    pub fn parse_statement(&mut self) -> ParseResult<Statement> {
        if self.match_token(&Token::Do) {
            let stmt_res = self.parse_statement();
            return match stmt_res.result {
                Some(stmt) => ParseResult {
                    result: Some(Statement::Do(Box::new(stmt))),
                    errors: stmt_res.errors,
                },
                None => ParseResult {
                    result: None,
                    errors: stmt_res.errors,
                },
            };
        }
        let current = self.current();
        match current {
            Some(Token::Print) => self.parse_print(),
            Some(Token::Function) => self.parse_function(),
            Some(Token::Set) => self.parse_variable(),
            Some(Token::Math) => self.parse_math(),
            Some(Token::If) => self.parse_if(),
            Some(Token::Identifier(id)) => {
                let span = self.tokens.get(self.pos).map(|t| t.span.clone());
                ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!("\x1b[31m[ERR-SYN-001] Invalid statement: '{}'.\x1b[0m", id),
                    span.as_ref().map_or(0, |s| s.start),
                    span.as_ref().map_or(0, |s| s.end),
                    self.src.clone(),
                    Some(format!("Did you mean 'set'?")),
                    Severity::Error,
                ))
            }
            Some(Token::Loop) => self.parse_loop(),
            Some(Token::While) => self.parse_while(),
            Some(Token::Break) => self.parse_break(),
            Some(Token::Input) => self.parse_input(),
            Some(token) => {
                let span = self.tokens.get(self.pos).map(|t| t.span.clone());
                ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!(
                        "\x1b[31m[ERR-SYN-002] Unexpected token: '{:?}'. Expected a valid statement.\x1b[0m",
                        token
                    ),
                    span.as_ref().map_or(0, |s| s.start),
                    span.as_ref().map_or(0, |s| s.end),
                    self.src.clone(),
                    None,
                    Severity::Error,
                ))
            }
            None => ParseResult::err(ParseError::new(
                ErrorKind::Syntax,
                format!(
                    "\x1b[31m[ERR-SYN-003] Unexpected end of input at position {}. Expected a statement.\x1b[0m",
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
    fn parse_break(&mut self) -> ParseResult<Statement> {
        let errors = Vec::new();
        if let Err(err) = self.expect(&Token::Break) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::EndStr) {
            return ParseResult::err(err);
        }
        return ParseResult {
            result: Some(Statement::Break {}),
            errors,
        };
    }
    pub fn parse_input(&mut self) -> ParseResult<Statement> {
        let errors = Vec::new();
        if let Err(err) = self.expect(&Token::Input) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::LParen) {
            return ParseResult::err(err);
        }
        let token = self.current().cloned();
        let target = match token {
            Some(Token::Identifier(id)) => {
                self.advance();
                Expr::Identifier(id.clone())
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!(
                        "\x1b[ERR - SYN - 109] Expected Token inside 'in(...)' at position { }.Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };
        if let Err(err) = self.expect(&Token::RParen) {
            return ParseResult::err(err);
        }

        let err_msg = if self.match_token(&Token::Dot) {
            match self.current() {
                Some(Token::Identifier(id)) if id == "Err" => {
                    self.advance();
                    if let Err(err) = self.expect(&Token::LParen) {
                        return ParseResult::err(err);
                    }
                    let msg = match self.current() {
                        Some(Token::StringLiteral(s)) => {
                            let m = s.clone();
                            self.advance();
                            m
                        }
                        _ => {
                            return ParseResult::err(ParseError::new(
                                ErrorKind::Semantic,
                                format!(
                                    "\x1b[31m[ERR-SEM-906] Expected string literal after Err at position {}.\x1b[0m",
                                    self.pos
                                ),
                                self.pos,
                                self.pos,
                                self.src.clone(),
                                None,
                                Severity::Error,
                            ));
                        }
                    };
                    if let Err(err) = self.expect(&Token::RParen) {
                        return ParseResult::err(err);
                    }
                    Some(msg)
                }
                _ => {
                    return ParseResult::err(ParseError::new(
                        ErrorKind::Semantic,
                        format!(
                            "\x1b[31m[ERR-SEM-907] Expected 'Err' after '.' at position {}.\x1b[0m",
                            self.pos
                        ),
                        self.pos,
                        self.pos,
                        self.src.clone(),
                        None,
                        Severity::Error,
                    ));
                }
            }
        } else {
            None
        };

        if let Err(err) = self.expect(&Token::EndStr) {
            return ParseResult::err(err);
        }
        return ParseResult {
            result: Some(Statement::Input { target, err: err_msg }),
            errors,
        };
    }
    pub fn parse_while(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::While) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::LParen) {
            return ParseResult::err(err);
        }
        let token = self.current().cloned();
        let left = match token {
            Some(Token::Identifier(id)) => {
                self.advance();
                Expr::Identifier(id.clone())
            }
            Some(Token::Number(val)) => {
                self.advance();
                Expr::Int64(val)
            }
            Some(Token::Float(val)) => {
                self.advance();
                Expr::Float64(val as f64)
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!(
                        "\x1b[31m[ERR-SYN-105] Expected identifier or number in while condition at position {}. Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };

        let logic = match self.current() {
            Some(Token::Equal) => {
                self.advance();
                Logic::Equal
            }
            Some(Token::NotEqual) => {
                self.advance();
                Logic::NotEqual
            }
            Some(Token::Greater) => {
                self.advance();
                Logic::Greater
            }
            Some(Token::Less) => {
                self.advance();
                Logic::Less
            }
            Some(Token::GreaterEqual) => {
                self.advance();
                Logic::GreaterEqual
            }
            Some(Token::LessEqual) => {
                self.advance();
                Logic::LessEqual
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Semantic,
                    format!(
                        "\x1b[31m[ERR-SEM-111] Unknown comparison operator at position {}. Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    Some("Expected one of: <, >, ==, !=, <=, >=.".to_string()),
                    Severity::Error,
                ));
            }
        };
        let token = self.current().cloned();
        let right = match token {
            Some(Token::Identifier(id)) => {
                self.advance();
                Expr::Identifier(id.clone())
            }
            Some(Token::Number(val)) => {
                self.advance();
                Expr::Int64(val)
            }
            Some(Token::Float(val)) => {
                self.advance();
                Expr::Float64(val as f64)
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!(
                        "\x1b[31m[ERR-SYN-106] Expected identifier or number in while condition at position {}. Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };
        if let Err(err) = self.expect(&Token::RParen) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::DoubleGt) {
            return ParseResult::err(err);
        }
        let mut statements = Vec::new();

        while self.current() != Some(&Token::DoubleLt) && self.current().is_some() {
            let start_pos = self.pos;
            let stmt_res = self.parse_statement();
            if let Some(stmt) = stmt_res.result {
                statements.push(stmt);
            }
            errors.extend(stmt_res.errors);

            if self.current() == Some(&Token::EndStr) {
                self.advance();
            }

            if self.pos == start_pos {
                self.advance();
            }
        }
        if let Err(err) = self.expect(&Token::DoubleLt) {
            errors.push(err);
        }
        return ParseResult {
            result: Some(Statement::While {
                logic,
                args: vec![left, right],
                body: statements,
            }),
            errors,
        };
    }
    pub fn parse_loop(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::Loop) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::DoubleGt) {
            return ParseResult::err(err);
        }
        let mut statements = Vec::new();

        while self.current() != Some(&Token::DoubleLt) && self.current().is_some() {
            let start_pos = self.pos;
            let stmt_res = self.parse_statement();
            if let Some(stmt) = stmt_res.result {
                statements.push(stmt);
            }
            errors.extend(stmt_res.errors);

            if self.current() == Some(&Token::EndStr) {
                self.advance();
            }

            if self.pos == start_pos {
                self.advance();
            }
        }
        if let Err(err) = self.expect(&Token::DoubleLt) {
            errors.push(err);
        }
        return ParseResult {
            result: Some(Statement::Loop { body: statements }),
            errors,
        };
    }
    pub fn parse_if(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::If) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::LParen) {
            return ParseResult::err(err);
        }
        let token = self.current().cloned();
        let left = match token {
            Some(Token::Identifier(id)) => {
                self.advance();
                Expr::Identifier(id.clone())
            }
            Some(Token::Number(val)) => {
                self.advance();
                Expr::Int64(val)
            }
            Some(Token::Float(val)) => {
                self.advance();
                Expr::Float64(val as f64)
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!(
                        "\x1b[31m[ERR-SYN-107] Expected identifier or number in if condition at position {}. Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };

        let logic = match self.current() {
            Some(Token::Equal) => {
                self.advance();
                Logic::Equal
            }
            Some(Token::NotEqual) => {
                self.advance();
                Logic::NotEqual
            }
            Some(Token::Greater) => {
                self.advance();
                Logic::Greater
            }
            Some(Token::Less) => {
                self.advance();
                Logic::Less
            }
            Some(Token::GreaterEqual) => {
                self.advance();
                Logic::GreaterEqual
            }
            Some(Token::LessEqual) => {
                self.advance();
                Logic::LessEqual
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Semantic,
                    format!(
                        "\x1b[31m[ERR-SEM-111] Unknown comparison operator at position {}. Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    Some("Expected one of: <, >, ==, !=, <=, >=.".to_string()),
                    Severity::Error,
                ));
            }
        };
        let token = self.current().cloned();
        let right = match token {
            Some(Token::Identifier(id)) => {
                self.advance();
                Expr::Identifier(id.clone())
            }
            Some(Token::Number(val)) => {
                self.advance();
                Expr::Int64(val)
            }
            Some(Token::Float(val)) => {
                self.advance();
                Expr::Float64(val as f64)
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Syntax,
                    format!(
                        "\x1b[31m[ERR-SYN-108] Expected identifier or number in if condition at position {}. Found: {:?}.\x1b[0m",
                        self.pos,
                        self.current()
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };
        if let Err(err) = self.expect(&Token::RParen) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::DoubleGt) {
            return ParseResult::err(err);
        }
        let mut statements = Vec::new();

        while self.current() != Some(&Token::DoubleLt) && self.current().is_some() {
            let start_pos = self.pos;
            let stmt_res = self.parse_statement();
            if let Some(stmt) = stmt_res.result {
                statements.push(stmt);
            }
            errors.extend(stmt_res.errors);

            if self.current() == Some(&Token::EndStr) {
                self.advance();
            }

            if self.pos == start_pos {
                self.advance();
            }
        }
        if let Err(err) = self.expect(&Token::DoubleLt) {
            errors.push(err);
        }
        let mut else_body = None;
        if self.current() == Some(&Token::Else) {
            self.advance();
            if let Err(err) = self.expect(&Token::DoubleGt) {
                errors.push(err);
                return ParseResult {
                    result: None,
                    errors,
                };
            }
            let mut else_statements = Vec::new();
            while self.current() != Some(&Token::DoubleLt) && self.current().is_some() {
                let start_pos = self.pos;
                let stmt_res = self.parse_statement();
                if let Some(stmt) = stmt_res.result {
                    else_statements.push(stmt);
                }
                errors.extend(stmt_res.errors);

                if self.current() == Some(&Token::EndStr) {
                    self.advance();
                }

                if self.pos == start_pos {
                    self.advance();
                }
            }
            if let Err(err) = self.expect(&Token::DoubleLt) {
                errors.push(err);
            }
            else_body = Some(else_statements);
        }
        return ParseResult {
            result: Some(Statement::If {
                logic,
                args: vec![left, right],
                body: statements,
                else_body,
            }),
            errors,
        };
    }

    pub fn parse_function(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::Function) {
            return ParseResult::err(err);
        }
        let name = match self.current() {
            Some(Token::Identifier(id)) => {
                let name = id.clone();
                self.advance();
                name
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Semantic,
                    format!(
                        "\x1b[31m[ERR-SEM-101] Expected function name at position {}.\x1b[0m",
                        self.pos
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };
        let start = name == "Start";
        if let Err(err) = self.expect(&Token::LParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let mut params = Vec::new();
        if self.current() != Some(&Token::RParen) {
            loop {
                let param_res = self.parse_parameter_name();
                if let Some(param_name) = param_res.result {
                    params.push(Expr::Identifier(param_name));
                }
                errors.extend(param_res.errors);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        if let Err(err) = self.expect(&Token::RParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let body_res = self.parse_block();
        errors.extend(body_res.errors);
        let result = body_res.result.map(|body| Statement::FunctionCall {
            name,
            start,
            params,
            body,
        });
        ParseResult { result, errors }
    }

    pub fn parse_print(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::Print) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::LParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let mut params = Vec::new();
        if self.current() != Some(&Token::RParen) {
            loop {
                let expr_res = self.parse_expr();
                if let Some(expr) = expr_res.result {
                    params.push(expr);
                }
                errors.extend(expr_res.errors);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        } else {
            errors.push(ParseError::new(
                ErrorKind::Semantic,
                "\x1b[33m[WARN-SEM-002] Empty print statement.\x1b[0m".to_string(),
                self.pos,
                self.pos,
                self.src.clone(),
                Some("Consider adding expressions to print.".to_string()),
                Severity::Warning,
            ));
        }
        if let Err(err) = self.expect(&Token::RParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        if let Err(err) = self.expect(&Token::EndStr) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        ParseResult {
            result: Some(Statement::Print { params }),
            errors,
        }
    }

    pub fn parse_math(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::Math) {
            return ParseResult::err(err);
        }
        if let Err(err) = self.expect(&Token::LParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        if let Err(err) = self.expect(&Token::LBracket) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let expr_res = self.parse_expr();
        errors.extend(expr_res.errors);
        let expression = expr_res.result;
        if let Err(err) = self.expect(&Token::RBracket) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let destination = if self.match_token(&Token::Comma) {
            match self.current() {
                Some(Token::Identifier(id)) => {
                    let name = id.clone();
                    self.advance();
                    name
                }
                _ => {
                    errors.push(ParseError::new(
                        ErrorKind::Semantic,
                        format!(
                            "\x1b[31m[ERR-SEM-102] Expected identifier for math result after comma at position {}.\x1b[0m",
                            self.pos
                        ),
                        self.pos,
                        self.pos,
                        self.src.clone(),
                        None,
                        Severity::Error,
                    ));
                    "Result".to_string()
                }
            }
        } else {
            errors.push(ParseError::new(
                ErrorKind::Semantic,
                "\x1b[33m[WARN-SEM-003] No destination specified for math expression.\x1b[0m"
                    .to_string(),
                self.pos,
                self.pos,
                self.src.clone(),
                Some("Using default 'Result'.".to_string()),
                Severity::Warning,
            ));
            "Result".to_string()
        };
        if let Err(err) = self.expect(&Token::RParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }

        let err_msg = if self.match_token(&Token::Dot) {
            match self.current() {
                Some(Token::Identifier(id)) if id == "Err" => {
                    self.advance();
                    if let Err(err) = self.expect(&Token::LParen) {
                        errors.push(err);
                        None
                    } else {
                        match self.current() {
                            Some(Token::StringLiteral(s)) => {
                                let msg = s.clone();
                                self.advance();
                                if let Err(err) = self.expect(&Token::RParen) {
                                    errors.push(err);
                                }
                                Some(msg)
                            }
                            _ => {
                                errors.push(ParseError::new(
                                    ErrorKind::Semantic,
                                    format!(
                                        "\x1b[31m[ERR-SEM-905] Expected string literal after Err at position {}.\x1b[0m",
                                        self.pos
                                    ),
                                    self.pos,
                                    self.pos,
                                    self.src.clone(),
                                    None,
                                    Severity::Error,
                                ));
                                None
                            }
                        }
                    }
                }
                _ => {
                    errors.push(ParseError::new(
                        ErrorKind::Semantic,
                        format!(
                            "\x1b[31m[ERR-SEM-904] Expected 'Err' after '.' at position {}.\x1b[0m",
                            self.pos
                        ),
                        self.pos,
                        self.pos,
                        self.src.clone(),
                        None,
                        Severity::Error,
                    ));
                    None
                }
            }
        } else {
            None
        };

        if let Err(err) = self.expect(&Token::EndStr) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        ParseResult {
            result: expression.map(|expr| Statement::Math {
                expression: expr,
                destination,
                err: err_msg,
            }),
            errors,
        }
    }

    pub fn parse_variable(&mut self) -> ParseResult<Statement> {
        let mut errors = Vec::new();
        if let Err(err) = self.expect(&Token::Set) {
            return ParseResult::err(err);
        }
        let mutable = if self.current() == Some(&Token::Colon) {
            self.advance();
            true
        } else {
            false
        };
        let name = match self.current() {
            Some(Token::Identifier(id)) => {
                let name = id.clone();
                self.advance();
                name
            }
            _ => {
                errors.push(ParseError::new(
                    ErrorKind::Semantic,
                    format!(
                        "\x1b[31m[ERR-SEM-103] Expected variable name at position {}.\x1b[0m",
                        self.pos
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
                return ParseResult {
                    result: None,
                    errors,
                };
            }
        };
        if let Err(err) = self.expect(&Token::LParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let type_var = match self.current() {
            Some(Token::I32) => {
                self.advance();
                Some(Type::I32)
            }
            Some(Token::I64) => {
                self.advance();
                Some(Type::I64)
            }
            Some(Token::F32) => {
                self.advance();
                Some(Type::F32)
            }
            Some(Token::F64) => {
                self.advance();
                Some(Type::F64)
            }
            Some(Token::TypeString) => {
                self.advance();
                Some(Type::String)
            }
            Some(Token::Vector) => {
                self.advance();
                let inner_res = self.parse_vector_type();
                errors.extend(inner_res.errors);
                inner_res.result.map(|inner| Type::Vector(Box::new(inner)))
            }
            Some(Token::Bool) => {
                self.advance();
                Some(Type::Bool)
            }
            _ => {
                errors.push(ParseError::new(
                    ErrorKind::Type,
                    format!(
                        "\x1b[31m[ERR-TYP-001] Expected type (i32, i64, f32, f64, string, vector, bool) at position {}.\x1b[0m",
                        self.pos
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
                None
            }
        };
        if let Err(err) = self.expect(&Token::RParen) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        let value = if self.match_token(&Token::Assign) {
            let expr_res = self.parse_expr();
            errors.extend(expr_res.errors);
            expr_res.result
        } else {
            errors.push(ParseError::new(
                ErrorKind::Semantic,
                format!(
                    "\x1b[31m[ERR-SEM-105] Variable '{}' requires an initialization value at position {}.\x1b[0m",
                    name, self.pos
                ),
                self.pos,
                self.pos,
                self.src.clone(),
                Some("Add an expression after '=' to initialize the variable.".to_string()),
                Severity::Error,
            ));
            return ParseResult {
                result: None,
                errors,
            };
        };
        if let Err(err) = self.expect(&Token::EndStr) {
            errors.push(err);
            return ParseResult {
                result: None,
                errors,
            };
        }
        ParseResult {
            result: value.map(|value| Statement::Assignment {
                name,
                mutable,
                type_var,
                value,
            }),
            errors,
        }
    }

    fn parse_parameter_name(&mut self) -> ParseResult<String> {
        match self.current() {
            Some(Token::Identifier(id)) => {
                let name = id.clone();
                self.advance();
                ParseResult::ok(name)
            }
            _ => ParseResult::err(ParseError::new(
                ErrorKind::Semantic,
                format!(
                    "\x1b[31m[ERR-SEM-104] Expected parameter name at position {}.\x1b[0m",
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
    fn parse_block(&mut self) -> ParseResult<Vec<Statement>> {
        let mut statements = Vec::new();
        let mut errors = Vec::new();

        if !self.match_token(&Token::DoubleGt) {
            return ParseResult::err(ParseError::new(
                ErrorKind::Syntax,
                "Expected '>>' to start block".to_string(),
                self.pos,
                self.pos,
                self.src.clone(),
                None,
                Severity::Error,
            ));
        }

        while let Some(token) = self.current() {
            if token == &Token::DoubleLt {
                self.advance();
                break;
            }
            let start_pos = self.pos;
            let stmt_res = self.parse_statement();

            if let Some(stmt) = stmt_res.result {
                statements.push(stmt);
            }
            errors.extend(stmt_res.errors);

            if self.current() == Some(&Token::EndStr) {
                self.advance();
            }

            if self.pos == start_pos {
                self.advance();
            }
        }

        ParseResult {
            result: Some(statements),
            errors,
        }
    }

    fn parse_vector_type(&mut self) -> ParseResult<Type> {
        if let Err(err) = self.expect(&Token::LParen) {
            return ParseResult::err(err);
        }
        let inner_type = match self.current() {
            Some(Token::I32) => {
                self.advance();
                Type::I32
            }
            Some(Token::I64) => {
                self.advance();
                Type::I64
            }
            Some(Token::F32) => {
                self.advance();
                Type::F32
            }
            Some(Token::F64) => {
                self.advance();
                Type::F64
            }
            Some(Token::TypeString) => {
                self.advance();
                Type::String
            }
            Some(Token::Bool) => {
                self.advance();
                Type::Bool
            }
            _ => {
                return ParseResult::err(ParseError::new(
                    ErrorKind::Type,
                    format!(
                        "\x1b[31m[ERR-TYP-002] Expected inner type for vector at position {}.\x1b[0m",
                        self.pos
                    ),
                    self.pos,
                    self.pos,
                    self.src.clone(),
                    None,
                    Severity::Error,
                ));
            }
        };
        if let Err(err) = self.expect(&Token::RParen) {
            return ParseResult::err(err);
        }
        ParseResult::ok(Type::Vector(Box::new(inner_type)))
    }
}
