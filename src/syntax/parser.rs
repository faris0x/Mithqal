//! Recursive descent parser for the source language.

use crate::builtins;
use crate::diag::{Diagnostic, ErrorCode, Span};
use crate::expr::{Arena, ExprId};
use crate::num::{Int, NumError, Rat};

use super::lexer::{Token, TokenKind, lex};

/// Parse a complete source string into one expression per statement.
pub fn parse_source(arena: &mut Arena, source: &str) -> Result<Vec<ExprId>, Diagnostic> {
    let tokens = lex(source)?;
    let mut statements = Vec::new();
    let mut start = 0usize;

    for index in 0..=tokens.len() {
        let at_end = index == tokens.len();
        let at_newline = !at_end && tokens[index].kind == TokenKind::Newline;
        if at_end || at_newline {
            if start < index {
                let statement = &tokens[start..index];
                statements.push(parse_statement_tokens(arena, statement)?);
            }
            start = index + 1;
        }
    }

    Ok(statements)
}

/// Parse one statement. A trailing newline is allowed.
pub fn parse_statement(arena: &mut Arena, source: &str) -> Result<ExprId, Diagnostic> {
    if let Some(expression) = parse_two_integer_operation(arena, source) {
        return Ok(expression);
    }
    if let Some(expression) = parse_simple_expression(arena, source) {
        return Ok(expression);
    }
    let statements = parse_source(arena, source)?;
    match statements.len() {
        1 => Ok(statements[0]),
        0 => Err(Diagnostic::eval(
            ErrorCode::ParseUnexpectedToken,
            "expected a statement",
        )),
        _ => Err(Diagnostic::eval(
            ErrorCode::ParseUnexpectedToken,
            "expected one statement",
        )),
    }
}

fn parse_statement_tokens(arena: &mut Arena, tokens: &[Token]) -> Result<ExprId, Diagnostic> {
    let mut parser = Parser {
        arena,
        tokens,
        pos: 0,
    };
    let expression = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() {
        let token = &parser.tokens[parser.pos];
        let code = if token.kind == TokenKind::LBracket {
            ErrorCode::ParseIndexingUnsupported
        } else {
            ErrorCode::ParseUnexpectedToken
        };
        let message = if code == ErrorCode::ParseIndexingUnsupported {
            "indexing is not implemented; use a part call"
        } else {
            "unexpected token after the statement"
        };
        return Err(Diagnostic::parse(code, message, token.span));
    }
    Ok(expression)
}

