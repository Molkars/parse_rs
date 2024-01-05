use std::borrow::Cow;

#[derive(Debug)]
pub struct Token<'a> {
    pub span: Span,
    pub content: Cow<'a, str>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start: Location,
    pub end: Location,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub location: Location,
}

impl<'a> Token<'a> {
    pub fn content(&self) -> &str {
        self.content.as_ref()
    }
}

impl Location {
    #[inline]
    pub fn zero() -> Self {
        Location {
            line: 0,
            column: 0,
            index: 0
        }
    }
}

impl Span {
    #[inline]
    pub fn len(&self) -> usize {
        self.end.index - self.start.index
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl PartialEq<str> for Token<'_> {
    #[inline]
    fn eq(&self, rhs: &str) -> bool {
        self.content.as_ref() == rhs
    }
}

impl PartialEq<&'_ str> for Token<'_> {
    #[inline]
    fn eq(&self, rhs: &&str) -> bool {
        (*rhs) == self.content.as_ref()
    }
}
