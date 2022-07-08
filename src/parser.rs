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
        .map(|literal| &*arena.alloc(AstBody::String(literal)));

    let number = text::int(10)
        .padded()
        .map(|literal: String| &*arena.alloc(AstBody::Number(literal.parse().unwrap())));

    let primitive = simple_string_literal.or(number);

    let factor = primitive
        .then(just('*').or(just('/')).then(primitive).repeated())
        .foldl(|lhs, (op, rhs)| match op {
            '*' => &*arena.alloc(AstBody::Mul(lhs, rhs)),
            '/' => &*arena.alloc(AstBody::Div(lhs, rhs)),
            _ => unreachable!(),
        });

    let term = factor
        .then(just('+').or(just('-')).then(factor).repeated())
        .foldl(|lhs, (op, rhs)| match op {
            '+' => &*arena.alloc(AstBody::Add(lhs, rhs)),
            '-' => &*arena.alloc(AstBody::Sub(lhs, rhs)),
            _ => unreachable!(),
        });

    let print_stmt = keyword("print")
        .padded()
        .ignore_then(term.delimited_by(just('('), just(')')).padded())
        .padded()
        .map(|expr| &*arena.alloc(AstBody::Print(expr)));

    print_stmt.then_ignore(end())
}