struct Parser<'a> {
    arena: &'a mut Arena,
    tokens: &'a [Token],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|token| &token.kind)
    }

    fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .or_else(|| self.tokens.last())
            .map(|token| token.span)
            .unwrap_or(Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            })
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == Some(kind)
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, Diagnostic> {
        if self.peek() == Some(&kind) {
            Ok(self.advance().unwrap().clone())
        } else {
            Err(Diagnostic::parse(
                ErrorCode::ParseUnexpectedToken,
                message,
                self.span(),
            ))
        }
    }

    fn error(&self, code: ErrorCode, message: impl Into<String>) -> Diagnostic {
        Diagnostic::parse(code, message, self.span())
    }

    fn head(&mut self, name: &str) -> ExprId {
        let attrs = builtins::attributes(name).unwrap_or_default();
        let symbol = self.arena.intern_symbol(name, attrs);
        self.arena.symbol_expr(symbol)
    }

    fn symbol(&mut self, name: &str) -> ExprId {
        let symbol = if let Some(attrs) = builtins::attributes(name) {
            self.arena.intern_symbol(name, attrs)
        } else {
            self.arena.symbol_or_intern(name)
        };
        self.arena.symbol_expr(symbol)
    }

    fn call(&mut self, head: ExprId, args: &[ExprId]) -> ExprId {
        self.arena.intern_call(head, args)
    }

    fn call_name(&mut self, name: &str, args: &[ExprId]) -> ExprId {
        let head = self.head(name);
        self.call(head, args)
    }

    fn parse_expr(&mut self) -> Result<ExprId, Diagnostic> {
        self.parse_set()
    }

    fn parse_set(&mut self) -> Result<ExprId, Diagnostic> {
        let left = self.parse_rule()?;
        match self.peek() {
            Some(TokenKind::Equal) => {
                self.advance();
                let right = self.parse_set()?;
                Ok(self.call_name("set", &[left, right]))
            }
            Some(TokenKind::SetDelayed) => {
                self.advance();
                let right = self.parse_set()?;
                Ok(self.call_name("setDelayed", &[left, right]))
            }
            _ => Ok(left),
        }
    }

    fn parse_rule(&mut self) -> Result<ExprId, Diagnostic> {
        let left = self.parse_or()?;
        match self.peek() {
            Some(TokenKind::Arrow) => {
                self.advance();
                let right = self.parse_rule()?;
                Ok(self.call_name("rule", &[left, right]))
            }
            Some(TokenKind::RuleDelayed) => {
                self.advance();
                let right = self.parse_rule()?;
                Ok(self.call_name("ruleDelayed", &[left, right]))
            }
            _ => Ok(left),
        }
    }

    fn parse_or(&mut self) -> Result<ExprId, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.at(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = self.call_name("or", &[left, right]);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ExprId, Diagnostic> {
        let mut left = self.parse_comparison()?;
        while self.at(&TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = self.call_name("and", &[left, right]);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ExprId, Diagnostic> {
        let left = self.parse_additive()?;
        let name = match self.peek() {
            Some(TokenKind::Equal3) => "equal",
            Some(TokenKind::Equal2) => "equal",
            Some(TokenKind::NotEqual) => "notEqual",
            Some(TokenKind::Tilde) => "same",
            Some(TokenKind::Less) => "less",
            Some(TokenKind::LessEqual) => "lessEqual",
            Some(TokenKind::Greater) => "greater",
            Some(TokenKind::GreaterEqual) => "greaterEqual",
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_additive()?;
        let result = self.call_name(name, &[left, right]);
        if is_comparison(self.peek()) {
            return Err(self.error(
                ErrorCode::ParseUnexpectedToken,
                "comparison operators do not chain",
            ));
        }
        Ok(result)
    }

    fn parse_additive(&mut self) -> Result<ExprId, Diagnostic> {
        let mut left = self.parse_multiplicative()?;
        loop {
            match self.peek() {
                Some(TokenKind::Plus) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = self.call_name("plus", &[left, right]);
                }
                Some(TokenKind::Minus) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    let negative = self.negate_value(right)?;
                    left = self.call_name("plus", &[left, negative]);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<ExprId, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(TokenKind::Star) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = self.call_name("times", &[left, right]);
                }
                Some(TokenKind::Slash) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    let quotient = match (self.as_rat(left), self.as_rat(right)) {
                        (Some(numerator), Some(denominator)) if !denominator.is_zero() => {
                            numerator.div(&denominator).ok()
                        }
                        _ => None,
                    };
                    if let Some(value) = quotient {
                        left = self.arena.intern_rat(value);
                        continue;
                    }
                    let minus_one = self.integer(-1);
                    let inverse = self.call_name("power", &[right, minus_one]);
                    left = self.call_name("times", &[left, inverse]);
                }
                Some(kind) if starts_primary(kind) => {
                    let right = self.parse_unary()?;
                    left = self.call_name("times", &[left, right]);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<ExprId, Diagnostic> {
        match self.peek() {
            Some(TokenKind::Minus) => {
                self.advance();
                let value = self.parse_unary()?;
                self.negate_value(value)
            }
            Some(TokenKind::Not) => {
                self.advance();
                let value = self.parse_unary()?;
                Ok(self.call_name("not", &[value]))
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<ExprId, Diagnostic> {
        let base = self.parse_primary()?;
        if self.at(&TokenKind::Caret) {
            self.advance();
            let exponent = self.parse_unary()?;
            Ok(self.call_name("power", &[base, exponent]))
        } else {
            Ok(base)
        }
    }

    fn parse_primary(&mut self) -> Result<ExprId, Diagnostic> {
        let token = match self.advance() {
            Some(token) => token.clone(),
            None => {
                return Err(self.error(ErrorCode::ParseUnexpectedToken, "expected an expression"));
            }
        };
        match token.kind {
            TokenKind::Number(text) => self.number(&text, token.span),
            TokenKind::Ident(name) => {
                if name == "fn" && self.at(&TokenKind::LParen) {
                    return self.parse_function();
                }
                let base = self.symbol(&name);
                match self.peek() {
                    Some(TokenKind::Underscore) => self.pattern(base, "blank"),
                    Some(TokenKind::Underscore2) => self.pattern(base, "blankSequence"),
                    Some(TokenKind::Underscore3) => self.pattern(base, "blankNullSequence"),
                    Some(TokenKind::LParen) => {
                        let head = if name == "when" {
                            self.head("condition")
                        } else {
                            base
                        };
                        self.parse_call(head)
                    }
                    _ => Ok(base),
                }
            }
            TokenKind::Underscore => Ok(self.call_name("blank", &[])),
            TokenKind::Underscore2 => Ok(self.call_name("blankSequence", &[])),
            TokenKind::Underscore3 => Ok(self.call_name("blankNullSequence", &[])),
            TokenKind::LParen => {
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen, "expected a closing parenthesis")?;
                Ok(inner)
            }
            TokenKind::LBracket => self.parse_list(),
            _ => Err(self.error(
                ErrorCode::ParseUnexpectedToken,
                "expected a number, name, list, or parenthesized expression",
            )),
        }
    }

    fn pattern(&mut self, name: ExprId, blank: &str) -> Result<ExprId, Diagnostic> {
        self.advance();
        let blank = self.call_name(blank, &[]);
        Ok(self.call_name("pattern", &[name, blank]))
    }

    fn parse_call(&mut self, head: ExprId) -> Result<ExprId, Diagnostic> {
        self.expect(TokenKind::LParen, "expected an opening parenthesis")?;
        let mut args = Vec::new();
        if self.at(&TokenKind::RParen) {
            self.advance();
            return Ok(self.call(head, &args));
        }
        loop {
            args.push(self.parse_expr()?);
            if self.at(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::RParen, "expected a closing parenthesis")?;
        Ok(self.call(head, &args))
    }

    fn parse_list(&mut self) -> Result<ExprId, Diagnostic> {
        let mut args = Vec::new();
        if self.at(&TokenKind::RBracket) {
            self.advance();
            return Ok(self.call_name("list", &args));
        }
        loop {
            args.push(self.parse_expr()?);
            if self.at(&TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBracket, "expected a closing bracket")?;
        Ok(self.call_name("list", &args))
    }

    fn parse_function(&mut self) -> Result<ExprId, Diagnostic> {
        self.expect(TokenKind::LParen, "expected function parameters")?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let token = match self.advance() {
                    Some(token) => token.clone(),
                    None => {
                        return Err(self.error(ErrorCode::ParseUnexpectedToken, "expected a name"));
                    }
                };
                let name = match token.kind {
                    TokenKind::Ident(name) => name,
                    _ => {
                        return Err(Diagnostic::parse(
                            ErrorCode::ParseUnexpectedToken,
                            "function parameters must be names",
                            token.span,
                        ));
                    }
                };
                params.push(self.symbol(&name));
                if self.at(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected a closing parenthesis")?;
        self.expect(TokenKind::FatArrow, "expected => after function parameters")?;
        let body = self.parse_expr()?;
        let params = self.call_name("list", &params);
        Ok(self.call_name("function", &[params, body]))
    }

    fn number(&mut self, text: &str, span: Span) -> Result<ExprId, Diagnostic> {
        let value = if let Some((whole, fraction)) = text.split_once('.') {
            let whole = Int::from_decimal(whole).map_err(|error| self.number_error(error, span))?;
            let fraction_value =
                Int::from_decimal(fraction).map_err(|error| self.number_error(error, span))?;
            let scale = Int::from_i64(10).pow_u32(fraction.len() as u32);
            let numerator = whole.mul(&scale).add(&fraction_value);
            Rat::from_ints(numerator, scale).map_err(|error| self.number_error(error, span))?
        } else {
            let integer =
                Int::from_decimal(text).map_err(|error| self.number_error(error, span))?;
            return Ok(self.arena.intern_int(integer));
        };
        Ok(self.arena.intern_rat(value))
    }

    fn number_error(&self, error: NumError, span: Span) -> Diagnostic {
        Diagnostic::parse(
            ErrorCode::ParseInvalidNumber,
            format!("invalid number: {error}"),
            span,
        )
    }

    fn negate_value(&mut self, value: ExprId) -> Result<ExprId, Diagnostic> {
        let integer = match self.arena.node(value) {
            crate::expr::Node::Int(number) => Some(number.clone()),
            _ => None,
        };
        if let Some(number) = integer {
            return Ok(self.arena.intern_int(number.neg()));
        }
        let rational = match self.arena.node(value) {
            crate::expr::Node::Rat(number) => Some(number.clone()),
            _ => None,
        };
        if let Some(number) = rational {
            return Ok(self.arena.intern_rat(number.neg()));
        }
        Ok(self.call_name("negate", &[value]))
    }

    fn integer(&mut self, value: i64) -> ExprId {
        self.arena.intern_int(Int::from_i64(value))
    }

    fn as_rat(&self, id: ExprId) -> Option<Rat> {
        match self.arena.node(id) {
            crate::expr::Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
            crate::expr::Node::Rat(value) => Some(value.clone()),
            _ => None,
        }
    }
}

fn starts_primary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Number(_) | TokenKind::Ident(_) | TokenKind::LParen
    )
}

fn is_comparison(kind: Option<&TokenKind>) -> bool {
    matches!(
        kind,
        Some(TokenKind::Equal3)
            | Some(TokenKind::Equal2)
            | Some(TokenKind::NotEqual)
            | Some(TokenKind::Tilde)
            | Some(TokenKind::Less)
            | Some(TokenKind::LessEqual)
            | Some(TokenKind::Greater)
            | Some(TokenKind::GreaterEqual)
    )
}

/// Fast path for whitespace free atom calls, lists, and integer literals.
fn parse_simple_expression(arena: &mut Arena, source: &str) -> Option<ExprId> {
    if source.is_empty()
        || source.bytes().any(|byte| byte.is_ascii_whitespace())
        || !source.is_ascii()
        || source.bytes().any(|byte| {
            matches!(
                byte,
                b'+' | b'-' | b'*' | b'/' | b'^' | b'<' | b'>' | b'=' | b'!' | b'&' | b'|'
            )
        })
    {
        return None;
    }
    let mut parser = SimpleParser {
        arena,
        bytes: source.as_bytes(),
        position: 0,
    };
    let expression = parser.value(0)?;
    if parser.position != parser.bytes.len() {
        return None;
    }
    Some(expression)
}

struct SimpleParser<'a> {
    arena: &'a mut Arena,
    bytes: &'a [u8],
    position: usize,
}

impl SimpleParser<'_> {
    fn value(&mut self, depth: usize) -> Option<ExprId> {
        if depth > 32 {
            return None;
        }
        match self.bytes.get(self.position).copied()? {
            b'-' => {
                self.position += 1;
                let start = self.position;
                while self
                    .bytes
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
                if self.position == start {
                    return None;
                }
                let text = core::str::from_utf8(&self.bytes[start - 1..self.position]).ok()?;
                let value = Int::from_decimal(text).ok()?;
                Some(self.arena.intern_int(value))
            }
            byte if byte.is_ascii_digit() => {
                let start = self.position;
                while self
                    .bytes
                    .get(self.position)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.position += 1;
                }
                let text = core::str::from_utf8(&self.bytes[start..self.position]).ok()?;
                let value = Int::from_decimal(text).ok()?;
                Some(self.arena.intern_int(value))
            }
            b'[' => {
                self.position += 1;
                let mut values = Vec::new();
                if self.bytes.get(self.position) == Some(&b']') {
                    self.position += 1;
                    let head = self.arena.builtin_head("list");
                    return Some(self.arena.intern_call(head, &values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    match self.bytes.get(self.position).copied()? {
                        b',' => self.position += 1,
                        b']' => {
                            self.position += 1;
                            let head = self.arena.builtin_head("list");
                            return Some(self.arena.intern_call(head, &values));
                        }
                        _ => return None,
                    }
                }
            }
            byte if byte.is_ascii_alphabetic() => {
                let start = self.position;
                self.position += 1;
                while self
                    .bytes
                    .get(self.position)
                    .is_some_and(|byte| byte.is_ascii_alphanumeric())
                {
                    self.position += 1;
                }
                let name = core::str::from_utf8(&self.bytes[start..self.position]).ok()?;
                let symbol = self.arena.symbol_or_intern(name);
                let head = self.arena.symbol_expr(symbol);
                if self.bytes.get(self.position) != Some(&b'(') {
                    return Some(head);
                }
                self.position += 1;
                let mut arguments = Vec::new();
                if self.bytes.get(self.position) == Some(&b')') {
                    self.position += 1;
                    return Some(self.arena.intern_call(head, &arguments));
                }
                loop {
                    arguments.push(self.value(depth + 1)?);
                    match self.bytes.get(self.position).copied()? {
                        b',' => self.position += 1,
                        b')' => {
                            self.position += 1;
                            return Some(self.arena.intern_call(head, &arguments));
                        }
                        _ => return None,
                    }
                }
            }
            _ => None,
        }
    }
}

fn parse_two_integer_operation(arena: &mut Arena, source: &str) -> Option<ExprId> {
    if source.len() < 3 || !source.is_ascii() {
        return None;
    }
    let bytes = source.as_bytes();
    let mut operator = None;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if index == 0 || index + 1 == bytes.len() || byte.is_ascii_digit() {
            continue;
        }
        if !matches!(byte, b'+' | b'-' | b'*' | b'/' | b'^') {
            return None;
        }
        if operator.is_some() {
            return None;
        }
        operator = Some((index, byte));
    }
    let (index, operator) = operator?;
    let left_value = Int::from_decimal(&source[..index]).ok()?;
    let right_value = Int::from_decimal(&source[index + 1..]).ok()?;
    let left = arena.intern_int(left_value.clone());
    let right = arena.intern_int(right_value.clone());
    let result = match operator {
        b'+' => {
            let head = arena.builtin_head("plus");
            arena.intern_call(head, &[left, right])
        }
        b'-' => {
            let negate = arena.builtin_head("negate");
            let right = arena.intern_call(negate, &[right]);
            let head = arena.builtin_head("plus");
            arena.intern_call(head, &[left, right])
        }
        b'*' => {
            let head = arena.builtin_head("times");
            arena.intern_call(head, &[left, right])
        }
        b'/' => {
            if right_value.is_zero() {
                return None;
            }
            let rational = Rat::from_ints(left_value, right_value).ok()?;
            arena.intern_rat(rational)
        }
        b'^' => {
            let head = arena.builtin_head("power");
            arena.intern_call(head, &[left, right])
        }
        _ => return None,
    };
    Some(result)
}
