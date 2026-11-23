//! Reader for the documented single-layer ASCII USD subset.

use std::collections::BTreeMap;

use super::{Error, invalid};

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Word(&'a str),
    Text(String),
    Path(&'a str),
    Punct(char),
}

#[derive(Debug)]
pub(super) enum Data {
    Word(String),
    Text(String),
    Path(String),
    List(Vec<Self>),
    Tuple(Vec<Self>),
    Map(Vec<(f64, Self)>),
}

impl Data {
    pub fn text(&self) -> Result<&str, Error> {
        if let Self::Text(s) = self {
            Ok(s)
        } else {
            Err(invalid("expected quoted token"))
        }
    }
    pub fn path(&self) -> Result<&str, Error> {
        if let Self::Path(s) = self {
            Ok(s)
        } else {
            Err(invalid("expected prim path"))
        }
    }
    pub fn list(&self) -> Result<&[Data], Error> {
        if let Self::List(a) = self {
            Ok(a)
        } else {
            Err(invalid("expected array"))
        }
    }
    pub fn tuple<const N: usize>(&self) -> Result<[f64; N], Error> {
        let Self::Tuple(a) = self else {
            return Err(invalid("expected numeric tuple"));
        };
        if a.len() != N {
            return Err(invalid("incorrect tuple arity"));
        }
        let mut out = [0.0; N];
        for (v, n) in out.iter_mut().zip(a) {
            *v = n.number()?;
        }
        Ok(out)
    }
    pub fn number(&self) -> Result<f64, Error> {
        if let Self::Word(s) = self
            && let Ok(n) = s.parse::<f64>()
            && n.is_finite()
        {
            return Ok(n);
        }
        Err(invalid("expected finite number"))
    }
    pub fn index(&self) -> Result<u32, Error> {
        if let Self::Word(s) = self
            && let Ok(n) = s.parse::<u32>()
        {
            return Ok(n);
        }
        Err(invalid("expected non-negative integer"))
    }
}

type Meta = BTreeMap<String, Data>;

pub(super) struct Prim {
    pub kind: String,
    pub name: String,
    pub meta: Meta,
    pub fields: BTreeMap<String, (Data, Meta)>,
    pub children: Vec<Prim>,
}

impl Prim {
    pub fn attr(&self, key: &str) -> Option<&Data> {
        self.fields.get(key).map(|(value, _)| value)
    }
    pub fn display_name(&self) -> Result<&str, Error> {
        self.meta
            .get("displayName")
            .map_or(Ok(&self.name), Data::text)
    }
}

pub(super) fn parse(source: &str) -> Result<(Meta, Vec<Prim>), Error> {
    let (header, source) = source
        .split_once('\n')
        .ok_or_else(|| invalid("missing USDA header"))?;
    if header.trim() != "#usda 1.0" {
        return Err(invalid("expected #usda 1.0"));
    }
    let mut p = Parser {
        tokens: lex(source)?.into_iter().peekable(),
    };
    let metadata = if p.eat('(') { p.meta()? } else { Meta::new() };
    let mut roots = Vec::new();
    while p.tokens.peek().is_some() {
        roots.push(p.prim(0)?);
    }
    Ok((metadata, roots))
}

fn lex(mut s: &str) -> Result<Vec<Token<'_>>, Error> {
    let mut out = Vec::new();
    while !s.is_empty() {
        s = s.trim_start();
        if s.is_empty() {
            break;
        }
        let first = s.as_bytes()[0];
        match first {
            b'#' => {
                s = s.find('\n').map_or("", |e| &s[e..]);
            }
            b'"' => {
                let mut escaped = false;
                let end = s
                    .as_bytes()
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find_map(|(i, &c)| {
                        let close = c == b'"' && !escaped;
                        escaped = c == b'\\' && !escaped;
                        close.then_some(i + 1)
                    })
                    .ok_or_else(|| invalid("unterminated quoted string"))?;
                let value = serde_json::from_str(&s[..end])
                    .map_err(|_| invalid("invalid quoted string"))?;
                out.push(Token::Text(value));
                s = &s[end..];
            }
            b'<' => {
                let end = s.find('>').ok_or_else(|| invalid("unterminated path"))?;
                out.push(Token::Path(&s[1..end]));
                s = &s[end + 1..];
            }
            b'@' => return Err(Error::Unsupported("external assets")),
            c if b"()[]{}=,:;".contains(&c) => {
                out.push(Token::Punct(char::from(c)));
                s = &s[1..];
            }
            _ => {
                let numeric = first.is_ascii_digit() || b"+-.".contains(&first);
                let end = s
                    .find(|c: char| {
                        c.is_whitespace() || "()[]{}=,;\"<>@#".contains(c) || (numeric && c == ':')
                    })
                    .unwrap_or(s.len());
                if end == 0 {
                    return Err(invalid("invalid USDA token"));
                }
                out.push(Token::Word(&s[..end]));
                s = &s[end..];
            }
        }
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: std::iter::Peekable<std::vec::IntoIter<Token<'a>>>,
}

impl Parser<'_> {
    fn eat(&mut self, c: char) -> bool {
        self.tokens.next_if_eq(&Token::Punct(c)).is_some()
    }
    fn expect(&mut self, c: char) -> Result<(), Error> {
        if self.eat(c) {
            Ok(())
        } else {
            Err(invalid(format!("expected {c}")))
        }
    }
    fn word(&mut self) -> Result<String, Error> {
        match self.tokens.next() {
            Some(Token::Word(w)) => Ok(w.into()),
            _ => Err(invalid("expected identifier")),
        }
    }
    fn meta(&mut self) -> Result<Meta, Error> {
        let mut m = Meta::new();
        while !self.eat(')') {
            let mut key = self.word()?;
            if matches!(key.as_str(), "prepend" | "append" | "delete" | "reorder") {
                key = self.word()?;
            }
            if matches!(
                key.as_str(),
                "references" | "payload" | "inherits" | "specializes" | "subLayers" | "variants"
            ) {
                return Err(Error::Unsupported("layer composition"));
            }
            self.expect('=')?;
            m.insert(key, self.value(0)?);
            self.eat(';');
        }
        Ok(m)
    }
    fn prim(&mut self, depth: usize) -> Result<Prim, Error> {
        if depth >= 128 {
            return Err(invalid("prim nesting limit"));
        }
        if self.word()? != "def" {
            return Err(Error::Unsupported("non-def prim"));
        }
        let kind = self.word()?;
        let Some(Token::Text(name)) = self.tokens.next() else {
            return Err(invalid("expected prim name"));
        };
        let meta = if self.eat('(') {
            self.meta()?
        } else {
            Meta::new()
        };
        self.expect('{')?;
        let mut children = Vec::new();
        let mut fields = BTreeMap::new();
        while !self.eat('}') {
            if matches!(self.tokens.peek(), Some(Token::Word("def"))) {
                children.push(self.prim(depth + 1)?);
                continue;
            }
            let mut ty = self.word()?;
            while matches!(ty.as_str(), "custom" | "uniform" | "varying" | "prepend") {
                ty = self.word()?;
            }
            if self.eat('[') {
                self.expect(']')?;
            }
            let key = self.word()?;
            if matches!(
                key.as_str(),
                "references" | "payload" | "inherits" | "specializes"
            ) {
                return Err(Error::Unsupported("prim composition"));
            }
            let value = if self.eat('=') {
                Some(self.value(0)?)
            } else {
                None
            };
            let attr_meta = if self.eat('(') {
                self.meta()?
            } else {
                Meta::new()
            };
            if let Some(value) = value
                && fields.insert(key, (value, attr_meta)).is_some()
            {
                return Err(invalid("duplicate property"));
            }
            self.eat(';');
        }
        Ok(Prim {
            kind,
            name,
            meta,
            fields,
            children,
        })
    }
    fn value(&mut self, depth: usize) -> Result<Data, Error> {
        if depth >= 128 {
            return Err(invalid("value nesting limit"));
        }
        match self.tokens.next() {
            Some(Token::Word(w)) => Ok(Data::Word(w.into())),
            Some(Token::Text(t)) => Ok(Data::Text(t)),
            Some(Token::Path(p)) => Ok(Data::Path(p.into())),
            Some(Token::Punct(c @ ('(' | '['))) => {
                let close = if c == '(' { ')' } else { ']' };
                let mut values = Vec::new();
                while !self.eat(close) {
                    values.push(self.value(depth + 1)?);
                    if !self.eat(',') {
                        self.expect(close)?;
                        break;
                    }
                }
                Ok(if c == '(' {
                    Data::Tuple(values)
                } else {
                    Data::List(values)
                })
            }
            Some(Token::Punct('{')) => {
                let mut values = Vec::new();
                while !self.eat('}') {
                    let time = Data::Word(self.word()?).number()?;
                    self.expect(':')?;
                    values.push((time, self.value(depth + 1)?));
                    if !self.eat(',') {
                        self.expect('}')?;
                        break;
                    }
                }
                Ok(Data::Map(values))
            }
            _ => Err(invalid("expected USDA value")),
        }
    }
}
