use chumsky::{
    prelude::{end, filter, just, Simple},
    text::{keyword, TextParser},
    Parser,
};

use crate::ast::Ast;

pub(crate) fn parse() -> impl Parser<char, Ast, Error = Simple<char>> {
    let simple_string_literal = just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .padded();

    let print_stmt = keyword("print")
        .padded()
        .ignore_then(
            simple_string_literal
                .delimited_by(just('('), just(')'))
                .padded(),
        )
        .padded()
        .map(Ast::Print);

    print_stmt.then_ignore(end())
}
