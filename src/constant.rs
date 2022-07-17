use crate::value::Function;

#[derive(Clone)]
pub(crate) enum Constant {
    Number(f64),
    String(String),
    Function(Function),
}

impl Constant {
    pub(crate) fn display(&self) -> String {
        match self {
            Constant::Number(n) => n.to_string(),
            Constant::String(s) => s.clone(),
            Constant::Function(f) => format!("<function {}>", f.name()),
        }
    }
}
