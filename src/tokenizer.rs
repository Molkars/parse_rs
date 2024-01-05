use crate::{Location, Span, Token, Error};
use std::cell::Cell;
use std::borrow::Cow;

pub struct Tokenizer<'src> {
    source: &'src str,
    location: Cell<Location>,
}

impl<'src> Tokenizer<'src> {
    #[inline]
    fn shimmy(&self) {
        let mut loc = self.loc();
        let mut chars = self.source[loc.index..].chars();
        while let Some(c) = chars.next() {
            if c.is_whitespace() {
                Self::adv(&mut loc, c);
            } else {
                break;
            }
        }
        self.location.set(loc);
    }

    #[inline(always)]
    fn loc(&self) -> Location {
        self.location.get()
    }

    #[inline]
    pub fn adv(l: &mut Location, c: char) {
        l.index += c.len_utf8();
        match c {
            '\n' => {
                l.line += 1;
                l.column = 0;
            }
            _ => {
                l.column += 1;
            }
        }
    }

    fn word_span(&self, s: &str) -> Option<Span> {
        self.shimmy();
        let start = self.loc();
        let mut end = start;
        let mut cursor = self.source[start.index..].chars();
        for c in s.chars() {
            let Some(o) = cursor.next() else {
                return None;
            };
            if c != o {
                return None;
            }
            Self::adv(&mut end, c);
        }
        match cursor.next() {
            Some(c) if !c.is_whitespace() => None,
            _ => Some(Span { start, end })
        }
    }
}

impl<'src> Tokenizer<'src> {
    #[inline]
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            location: Cell::new(Location::zero()),
        }
    }

    #[inline]
    pub fn source(&self) -> &'src str {
        self.source
    }
    
    #[inline]
    pub fn cursor(&self) -> &'src str {
        self.shimmy();
        &self.source[self.loc().index..]
    }

    pub fn location(&self) -> Location {
        self.shimmy();
        self.loc()
    }

    #[inline]
    pub fn has_more_tokens(&self) -> bool {
        !self.cursor().is_empty()
    }

    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.cursor().chars().next()
    }

    #[inline]
    pub fn advance(&self) -> Option<char> {
        let c = self.peek()?;
        let mut loc = self.loc();
        Self::adv(&mut loc, c);
        self.location.set(loc);
        Some(c)
    }

    #[inline]
    pub fn peek_word(&self) -> Option<Span> {
        self.peek_while(|c| !c.is_whitespace())
    }

    pub fn peek_while(&self, f: impl Fn(char) -> bool) -> Option<Span> {
        let mut cursor = self.cursor().chars().take_while(|c| f(*c));
        let start = self.loc();
        let mut end = start;
        for c in cursor {
            Self::adv(&mut end, c);
        }
        (start != end).then(|| Span { start, end })
    }

    #[inline]
    pub fn match_word(&self, s: &str) -> bool {
        self.word_span(s).is_some()
    }

    #[inline]
    pub fn consume_word(&self, s: &str) -> Option<Token<'src>> {
        self.word_span(s)
            .map(|span| {
                self.location.set(span.end);
                Token {
                    span,
                    content: Cow::Borrowed(self.lex_for(span).unwrap()) 
                }
            })
    }

    #[inline]
    pub fn cursor_for(&self, loc: Location) -> Option<&'src str> {
        (loc.index < self.source.len())
            .then(|| &self.source[loc.index..])
    }

    #[inline]
    pub fn lex_for(&self, span: Span) -> Option<&'src str> {
        self.cursor_for(span.start)
            .and_then(|cursor| {
                (span.len() <= cursor.len())
                    .then(|| &cursor[..span.len()])
            })
    }

    #[inline]
    pub fn peek_str(&self, str: &str) -> Option<Span> {
        let start = self.location();
        let mut end = start;
        for (a, b) in self.cursor().chars().zip(str.chars()) {
            if a != b {
                return None;
            }
            Self::adv(&mut end, a);
        }
        (start != end)
            .then(|| Span { start, end })
    }

    #[inline]
    pub fn consume_while(&self, f: impl Fn(char) -> bool) -> Option<Token<'src>> {
        let start = self.location();
        let mut end = start;
        let iter = self.cursor().chars().take_while(|c| f(*c));
        for c in iter {
            Self::adv(&mut end, c);
        }
        (start != end)
            .then(|| {
                self.location.set(end);
                let span = Span { start, end };
                Token {
                    span,
                    content: Cow::Borrowed(self.lex_for(span)
                        .expect("this should be unreachable"))
                }
            })
    }

    #[inline]
    pub fn consume(&self, s: &str) -> Option<Token<'src>> {
        self.peek_str(s).map(|_| {
            let start = self.loc();
            let mut end = start;
            for c in s.chars() {
                Self::adv(&mut end, c);
            }
            self.location.set(end);
            let span = Span { start, end };
            Token {
                span,
                content: Cow::Borrowed(self.lex_for(span).unwrap()),
            }
        })
    }

    #[inline]
    pub fn expect(&self, s: &str) -> Result<Token<'src>, Error> {
        let start = self.location();
        self.consume(s)
            .ok_or_else(|| Error {
                location: start,
                message: format!("Expected `{s:?}`"),
            })
    }
}

