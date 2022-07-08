pub(crate) type Ast<'arena> = &'arena AstBody<'arena>;

pub(crate) enum AstBody<'arena> {
    Number(f64),
    String(String),
    Add(Ast<'arena>, Ast<'arena>),
    Sub(Ast<'arena>, Ast<'arena>),
    Mul(Ast<'arena>, Ast<'arena>),
    Div(Ast<'arena>, Ast<'arena>),
    Print(Ast<'arena>),
}
