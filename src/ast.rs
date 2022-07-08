use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Span {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl From<Range<usize>> for Span {
    fn from(span: Range<usize>) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

impl Span {
    pub(crate) fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Ast<'arena> {
    pub(crate) body: &'arena AstBody<'arena>,
    pub(crate) span: Span,
}

impl<'arena> Ast<'arena> {
    pub(crate) fn merge_span(self, other: Self) -> Span {
        self.span.merge(other.span)
    }
}

pub(crate) enum AstBody<'arena> {
    Root(Vec<Ast<'arena>>),
    Number(f64),
    String(String),
    Add(Ast<'arena>, Ast<'arena>),
    Sub(Ast<'arena>, Ast<'arena>),
    Mul(Ast<'arena>, Ast<'arena>),
    Div(Ast<'arena>, Ast<'arena>),
    Print(Ast<'arena>),
    Assign(String, Ast<'arena>),
    Var(String),
    FunDecl {
        ident: String,
        parameters: Vec<String>,
        body: Vec<Ast<'arena>>,
    },
    Call {
        callee: Ast<'arena>,
        arguments: Vec<Ast<'arena>>,
    },
    ExprStmt {
        expr: Ast<'arena>,
    },
}
