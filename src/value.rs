use std::fmt::Display;

pub(crate) enum Value {
    String(String),
}

impl Value {
    pub(crate) fn display(&self) -> impl Display + '_ {
        match self {
            Value::String(s) => s,
        }
    }
}
