use std::fmt::Display;

#[derive(Clone)]
pub(crate) enum Value {
    Nil,
    Boolean(bool),
    String(String),
}

impl Value {
    pub(crate) fn display(&self) -> impl Display + '_ {
        match self {
            Value::Nil => "nil".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::String(s) => format!("\"{}\"", s),
        }
    }
}
