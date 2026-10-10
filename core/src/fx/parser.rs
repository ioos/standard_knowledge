//! Recursive descent parser producing the same postfix token order as the
//! pyparsing grammar in ioos_qc's `fx_parser.BNF()`.

use super::{FxError, Token};

/// pyparsing's default whitespace characters
fn is_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

/// Characters allowed after the first letter of a word: `alphanums + "_$"`
fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    out: Vec<Token>,
}

pub(super) fn parse(source: &str) -> Result<Vec<Token>, FxError> {
    let mut parser = Parser {
        src: source.as_bytes(),
        pos: 0,
        out: Vec::new(),
    };
    parser.expr()?;
    parser.skip_whitespace();
    if parser.pos < parser.src.len() {
        return Err(parser.error("Expected end of text"));
    }
    Ok(parser.out)
}

impl Parser<'_> {
    fn error(&self, message: &str) -> FxError {
        // Report the position as a character offset, like pyparsing does
        let position = String::from_utf8_lossy(&self.src[..self.pos.min(self.src.len())])
            .chars()
            .count();
        FxError::Parse {
            position,
            message: message.to_string(),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.src.len() && is_whitespace(self.src[self.pos]) {
            self.pos += 1;
        }
    }

    /// Skip whitespace and return the next byte without consuming it
    fn peek(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.src.get(self.pos).copied()
    }

    /// `expr :: term [ addop term ]*`
    fn expr(&mut self) -> Result<(), FxError> {
        self.term()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.pos += 1;
            self.term()?;
            self.out.push(Token::BinOp(op as char));
        }
        Ok(())
    }

    /// `term :: factor [ multop factor ]*`
    fn term(&mut self) -> Result<(), FxError> {
        self.factor()?;
        while let Some(op @ (b'*' | b'/')) = self.peek() {
            self.pos += 1;
            self.factor()?;
            self.out.push(Token::BinOp(op as char));
        }
        Ok(())
    }

    /// `factor :: atom [ expop factor ]*`, so `^` is right-associative
    fn factor(&mut self) -> Result<(), FxError> {
        self.atom()?;
        if self.peek() == Some(b'^') {
            self.pos += 1;
            self.factor()?;
            self.out.push(Token::BinOp('^'));
        }
        Ok(())
    }

    /// `atom :: addop* ( fn_call | PI | E | number | ident | '(' expr ')' )`
    fn atom(&mut self) -> Result<(), FxError> {
        let mut signs = Vec::new();
        while let Some(sign @ (b'+' | b'-')) = self.peek() {
            self.pos += 1;
            signs.push(sign);
        }

        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                self.expr()?;
                if self.peek() != Some(b')') {
                    return Err(self.error("Expected ')'"));
                }
                self.pos += 1;
            }
            Some(c) if c.is_ascii_alphabetic() => self.word()?,
            Some(c) if c.is_ascii_digit() => self.number(),
            _ => return Err(self.error("Expected an operand")),
        }

        // ioos_qc's push_unary_minus pushes one negation per leading '-' and
        // stops at the first '+', so "+-3" is 3 and "-+-3" is -3.
        for sign in signs {
            if sign != b'-' {
                break;
            }
            self.out.push(Token::UnaryMinus);
        }
        Ok(())
    }

    /// A word: a function call, the `PI` or `E` keyword, or an identifier
    fn word(&mut self) -> Result<(), FxError> {
        let start = self.pos;
        while self.pos < self.src.len() && is_word_char(self.src[self.pos]) {
            self.pos += 1;
        }
        // Only ASCII bytes were consumed, so this slice is valid UTF-8
        let name = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();

        if self.peek() == Some(b'(') {
            self.pos += 1;
            let mut arg_count = 0;
            loop {
                self.expr()?;
                arg_count += 1;
                match self.peek() {
                    Some(b',') => self.pos += 1,
                    Some(b')') => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(self.error("Expected ',' or ')'")),
                }
            }
            self.out.push(Token::Call(name, arg_count));
        } else if name.eq_ignore_ascii_case("pi") {
            self.out.push(Token::Pi);
        } else if name.eq_ignore_ascii_case("e") {
            self.out.push(Token::E);
        } else {
            self.out.push(Token::Ident(name));
        }
        Ok(())
    }

    /// `number :: digit+ [ '.' digit* ] [ ('e' | 'E') [ '+' | '-' ] digit+ ]`
    fn number(&mut self) {
        let start = self.pos;
        self.digits();
        if self.src.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            self.digits();
        }
        // The exponent is only taken when it is complete, otherwise the
        // number ends before the 'e'
        if matches!(self.src.get(self.pos), Some(b'e' | b'E')) {
            let mut end = self.pos + 1;
            if matches!(self.src.get(end), Some(b'+' | b'-')) {
                end += 1;
            }
            if self.src.get(end).is_some_and(u8::is_ascii_digit) {
                self.pos = end;
                self.digits();
            }
        }
        let text = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
        self.out.push(Token::Number(text));
    }

    fn digits(&mut self) {
        while self.src.get(self.pos).is_some_and(u8::is_ascii_digit) {
            self.pos += 1;
        }
    }
}
