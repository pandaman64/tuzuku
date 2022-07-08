use std::ops::Range;

use chumsky::{
    prelude::{end, filter, just, Simple},
    text::{self, keyword, TextParser},
    Parser,
};
use typed_arena::Arena;

use crate::ast::{Ast, AstBody};

pub(crate) fn parser<'arena>(
    arena: &'arena Arena<AstBody<'arena>>,
) -> impl Parser<char, Ast<'arena>, Error = Simple<char>> {
    let simple_string_literal = just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .padded()
        .map_with_span(|literal, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::String(literal)),
            span: span.into(),
        });

    let number = text::int(10)
        .padded()
        .map_with_span(|literal: String, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::Number(literal.parse().unwrap())),
            span: span.into(),
        });

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
        .padded()
        .map_with_span(|expr, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::Print(expr)),
            span: span.into(),
        });

    print_stmt.then_ignore(end())
}
