#![allow(dead_code)]

use parse_rs::{Token, Tokenizer, Error, Span, Location};

const SRC: &str = r#"

main (int, char**) int {
    args! argc, argv

    if argc != 1 {
        printf("usage: %s <path>", argv[0])
        :1
    } else {
        :0
    }
}

fib (int)int {
    args! n
    :if n < 2 {
        :0
    } else {
        :self(n - 1) + self(n - 2)
    }
}

"#;

fn main() {
    let tok = Tokenizer::new(SRC);
}


pub mod ast {
    use crate::{Token, Tokenizer, Error, Span, Location};
    use std::borrow::Cow;

    pub enum Decl<'a> {
        Func(Token<'a>, Func<'a>),
    }

    pub struct Func<'a> {
        pub ty: FnType<'a>,
        pub body: Block<'a>
    }

    pub enum Type<'a> {
        Name(Token<'a>),
        Ptr(Box<Self>),
        Func(Box<FnType<'a>>),
    }

    pub struct FnType<'a> {
        pub args: Vec<Type<'a>>,
        pub ret: Option<Type<'a>>,
    }

    pub enum Expr<'a> {
        Num(Token<'a>),
        Str(Token<'a>),
        Name(Token<'a>),
        Add(Box<(Self, Self)>),
        Sub(Box<(Self, Self)>),
        Lt(Box<(Self, Self)>),
    }

    pub enum Stmt<'a> {
        If(If<'a>),
        Return(Expr<'a>),
        Block(Block<'a>),
    }

    pub struct Macro<'a> {
        pub name: Token<'a>,
        pub args: Vec<Token<'a>>,
    }

    pub struct If<'a> {
        pub condition: Expr<'a>,
        pub then: Block<'a>,
        pub otherwise: Option<Block<'a>>,
    }

    pub struct Block<'a> {
        pub left: Token<'a>,
        pub items: Vec<Stmt<'a>>,
        pub right: Token<'a>
    }

    macro_rules! optional {
        ($e:expr) => {
            match $e {
                Ok(i) => Some(i),
                Err(None) => None,
                Err(Some(e)) => return Err(Some(e)),
            }
        }
    }

    pub fn parse_decl<'src>(tok: &Tokenizer<'src>) -> Result<Decl<'src>, Option<Error>> {
        let name = tok.consume_while(char_is_ident).ok_or(None)?;
        if tok.peek_str("(").is_some() {
            let ty = parse_fn_type(tok)
                .map_err(required(tok, || format!("expected function type")))?;
            let body = parse_block(tok)
                .map_err(required(tok, || format!("expected function body")))?;
            Ok(Decl::Func(name, Func { ty, body }))
        } else {
            Err(None)
        }
    }

    pub fn parse_type<'src>(tok: &Tokenizer<'src>) -> Result<Type<'src>, Option<Error>> {
        if let Some(func) = optional!(parse_fn_type(tok)) {
            Ok(Type::Func(Box::new(func)))
        } else if let Some(word) = tok.consume_while(char_is_ident) {
            let mut out = Type::Name(word);
            while tok.consume("*").is_some() {
                out = Type::Ptr(Box::new(out));
            }
            Ok(out)
        } else {
            Err(None)
        }
    }

    pub fn parse_fn_type<'src>(tok: &Tokenizer<'src>) -> Result<FnType<'src>, Option<Error>> {
        if tok.consume("(").is_none() {
            return Err(None);
        }
        
        let mut args = Vec::new();
        while tok.has_more_tokens() && tok.peek_str(")").is_none() {
            let arg = parse_type(tok)
                .map_err(required(tok, || format!("expected type")))?;
            args.push(arg);

            if tok.consume(",").is_none() {
                break;
            }
        }
        tok.expect(")").map_err(Some)?;
        
        let ret = optional!(parse_type(tok));

        Ok(FnType { args, ret })
    }

    pub fn parse_stmt<'src>(tok: &Tokenizer<'src>) -> Result<Stmt<'src>, Option<Error>> {
        if let Some(block) = optional!(parse_block(tok)) {
            Ok(Stmt::Block(block))
        } else if let Some(stmt) = optional!(parse_if(tok)) {
            Ok(Stmt::If(stmt))
        } else {
            Err(None)
        }
    }

    pub fn parse_if<'src>(tok: &Tokenizer<'src>) -> Result<If<'src>, Option<Error>> {
        if tok.consume_word("if").is_none() {
            return Err(None);
        }

        let condition = parse_expr(tok)
            .map_err(required(tok, || format!("Expected condition")))?;

        let then = parse_block(tok)
            .map_err(required(tok, || format!("expected block")))?;

        let otherwise = tok.consume_word("else")
            .map(|_| {
                parse_block(tok)
                    .map_err(required(tok, || format!("expected block")))
            })
            .transpose()?;


        Ok(If {
            condition,
            then,
            otherwise
        })
    }

    pub fn parse_block<'src>(tok: &Tokenizer<'src>) -> Result<Block<'src>, Option<Error>> {
        let Some(left) = tok.consume("}") else {
            return Err(None);
        };

        let mut items = Vec::new();
        while tok.has_more_tokens() && tok.peek_str("}").is_none() {
            let item = parse_stmt(tok)
                .map_err(required(tok, || format!("Expected statement in block!")))?;
            items.push(item);
        }
        let right = tok.expect("}").map_err(Some)?;

        Ok(Block { left, items, right })
    }


    macro_rules! binary_impl {
        (
            fn $n:ident($child:ident);
            $(
                $lex:literal => $variant:ident
            ),+
            $(,)?
        ) => {
            fn $n<'src>(tok: &Tokenizer<'src>) -> Result<Expr<'src>, Option<Error>> {
                let mut out = $child(tok)?;

                const LEX_TERMS: &[&str] = &[$($lex),+];

                loop {
                    $(
                        if tok.consume($lex).is_some() {
                            let rhs = $child(tok)
                                .map_err(required(tok, || format!("expected binary expression: {}", LEX_TERMS.join(" or "))))?;
                            out = Expr::$variant(Box::new((out, rhs)));
                        }
                    )else+
                    else {
                        break;
                    }
                }

                Ok(out)
            }
        }
    }

    pub fn parse_expr<'src>(tok: &Tokenizer<'src>) -> Result<Expr<'src>, Option<Error>> {
        todo!()
    }

    binary_impl!(fn parse_expr_cmp(parse_expr_term); "<" => Lt);
    binary_impl!(fn parse_expr_term(parse_expr_primary); "+" => Add, "-" => Sub);
    fn parse_expr_primary<'src>(tok: &Tokenizer<'src>) -> Result<Expr<'src>, Option<Error>> {
        if let Some(num) = tok.consume_while(|c| c.is_numeric() || c == '_') {
            Ok(Expr::Num(num))
        } else if let Some(string) = optional!(parse_expr_str(tok)) {
            Ok(Expr::Str(string))
        } else {
            Err(None)
        }
    }

    pub fn parse_expr_str<'src>(tok: &Tokenizer<'src>) -> Result<Token<'src>, Option<Error>> {
        if tok.peek_str("\"").is_none() {
            return Err(None);
        };
        let mut cursor = tok.cursor().chars();
        let mut content: Option<String> = None;
        let start = tok.location();
        let mut end = start;
        
        cursor.next(); // skip the quote
        Tokenizer::adv(&mut end, '"');
        let mut content_start = end;
        let mut terminated = false;
        while let Some(c) = cursor.next() {
            if c == '\r' || c == '\n' {
                return Err(Some(Error {
                    location: end,
                    message: format!("unterminated string"),
                }));
            }
            if c == '"' {
                terminated = true;
                break;
            }

            if c != '\\' {
                if let Some(content) = content.as_mut() {
                    content.push(c);
                }
                continue;
            }
            
            let Some(c) = cursor.next() else {
                break;
            };
            Tokenizer::adv(&mut end, c);

            let segment = tok.lex_for(Span { start: content_start, end })
                .expect("span is invalid");
            let str = content
                .get_or_insert_with(String::new);
            str.push_str(segment);

            fn radix_escape(
                count: usize, radix: u32, 
                cursor: &mut impl Iterator<Item=char>, location: &mut Location,
                src: &str,
            ) -> Result<char, Error> {
                let Some('{') = cursor.next() else {
                    return Err(Error {
                        location: *location,
                        message: format!("Expected '{{'"),
                    });
                };
                Tokenizer::adv(location, '{');

                let start = *location;
                for _ in 0..count {
                    if let Some(c) = cursor.next().filter(|c| c.is_digit(radix)) {
                        Tokenizer::adv(location, c);
                    } else {
                        return Err(Error {
                            location: *location,
                            message: format!("Expected {}-radix digit", radix),
                        });
                    }
                }
                let end = *location;
                if cursor.next() != Some('}') {
                    return Err(Error {
                        location: *location,
                        message: format!("Expected '}}'"),
                    });
                }
                Tokenizer::adv(location, '}');

                let content = &src[start.index..end.index];
                let value = u32::from_str_radix(content, radix).unwrap();
                Ok(char::from_u32(value).unwrap())
            }

            str.push(match c {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                'u' => radix_escape(4, 16, &mut cursor, &mut end, tok.source()).map_err(Some)?,
                'x' => radix_escape(2, 16, &mut cursor, &mut end, tok.source()).map_err(Some)?,
                _ => {
                    todo!("error")
                }
            });
            content_start = end;
        }
        if !terminated {
            return Err(Some(Error {
                location: end,
                message: format!("Expected {:?}", '"'),
            }));
        }

        let span = Span { start: content_start, end };
        let substring = tok.lex_for(span)
            .expect("source_for_span failed");
        let content = match content {
            Some(mut content) => {
                content.push_str(substring);
                Cow::Owned(content)
            }
            None => {
                Cow::Borrowed(substring)
            }
        };

        Ok(Token {
            span: Span { start, end },
            content,
        })
    }

    fn required<'a, 'src>(t: &'a Tokenizer<'src>, f: impl (FnOnce() -> String) + 'a) -> impl (FnOnce(Option<Error>) -> Option<Error>) + 'a {
        move |err| {
            Some(err
                .unwrap_or_else(|| {
                    Error {
                        location: t.location(),
                        message: f()
                    }
                }))
        }
    }

    #[inline]
    fn char_is_ident(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
}

#[cfg(test)]
mod tests {
    use crate::{Tokenizer, Error};
    use super::ast::*;

    fn parse<'a, T>(src: &'a str, f: impl Fn(&Tokenizer<'a>) -> Result<T, Option<Error>>) -> Option<T> {
        match f(&Tokenizer::new(src)) {
            Err(Some(e)) => {
                eprintln!("error at {}", e.location);
                eprintln!(" :: {}", e.message);
                panic!("error occurred!")
            }
            Err(None) => None,
            Ok(item) => Some(item),
        }
    }

    impl<'a> Type<'a> {
        fn assert_named(&self) -> &str {
            match self {
                Self::Name(n) => n.content(),
                _ => panic!("type is not named!"),
            }
        }

        fn assert_pointer(&self) -> &Type<'a> {
            match self {
                Self::Ptr(inner) => inner.as_ref(),
                _ => panic!("type is not a pointer!"),
            }
        }

        fn assert_func(&self) -> &FnType<'a> {
            match self {
                Self::Func(func) => func.as_ref(),
                _ => panic!("type is not a function!"),
            }
        }
    }

    #[test]
    fn test_types() {
        let ty = parse("int", parse_type).unwrap();
        assert_eq!(ty.assert_named(), "int");

        let ty = parse("int**", parse_type).unwrap();
        let ty = ty.assert_pointer();
        let ty = ty.assert_pointer();
        assert_eq!(ty.assert_named(), "int");

        let ty = parse("(int,char**)int", parse_type).unwrap();
        let func = ty.assert_func();
        assert_eq!(func.args.len(), 2);
        assert_eq!(func.args[0].assert_named(), "int");
        assert_eq!(func.args[1].assert_pointer().assert_pointer().assert_named(), "char");
        assert_eq!(func.ret.as_ref().unwrap().assert_named(), "int");


        let ty = parse("((int,void*)bool,void*)", parse_type).unwrap();
        let func = ty.assert_func();
        assert_eq!(func.args.len(), 2);
        let inner = func.args[0].assert_func();
        assert_eq!(inner.args.len(), 2);
        assert_eq!(inner.args[0].assert_named(), "int");
        assert_eq!(inner.args[1].assert_pointer().assert_named(), "void");
        assert_eq!(inner.ret.as_ref().unwrap().assert_named(), "bool");
        assert_eq!(func.args[1].assert_pointer().assert_named(), "void");
        assert!(func.ret.is_none())
    }

    #[test]
    fn test_strings() {
        let src = r#""""#;
        let content = parse(src, parse_expr_str).unwrap();
        assert_eq!(content.content(), "");

        let src = r#""Hello World!""#;
        let content = parse(src, parse_expr_str).unwrap();
        assert_eq!(content.content(), "Hello World!");
    }
}

