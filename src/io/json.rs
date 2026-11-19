//! A small JSON reader and value tree.

use std::string::String;
use std::vec::Vec;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// JSON null.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number, stored as its source text.
    Number(String),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object with ordered keys.
    Object(Vec<(String, Json)>),
}

/// Parse JSON text.
pub fn parse(text: &str) -> Result<Json, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
    };
    let value = parser.value()?;
    parser.skip_space();
    if parser.position != parser.bytes.len() {
        return Err(format!("unexpected JSON text at byte {}", parser.position));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn value(&mut self) -> Result<Json, String> {
        self.skip_space();
        match self.bytes.get(self.position).copied() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Json::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Json::Bool(false))
            }
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(format!("expected a JSON value at byte {}", self.position)),
        }
    }

    fn literal(&mut self, text: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.position..self.position + text.len()) == Some(text) {
            self.position += text.len();
            Ok(())
        } else {
            Err(format!("expected JSON literal at byte {}", self.position))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.position += 1;
        let mut out = String::new();
        while let Some(byte) = self.bytes.get(self.position).copied() {
            self.position += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = self.bytes.get(self.position).copied().ok_or_else(|| {
                        format!("unterminated JSON escape at byte {}", self.position)
                    })?;
                    self.position += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.hex4()?;
                            let character = char::from_u32(u32::from(code)).ok_or_else(|| {
                                format!("invalid JSON code point at byte {}", self.position)
                            })?;
                            out.push(character);
                        }
                        _ => {
                            return Err(format!("invalid JSON escape at byte {}", self.position));
                        }
                    }
                }
                _ => {
                    let start = self.position - 1;
                    let character = core::str::from_utf8(&self.bytes[start..])
                        .ok()
                        .and_then(|text| text.chars().next())
                        .ok_or_else(|| format!("invalid JSON text at byte {start}"))?;
                    self.position = start + character.len_utf8();
                    out.push(character);
                }
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let mut value = 0u16;
        for _ in 0..4 {
            let byte = self
                .bytes
                .get(self.position)
                .copied()
                .ok_or_else(|| format!("unterminated JSON escape at byte {}", self.position))?;
            self.position += 1;
            let digit = match byte {
                b'0'..=b'9' => u16::from(byte - b'0'),
                b'a'..=b'f' => u16::from(byte - b'a') + 10,
                b'A'..=b'F' => u16::from(byte - b'A') + 10,
                _ => return Err(format!("invalid JSON escape at byte {}", self.position)),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn array(&mut self) -> Result<Json, String> {
        self.position += 1;
        let mut values = Vec::new();
        self.skip_space();
        if self.bytes.get(self.position) == Some(&b']') {
            self.position += 1;
            return Ok(Json::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.skip_space();
            match self.bytes.get(self.position).copied() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(Json::Array(values));
                }
                _ => return Err(format!("expected ] at byte {}", self.position)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.position += 1;
        let mut entries = Vec::new();
        self.skip_space();
        if self.bytes.get(self.position) == Some(&b'}') {
            self.position += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_space();
            if self.bytes.get(self.position) != Some(&b'"') {
                return Err(format!("expected an object key at byte {}", self.position));
            }
            let key = self.string()?;
            self.skip_space();
            if self.bytes.get(self.position) != Some(&b':') {
                return Err(format!("expected : at byte {}", self.position));
            }
            self.position += 1;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_space();
            match self.bytes.get(self.position).copied() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err(format!("expected }} at byte {}", self.position)),
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.position;
        while let Some(byte) = self.bytes.get(self.position).copied() {
            if byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.position += 1;
            } else {
                break;
            }
        }
        let text = core::str::from_utf8(&self.bytes[start..self.position])
            .map_err(|_| "invalid JSON number".to_string())?;
        if text.is_empty() {
            return Err(format!("expected a JSON number at byte {start}"));
        }
        Ok(Json::Number(text.to_string()))
    }

    fn skip_space(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.position += 1;
        }
    }
}
