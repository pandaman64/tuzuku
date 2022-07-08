use std::fmt::Display;

#[derive(Debug, Clone)]
pub(crate) enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
}

impl Value {
    pub(crate) fn display(&self) -> impl Display + '_ {
        match self {
            Value::Nil => "<nil>".to_string(),
            Value::Boolean(b) => format!("<{}>", b),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
        }
    }
}
