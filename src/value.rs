use std::{fmt::Display, rc::Rc};

use crate::opcode::Chunk;

#[derive(Clone)]
pub(crate) enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function { name: String, chunk: Rc<Chunk> },
}

impl Value {
    pub(crate) fn display(&self) -> impl Display + '_ {
        match self {
            Value::Nil => "<nil>".to_string(),
            Value::Boolean(b) => format!("<{}>", b),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Function { name, .. } => format!("<function {}>", name),
        }
    }
}
