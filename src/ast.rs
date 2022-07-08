use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Span {
    start: usize,
    end: usize,
}

impl From<Range<usize>> for Span {
    fn from(span: Range<usize>) -> Self {
        Span {
            start: span.start,
            end: span.end,
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
        Span {
            start: self.span.start.min(other.span.start),
            end: self.span.end.max(other.span.end),
        }
    }
}

pub(crate) enum AstBody<'arena> {
    Number(f64),
    String(String),
    Add(Ast<'arena>, Ast<'arena>),
    Sub(Ast<'arena>, Ast<'arena>),
    Mul(Ast<'arena>, Ast<'arena>),
    Div(Ast<'arena>, Ast<'arena>),
    Print(Ast<'arena>),
}
