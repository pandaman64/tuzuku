use std::ops::Range;

use chumsky::{
    prelude::{end, filter, just, Simple},
    text::{self, keyword, TextParser},
    Parser,
};
use typed_arena::Arena;

use crate::ast::{Ast, AstBody};

#[derive(Debug)]
pub(crate) struct LineMapper {
    // `lines[l] = idx` means that line `l` starts at char index `idx`
    lines: Vec<usize>,
}

impl LineMapper {
    pub(crate) fn new(source: &str) -> Self {
        let mut lines = vec![0];

        for (i, _) in source.chars().enumerate().filter(|(_, c)| *c == '\n') {
            lines.push(1 + i);
        }

        Self { lines }
    }

    pub(crate) fn find(&self, idx: usize) -> usize {
        match self.lines.binary_search(&idx) {
            Ok(l) => l + 1,
            Err(l) => l,
        }
    }
}

#[cfg(test)]
mod test_line_mapper {
    use super::LineMapper;

    #[test]
    fn test_line_mapper() {
        let source = r#"abc
defg"#;
        let mapper = LineMapper::new(source);

        assert_eq!(mapper.find(0), 1);
        assert_eq!(mapper.find(1), 1);
        assert_eq!(mapper.find(2), 1);
        assert_eq!(mapper.find(3), 1);
        assert_eq!(mapper.find(4), 2);
        assert_eq!(mapper.find(5), 2);
        assert_eq!(mapper.find(6), 2);
        assert_eq!(mapper.find(7), 2);
        assert_eq!(mapper.find(8), 2);
    }
}

pub(crate) fn parser<'arena>(
    arena: &'arena Arena<AstBody<'arena>>,
) -> impl Parser<char, Ast<'arena>, Error = Simple<char>> {
    let simple_string_literal = just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .map_with_span(|literal, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::String(literal)),
            span: span.into(),
        })
        .padded();

    let number = text::int(10)
        .map_with_span(|literal: String, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::Number(literal.parse().unwrap())),
            span: span.into(),
        })
        .padded();

    let primitive = simple_string_literal.or(number);

    let factor = primitive
        .then(just('*').or(just('/')).then(primitive).repeated())
        .foldl(|lhs, (op, rhs)| match op {
            '*' => Ast {
                body: arena.alloc(AstBody::Mul(lhs, rhs)),
                span: lhs.merge_span(rhs),
            },
            '/' => Ast {
                body: arena.alloc(AstBody::Div(lhs, rhs)),
                span: lhs.merge_span(rhs),
            },
            _ => unreachable!(),
        });

    let term = factor
        .then(just('+').or(just('-')).then(factor).repeated())
        .foldl(|lhs, (op, rhs)| match op {
            '+' => Ast {
                body: arena.alloc(AstBody::Add(lhs, rhs)),
                span: lhs.merge_span(rhs),
            },
            '-' => Ast {
                body: arena.alloc(AstBody::Sub(lhs, rhs)),
                span: lhs.merge_span(rhs),
            },
            _ => unreachable!(),
        });

    let print_stmt = keyword("print")
        .padded()
        .ignore_then(term.delimited_by(just('('), just(')')).padded())
        .map_with_span(|expr, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::Print(expr)),
            span: span.into(),
        })
        .padded();

    print_stmt.then_ignore(end())
}
