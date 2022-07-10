use std::{collections::HashSet, ops::Range};

use chumsky::{
    prelude::{end, filter, just, recursive, Simple},
    text::{ident, int, keyword, TextParser},
    Parser,
};
use once_cell::sync::Lazy;
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

fn allowed_ident() -> impl Parser<char, String, Error = Simple<char>> + Clone + Copy {
    static KEYWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
        let mut keywords = HashSet::new();
        keywords.insert("var");
        keywords.insert("fun");
        keywords.insert("print");
        keywords
    });

    ident().try_map(|ident: String, span| {
        if !KEYWORDS.contains(ident.as_str()) {
            Ok(ident)
        } else {
            Err(Simple::custom(
                span,
                format!("{} is a reserved keyword", ident),
            ))
        }
    })
}

#[allow(clippy::let_and_return)]
pub(crate) fn parser<'arena>(
    arena: &'arena Arena<AstBody<'arena>>,
) -> impl Parser<char, Ast<'arena>, Error = Simple<char>> {
    let expr = recursive(|expr| {
        let simple_string_literal = just('"')
            .ignore_then(filter(|c| *c != '"').repeated())
            .then_ignore(just('"'))
            .collect::<String>()
            .map_with_span(|literal, span: Range<usize>| Ast {
                body: arena.alloc(AstBody::String(literal)),
                span: span.into(),
            })
            .padded();

        let number = int(10)
            .map_with_span(|literal: String, span: Range<usize>| Ast {
                body: arena.alloc(AstBody::Number(literal.parse().unwrap())),
                span: span.into(),
            })
            .padded();

        let var = allowed_ident()
            .map_with_span(|ident, span: Range<usize>| Ast {
                body: arena.alloc(AstBody::Var(ident)),
                span: span.into(),
            })
            .padded();

        let primitive = simple_string_literal.or(number).or(var);

        let call = primitive
            .then(
                expr.separated_by(just(',').padded())
                    .allow_trailing()
                    .delimited_by(just('('), just(')'))
                    .padded()
                    .repeated(),
            )
            .foldl(|callee, arguments| Ast {
                span: arguments
                    .iter()
                    .fold(callee.span, |span, ast: &Ast<'_>| span.merge(ast.span)),
                body: arena.alloc(AstBody::Call { callee, arguments }),
            })
            .padded();

        let factor = call
            .clone()
            .then(just('*').or(just('/')).then(call).repeated())
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
            .clone()
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

        term
    });

    let stmt = recursive(move |stmt| {
        let print_stmt = keyword("print")
            .padded()
            .ignore_then(expr.clone().delimited_by(just('('), just(')')).padded())
            .then_ignore(just(';'))
            .map_with_span(|expr, span: Range<usize>| Ast {
                body: arena.alloc(AstBody::Print(expr)),
                span: span.into(),
            })
            .padded();

        let assign_stmt = allowed_ident()
            .padded()
            .then_ignore(just('=').padded())
            .then(expr.clone())
            .then_ignore(just(';'))
            .map_with_span(|(ident, expr), span: Range<usize>| Ast {
                body: arena.alloc(AstBody::Assign(ident, expr)),
                span: span.into(),
            })
            .padded();

        let expr_stmt = expr
            .clone()
            .then_ignore(just(';'))
            .map_with_span(|expr, span: Range<usize>| Ast {
                body: arena.alloc(AstBody::ExprStmt { expr }),
                span: span.into(),
            })
            .padded();

        let var_decl = keyword("var")
            .padded()
            .ignore_then(allowed_ident().padded())
            .then(just('=').ignore_then(expr).or_not())
            .then_ignore(just(';'))
            .map_with_span(|(ident, initializer), span: Range<usize>| Ast {
                body: arena.alloc(AstBody::VarDecl { ident, initializer }),
                span: span.into(),
            })
            .padded();

        let fun_decl = keyword("fun")
            .ignore_then(allowed_ident().padded())
            .then(
                allowed_ident()
                    .separated_by(just(',').padded())
                    .allow_trailing()
                    .delimited_by(just('('), just(')'))
                    .padded(),
            )
            .then(stmt.clone().repeated().delimited_by(just('{'), just('}')))
            .map_with_span(|((ident, parameters), body), span: Range<usize>| Ast {
                body: arena.alloc(AstBody::FunDecl {
                    ident,
                    parameters,
                    body,
                }),
                span: span.into(),
            })
            .padded();

        print_stmt
            .or(assign_stmt)
            .or(expr_stmt)
            .or(var_decl)
            .or(fun_decl)
    });

    let program = stmt
        .repeated()
        .map_with_span(|stmts, span: Range<usize>| Ast {
            body: arena.alloc(AstBody::Root(stmts)),
            span: span.into(),
        });

    program.then_ignore(end())
}
